#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

readonly FULL_RUN_TRIGGERS=(
  .cargo/mutants.toml
  .github/workflows/mutants.yml
  scripts/tools/mutants-plan.sh
  scripts/tools/mutants-night.sh
  flake.lock
)
readonly OUTCOMES=(caught unviable missed timeout)
readonly MAX_SHARDS=256
readonly MODEL_RUN=38447766912
readonly MODEL_OVERHEAD=471
readonly MODEL_MUTANT=19
readonly JQ_EXPECT_ARRAY='def expect_array($key; $what):
  if (.[$key] | type) != "array" then
    error("the \($what) response has no \($key): \(.message // "unexpected body")")
  else . end;'

die() {
  echo "mutants-plan: $*" >&2
  exit 1
}

guard() {
  local ref=$1 default=$2 sha=$3 night=$4
  refuse() {
    echo "::error title=$1::$2"
    die "$1: $2"
  }
  [ "$ref" = "refs/heads/$default" ] ||
    refuse mutants-off-main "${ref//[^0-9A-Za-z\/._-]/?} is not refs/heads/$default: Mutants runs only on $default"
  if [ -n "$sha" ]; then
    [[ $sha =~ ^[0-9a-f]{40}$ ]] ||
      refuse mutants-sha-invalid "the sha input '${sha//[^0-9A-Za-z]/?}' is not 40 lowercase hex digits"
    git merge-base --is-ancestor -- "$sha" "origin/$default" 2>/dev/null ||
      refuse mutants-sha-off-main "$sha is not a commit $default reaches"
  fi
  if [ -n "$night" ] && ! [[ $night =~ ^[0-9]+$ ]]; then
    refuse mutants-night-invalid "the night input '${night//[^0-9A-Za-z]/?}' is not a run id"
  fi
}

mode() {
  local event=$1 green=$2 head=$3
  is_ancestor() { git merge-base --is-ancestor "$1" "$head" 2>/dev/null; }
  emit() { printf 'mode=%s\nbase=%s\nreason=%s\n' "$1" "$2" "$3"; }

  case "$event" in
    workflow_dispatch)
      emit full "" "a dispatch tests every mutant of its packages"
      return
      ;;
    schedule) ;;
    *) die "unexpected event '$event'" ;;
  esac

  if [ -z "$green" ]; then
    emit full "" "no green night"
    return
  fi
  if ! is_ancestor "$green"; then
    emit full "" "green night $green is not an ancestor of $head"
    return
  fi
  local changed file
  changed=$(git diff --name-only "$green" "$head")
  for file in "${FULL_RUN_TRIGGERS[@]}"; do
    if grep -qxF -- "$file" <<<"$changed"; then
      emit full "" "$file changed since green night $green"
      return
    fi
  done
  emit in-diff "$green" "diff from green night $green"
}

packages() {
  local list_json=$1 requested=$2
  jq -r -n --slurpfile list "$list_json" --arg requested "$requested" '
    ($list[0] | map(.package) | unique) as $known
    | if $requested == "" then $known
      else ($requested | try fromjson catch error("packages is not JSON: \($requested)"))
        | if type != "array" or length == 0 then error("packages must be a non-empty JSON array of names")
          elif any(.[]; type != "string" or (. as $p | $known | index($p) | not))
          then error("packages names a package with no mutant; known: \($known | join(", "))")
          else unique end
      end
    | .[] | "--package=\(.)"
  ' || die "the packages input is refused"
}

resume_runs() {
  jq -r --argjson id "$2" --argjson attempt "$3" --arg sha "$4" "$JQ_EXPECT_ARRAY"'
    expect_array("workflow_runs"; "runs")
    | .workflow_runs[]
    | select(.head_branch == "main")
    | select(.display_title == "Mutants \($sha)"
        or (.display_title | startswith("Mutants \($sha) for "))
        or (.event == "push" and .head_sha == $sha))
    | select((.id != $id and .status == "completed") or (.id == $id and $attempt > 1))
    | .id
  ' "$1" || die "cannot read the runs to resume from $1"
}

timing_source() {
  jq -r "$JQ_EXPECT_ARRAY"'
    expect_array("workflow_runs"; "runs")
    | [.workflow_runs[] | select(.head_branch == "main" and .status == "completed")]
    | sort_by(.created_at) | reverse | .[].id
  ' "$1" || die "cannot read the timing source runs from $1"
}

artifacts() {
  jq -r "$JQ_EXPECT_ARRAY"'
    expect_array("artifacts"; "artifacts")
    | .artifacts[]
    | select((.expired | not) and (.name | test("^(mutants\\.out(-.*)?|mutants-plan-.*)$")))
    | "\(.id) \(.name)"
  ' "$1" || die "cannot read the artifacts from $1"
}

timings() {
  local jobs=$1 dir
  shift
  for dir in "$@"; do
    [ -f "$dir/outcomes.json" ] || continue
    jq -c --arg dir "${dir%/}" '{
      shard: ($dir | capture("-(?<k>[0-9]+)$").k | tonumber),
      outcomes: [.outcomes[] | select(.scenario != "Baseline")
        | {file: (.scenario.Mutant.name | split(":")[0]), seconds: ([.phase_results[].duration] | add // 0)}]
    }' "$dir/outcomes.json"
  done | jq -s --slurpfile jobs "$jobs" --argjson model_overhead "$MODEL_OVERHEAD" \
    --argjson model_mutant "$MODEL_MUTANT" --arg model_run "$MODEL_RUN" "$JQ_EXPECT_ARRAY"'
    ($jobs[0] | expect_array("jobs"; "jobs") | [.jobs[]
      | select(.conclusion == "success" and (.name | test("shard [0-9]+$")))
      | {shard: (.name | capture("shard (?<k>[0-9]+)$").k | tonumber),
         wall: ((.completed_at | fromdate) - (.started_at | fromdate))}]) as $walls
    | [.[].outcomes[]] as $all
    | if ($all | length) == 0 then
        {source: "model from run \($model_run)", overhead: $model_overhead, mean: $model_mutant, per_file: {}}
      else
        {source: "measured",
         overhead: ([.[] as $s | $walls[] | select(.shard == $s.shard)
           | .wall - ([$s.outcomes[].seconds] | add // 0)] | max // $model_overhead | ceil),
         mean: ([$all[].seconds] | add / length),
         per_file: ($all | group_by(.file)
           | map({key: .[0].file, value: ([.[].seconds] | add / length)}) | from_entries)}
      end
  ' || die "cannot read the timings"
}

shards() {
  local timings=$1 list=$2 budget=$3
  jq -r -n --slurpfile t "$timings" --rawfile list "$list" --argjson budget "$budget" \
    --argjson most "$MAX_SHARDS" '
    $t[0] as $t
    | [$list | split("\n")[] | select(length > 0) | split(":")[0] | ($t.per_file[.] // $t.mean)] as $est
    | ($est | length) as $count
    | ([$count, $most] | min) as $limit
    | if $count == 0 then {n: 0, loads: [0]}
      else first(range(1; $limit + 1) as $n
        | [range(0; $n) as $k | [$est | to_entries[] | select(.key % $n == $k) | .value] | add] as $loads
        | select(($loads | max) + $t.overhead <= $budget or $n == $limit)
        | {n: $n, loads: $loads})
      end
    | (if .n == 0 then 0 else (.loads | max) + $t.overhead end) as $predicted
    | "n=\(.n)",
      "shards=[\([range(0; .n)] | map(tostring) | join(","))]",
      "predicted=\($predicted | ceil)",
      "over=\(.n > 0 and $predicted > $budget)",
      "minutes=\(if .n == 0 then 0 else ((.loads | add) + .n * $t.overhead) / 60 | ceil end)",
      "source=\($t.source)"
  ' || die "cannot size the shards"
}

union_of() {
  local list=$1 dir
  shift
  for dir in "$@"; do
    [ -n "$dir" ] || continue
    cat "$dir/$list.txt" 2>/dev/null || :
  done | sort -u
}

merge() {
  local dest=$1 outcome
  shift
  mkdir -p "$dest"
  for outcome in "${OUTCOMES[@]}"; do
    union_of "$outcome" "$@" > "$dest/$outcome.txt"
  done
  for outcome in "${OUTCOMES[@]}" previously_caught; do
    union_of "$outcome" "$@"
  done | sort -u > "$dest/previously_caught.txt"
}

summary() {
  local carried=$1 outcome tested
  shift
  tested=$(mktemp -d)
  merge "$tested" "$@"
  echo "| Outcome | Tested in this run | Carried from earlier runs at this commit |"
  echo "| --- | --- | --- |"
  for outcome in "${OUTCOMES[@]}"; do
    echo "| $outcome | $(wc -l < "$tested/$outcome.txt") | $(union_of "$outcome" "$carried" | wc -l) |"
  done
  for outcome in missed timeout; do
    union_of "$outcome" "$tested" "$carried" > "$tested/all-$outcome.txt"
    if [ -s "$tested/all-$outcome.txt" ]; then
      printf '\n### %s\n\n```text\n' "${outcome^}"
      cat "$tested/all-$outcome.txt"
      echo '```'
    fi
  done
  rm -rf "$tested"
}

exit_ok() {
  case "$1" in
    0 | 2 | 3) return 0 ;;
    *) return 1 ;;
  esac
}

job_report() {
  local job=$1 start=$2 now=$3 codes=$4 reproduce=$5 steps=$6
  printf '**%s wall time:** %ss\n' "$job" "$((now - start))"
  jq -r --arg job "$job" --arg codes "$codes" --arg reproduce "$reproduce" '
    ($codes | split(" ") | map(select(length > 0) | split("=") | {key: .[0], value: .[1]}) | from_entries) as $code
    | [to_entries[] | select(.value.outcome == "failure" or .value.outcome == "cancelled")] | first
    | if . == null then empty
      else "",
        "### \($job) failed: `\(if .value.outcome == "cancelled" then "mutants-timeout" else ($code[.key] // "unclassified") end)`",
        "",
        "- **Step:** `\(.key)` (\(.value.outcome))",
        "- **Reproduce:** `\($reproduce)`"
      end
  ' "$steps" || die "cannot read the steps of $job"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  command=${1:-}
  shift || :
  case "$command" in
    guard) guard "$@" ;;
    mode) mode "$@" ;;
    packages) packages "$@" ;;
    resume-runs) resume_runs "$@" ;;
    timing-source) timing_source "$@" ;;
    artifacts) artifacts "$@" ;;
    timings) timings "$@" ;;
    shards) shards "$@" ;;
    merge) merge "$@" ;;
    summary) summary "$@" ;;
    exit-ok) exit_ok "$@" ;;
    job-report) job_report "$@" ;;
    *) die "usage: mutants-plan.sh guard|mode|packages|resume-runs|timing-source|artifacts|timings|shards|merge|summary|exit-ok|job-report ..." ;;
  esac
fi
