#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

readonly SHARD_CAPACITY=9
readonly FULL_RUN_TRIGGERS=(.cargo/mutants.toml .github/workflows/mutants.yml)
readonly OUTCOMES=(caught unviable missed timeout)
readonly JQ_EXPECT_ARRAY='def expect_array($key; $what):
  if (.[$key] | type) != "array" then
    error("the \($what) response has no \($key): \(.message // "unexpected body")")
  else . end;'

die() {
  echo "mutants-plan: $*" >&2
  exit 1
}

green_sha() {
  jq -r "$JQ_EXPECT_ARRAY"'
    expect_array("workflow_runs"; "runs")
    | [.workflow_runs[] | select(.head_branch == "main" and .event == "push" and .conclusion == "success")]
    | max_by(.created_at)
    | if . == null then empty
      elif (.head_sha | type) == "string" and (.head_sha | test("^[0-9a-f]{40}$")) then .head_sha
      else error("run \(.id) has no head_sha") end
  ' "$1" || die "cannot read the last green main run from $1"
}

mode() {
  local event=$1 input_mode=$2 input_base=$3 green=$4 head=$5 base
  is_ancestor() { git merge-base --is-ancestor "$1" "$head" 2>/dev/null; }
  emit() { printf 'mode=%s\nbase=%s\nreason=%s\n' "$1" "$2" "$3"; }

  case "$event" in
    workflow_dispatch)
      case "$input_mode" in
        full)
          emit full "" "dispatched full run"
          return
          ;;
        in-diff) ;;
        *) die "unknown mode '$input_mode': expected full or in-diff" ;;
      esac
      if [ -n "$input_base" ]; then
        base=$(git rev-parse --verify --quiet "$input_base^{commit}") ||
          die "base '$input_base' is not a commit in this repository"
        is_ancestor "$base" || die "base $base is not an ancestor of $head"
        emit in-diff "$base" "dispatched in-diff run from the given base"
        return
      fi
      ;;
    push) ;;
    *) die "unexpected event '$event'" ;;
  esac

  if [ -z "$green" ]; then
    emit full "" "no green main run"
    return
  fi
  if ! is_ancestor "$green"; then
    emit full "" "green main run $green is not an ancestor of $head"
    return
  fi
  if [ "$event" = push ]; then
    local changed
    changed=$(git diff --name-only "$green" "$head")
    for file in "${FULL_RUN_TRIGGERS[@]}"; do
      if grep -qxF -- "$file" <<<"$changed"; then
        emit full "" "$file changed since green main run $green"
        return
      fi
    done
  fi
  emit in-diff "$green" "diff from green main run $green"
}

shards() {
  local total=$1 count=$2 n
  if [ "$total" -eq 0 ]; then
    die "examine_globs in .cargo/mutants.toml selects no mutant"
  fi
  n=$(((count + SHARD_CAPACITY - 1) / SHARD_CAPACITY))
  printf 'capacity=%s\nn=%s\nshards=[%s]\n' "$SHARD_CAPACITY" "$n" "$( ((n > 0)) && seq -s, 0 $((n - 1)))"
}

prior_runs() {
  jq -r --argjson id "$2" --argjson attempt "$3" --arg branch "$4" --arg sha "$5" "$JQ_EXPECT_ARRAY"'
    expect_array("workflow_runs"; "runs")
    | .workflow_runs[]
    | select(.head_branch == $branch and .head_sha == $sha)
    | select((.id != $id and .status == "completed") or (.id == $id and $attempt > 1))
    | .id
  ' "$1" || die "cannot read the prior runs from $1"
}

artifacts() {
  jq -r "$JQ_EXPECT_ARRAY"'
    expect_array("artifacts"; "artifacts")
    | .artifacts[]
    | select((.expired | not) and (.name | test("^(mutants\\.out(-.*)?|mutants-plan-.*)$")))
    | "\(.id) \(.name)"
  ' "$1" || die "cannot read the artifacts from $1"
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

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  command=${1:-}
  shift || :
  case "$command" in
    green-sha) green_sha "$@" ;;
    mode) mode "$@" ;;
    shards) shards "$@" ;;
    prior-runs) prior_runs "$@" ;;
    artifacts) artifacts "$@" ;;
    merge) merge "$@" ;;
    summary) summary "$@" ;;
    exit-ok) exit_ok "$@" ;;
    *) die "usage: mutants-plan.sh green-sha|mode|shards|prior-runs|artifacts|merge|summary|exit-ok ..." ;;
  esac
fi
