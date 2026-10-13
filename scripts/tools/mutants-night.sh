#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

readonly TASK_MARKER='<!-- mutants-night-task -->'
readonly JQ_EXPECT_ARRAY='def expect_array($key; $what):
  if (.[$key] | type) != "array" then
    error("the \($what) response has no \($key): \(.message // "unexpected body")")
  else . end;'
readonly JQ_NAMED='def named: (.body // "") | gsub("\r"; "") | (capture("<!-- items\n(?<items>.*?)-->"; "p").items // "") | split("\n") | map(select(length > 0));'

die() {
  echo "mutants-night: $*" >&2
  exit 1
}

nights() {
  jq -r "$JQ_EXPECT_ARRAY"'
    expect_array("workflow_runs"; "runs")
    | [.workflow_runs[] | select(.event == "schedule" and .head_branch == "main" and .conclusion == "success")]
    | sort_by(.created_at) | reverse | .[].id
  ' "$1" || die "cannot read the nights from $1"
}

green_record() {
  jq -r --argjson night "$2" '
    select(.night == $night and .verdict == "green" and .complete == true and (.sha | test("^[0-9a-f]{40}$")))
    | .sha
  ' "$1" || die "cannot read the night record $1"
}

lines_json() {
  if [ -f "$1" ]; then
    jq -R -s 'split("\n") | map(select(length > 0)) | unique' "$1"
  else
    echo null
  fi
}

verdict() {
  local night=$1 attempt=$2 sha=$3 base=$4 base_record=$5 mode=$6 reason=$7 planned=$8 tested=$9 carried=${10} needs=${11}
  jq -n \
    --argjson night "$night" --argjson attempt "$attempt" --arg sha "$sha" --arg base "$base" --arg mode "$mode" \
    --arg reason "$reason" --argjson planned "$(lines_json "$planned")" \
    --argjson caught "$(lines_json "$tested/caught.txt")" --argjson unviable "$(lines_json "$tested/unviable.txt")" \
    --argjson missed "$(lines_json "$tested/missed.txt")" --argjson timeout "$(lines_json "$tested/timeout.txt")" \
    --argjson carried_missed "$(lines_json "$carried/missed.txt")" \
    --argjson carried_timeout "$(lines_json "$carried/timeout.txt")" \
    --slurpfile base_record "$base_record" --slurpfile needs "$needs" '
    def list: . // [];
    ($base_record[0].missed // []) as $base_missed
    | (($missed | list) + ($carried_missed | list) | unique) as $all_missed
    | (($timeout | list) + ($carried_timeout | list) | unique) as $all_timeout
    | (($caught | list) + ($unviable | list) + ($missed | list) + ($timeout | list)) as $outcomes
    | (if $planned == null then null else $planned - $outcomes end) as $untested
    | [$needs[0] | to_entries[] | select(.value.result != "success" and .value.result != "skipped")
        | {job: .key, result: .value.result}] as $failures
    | ($all_missed - $base_missed) as $new_missed
    | ($planned != null and ($untested | length) == 0) as $complete
    | {
        night: $night, attempt: $attempt, sha: $sha,
        base: (if $base == "" then null else $base end),
        base_night: ($base_record[0].night // null),
        mode: $mode, reason: $reason,
        planned: $planned,
        counts: {caught: ($caught | list | length), unviable: ($unviable | list | length),
                 missed: ($all_missed | length), timeout: ($all_timeout | length)},
        missed: $all_missed, new_missed: $new_missed, timeout: $all_timeout,
        untested: (($untested // []) | length),
        failures: $failures, complete: $complete,
        items: ($new_missed
          + [$failures[] | "failure: \(.job)"]
          + (if $complete or $planned == null then [] else ["incomplete: planned mutants without an outcome"] end)),
        verdict: (if ($failures | length) == 0 and $complete and ($new_missed | length) == 0 then "green" else "red" end)
      }
  ' || die "cannot decide the night"
}

route() {
  local record=$1 tasks=$2 all=$3
  jq -n --slurpfile record "$record" --slurpfile tasks "$tasks" --argjson all "$(lines_json "$all")" "$JQ_NAMED"'
    $record[0] as $r
    | ($r.items) as $shown_now
    | (($r.planned // []) - ($r.missed // [])) as $passed
    | def has_item($xs; $x): any($xs[]; . == $x);
      def gone($item):
        if has_item($shown_now; $item) then false
        elif ($item | startswith("failure: ")) or ($item | startswith("incomplete: ")) then true
        elif has_item($passed; $item) then true
        else $all != null and (has_item($all; $item) | not)
        end;
    [$tasks[0][] | {number, named: named}] as $open
    | {
        update: [$open[]
          | {number,
             shown: [.named[] | select(has_item($shown_now; .))],
             gone: [.named[] | select(gone(.))]}
          | select((.shown | length) > 0 or (.gone | length) > 0)],
        file: ($shown_now - [$open[].named[]])
      }
  ' || die "cannot route the night"
}

mentions() {
  jq -r '[.[].author | select(. != null and (endswith("[bot]") | not))] | unique | map("@" + .) | join(" ")' "$1"
}

reproduce() {
  local repo=$1 sha=$2 packages=$3 night=$4
  printf 'gh workflow run Mutants --repo %s --ref main -f sha=%s' "$repo" "$sha"
  [ -z "$packages" ] || printf " -f packages='%s'" "$packages"
  printf ' -f night=%s' "$night"
}

task_body() {
  local record=$1 merges=$2 items=$3 repo=$4 packages=$5 minutes=$6
  jq -r -n --slurpfile record "$record" --slurpfile merges "$merges" --slurpfile items "$items" \
    --arg repo "$repo" --arg marker "$TASK_MARKER" --arg mentions "$(mentions "$merges")" \
    --arg reproduce "$(reproduce "$repo" "$(jq -r .sha "$record")" "$packages" "$(jq -r .night "$record")")" \
    --arg minutes "$minutes" '
    $record[0] as $r | $items[0] as $items
    | $marker,
      "Mutants night [\($r.night)](https://github.com/\($repo)/actions/runs/\($r.night)) at `\($r.sha)` is red.",
      "",
      "**Range:** \(if $r.base then "`\($r.base)`..`\($r.sha)` (from green night \($r.base_night))" else "no green night yet, so the newest \($merges[0] | length) first-parent commits to `\($r.sha)`" end). **Scope:** \($r.mode) (\($r.reason)).",
      "",
      "**Survivors and failures:**",
      "",
      "```text",
      ($items[]),
      "```",
      "",
      "**Candidates:** the attribution is ambiguous, because no triage re-tests a night, so every merge in the range is a candidate.",
      "",
      ($merges[0][] | "- `\(.sha[0:12])` \(if .number then "#\(.number) " else "" end)\(.title // "")"),
      "",
      (if $mentions != "" then "Opened by \($mentions)." else empty end),
      "",
      "**Reproduce** (the plan predicts about \($minutes) job-minutes for these packages):",
      "",
      "```sh",
      $reproduce,
      "```",
      "",
      "<!-- items",
      ($items[]),
      "-->"
  ' || die "cannot write the task"
}

comment_body() {
  local record=$1 shown=$2 gone=$3 repo=$4
  jq -r -n --slurpfile record "$record" --argjson shown "$shown" --argjson gone "$gone" --arg repo "$repo" '
    $record[0] as $r
    | "Mutants night [\($r.night)](https://github.com/\($repo)/actions/runs/\($r.night)) at `\($r.sha)`:",
      (if ($shown | length) > 0 then "", "Still shows:", "", "```text", $shown[], "```" else empty end),
      (if ($gone | length) > 0 then "", "No longer shows:", "", "```text", $gone[], "```" else empty end)
  ' || die "cannot write the comment"
}

tasks() {
  jq --arg marker "$TASK_MARKER" '
    if type != "array" then error("the issues response is no array: \(.message // "unexpected body")") else . end
    | [.[] | select(.pull_request == null and ((.body // "") | contains($marker))) | {number, body}]
  ' "$1" || die "cannot read the open tasks from $1"
}

green_line() {
  jq -r '"Mutants night \(.night): \(.base // "start")..\(.sha) green"' "$1" || die "cannot read $1"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  command=${1:-}
  shift || :
  case "$command" in
    nights) nights "$@" ;;
    tasks) tasks "$@" ;;
    green-record) green_record "$@" ;;
    verdict) verdict "$@" ;;
    route) route "$@" ;;
    task-body) task_body "$@" ;;
    comment-body) comment_body "$@" ;;
    green-line) green_line "$@" ;;
    *) die "usage: mutants-night.sh nights|green-record|tasks|verdict|route|task-body|comment-body|green-line ..." ;;
  esac
fi
