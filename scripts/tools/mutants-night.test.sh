#!/usr/bin/env bash
set -uo pipefail
export LC_ALL=C

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
night=("bash" "$here/mutants-night.sh")
fixtures=$here/mutants-night-fixtures
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0

pass() { echo "ok - $1"; }
fail() {
  echo "not ok - $1" >&2
  printf '%s\n' "${@:2}" >&2
  failures=$((failures + 1))
}

expect() {
  local name=$1 expected=$2 actual
  shift 2
  if ! actual=$("$@" 2>"$work/stderr"); then
    fail "$name" "exited nonzero:" "$(cat "$work/stderr")"
  elif [ "$actual" != "$expected" ]; then
    fail "$name" "expected:" "$expected" "actual:" "$actual"
  else
    pass "$name"
  fi
}

lines() { printf '%s\n' "$@"; }
compact() { "$@" | jq -c "$projection"; }

tip=7777777777777777777777777777777777777777
expect "nights: successful schedule runs on main, newest first" "$(lines 52 51)" \
  "${night[@]}" nights "$fixtures/runs-nights.json"
expect "green record: the night's own green, complete record gives its commit" "$tip" \
  "${night[@]}" green-record "$fixtures/record-green.json" 52
expect "green record: a record of another night counts for nothing" "" \
  "${night[@]}" green-record "$fixtures/record-green.json" 51
expect "green record: a run that succeeded with a red record is no green night" "" \
  "${night[@]}" green-record "$fixtures/record-red.json" 52

old='src/a.rs:1:1: old survivor'
m1='src/a.rs:2:1: replace f -> bool with true'
m2='src/a.rs:3:1: replace g with ()'
m3='src/b.rs:4:1: replace h -> u32 with 0'
dir() {
  local path=$work/$1 file
  shift
  mkdir -p "$path"
  while [ $# -gt 0 ]; do
    file=$1
    shift
    lines "$1" >> "$path/$file.txt"
    shift
  done
  echo "$path"
}
lines "$m1" "$m2" "$m3" > "$work/planned-three.txt"
lines "$m1" "$m3" > "$work/planned-two.txt"
: > "$work/planned-none.txt"
red_tested=$(dir red-tested caught "$m1" missed "$m2" timeout "$m3")
red_carried=$(dir red-carried missed "$old")
green_tested=$(dir green-tested caught "$m1" timeout "$m3")
part_tested=$(dir part-tested caught "$m1")
empty=$(dir empty)
decide() {
  "${night[@]}" verdict 99 1 "$tip" "${base:-}" "${base_record:-/dev/null}" in-diff "test reason" "$@"
}

projection='{verdict, complete, new_missed, items, failures, counts}'
base=$tip base_record=$fixtures/record-green.json expect \
  "verdict: a survivor the green night lacked is new and the night is red; a carried old one is not new" \
  "$(jq -cn --arg m2 "$m2" '{verdict: "red", complete: true, new_missed: [$m2], items: [$m2], failures: [],
    counts: {caught: 1, unviable: 0, missed: 2, timeout: 1}}')" \
  compact decide "$work/planned-three.txt" "$red_tested" "$red_carried" "$fixtures/needs-green.json"
expect "verdict: a complete night with timeouts and no survivor is green" \
  '{"verdict":"green","complete":true,"new_missed":[],"items":[],"failures":[],"counts":{"caught":1,"unviable":0,"missed":0,"timeout":1}}' \
  compact decide "$work/planned-two.txt" "$green_tested" "$empty" "$fixtures/needs-green.json"
expect "verdict: a planned mutant without an outcome makes the night red" \
  '{"verdict":"red","complete":false,"new_missed":[],"items":["incomplete: planned mutants without an outcome"],"failures":[],"counts":{"caught":1,"unviable":0,"missed":0,"timeout":0}}' \
  compact decide "$work/planned-two.txt" "$part_tested" "$empty" "$fixtures/needs-green.json"
expect "verdict: a failed plan is a red night naming the plan" \
  '{"verdict":"red","complete":false,"new_missed":[],"items":["failure: plan"],"failures":[{"job":"plan","result":"failure"}],"counts":{"caught":0,"unviable":0,"missed":0,"timeout":0}}' \
  compact decide "$work/absent.txt" "$empty" "$empty" "$fixtures/needs-plan-failed.json"
expect "verdict: a night with nothing to test is green" \
  '{"verdict":"green","complete":true,"new_missed":[],"items":[],"failures":[],"counts":{"caught":0,"unviable":0,"missed":0,"timeout":0}}' \
  compact decide "$work/planned-none.txt" "$empty" "$empty" "$fixtures/needs-nothing-planned.json"

shown='src/a.rs:3:1: replace x -> bool with true'
fresh='src/c.rs:1:1: brand new'
passed='src/b.rs:9:9: other'
jq -n --arg shown "$shown" --arg fresh "$fresh" --arg passed "$passed" '{
  items: [$shown, $fresh, "failure: shard"],
  planned: [$shown, $passed, $fresh], missed: [$shown, $fresh]}' > "$work/record-red.json"
lines "$shown" 'src/a.rs:5:1: untested mutant' "$passed" "$fresh" > "$work/all.txt"
expect "route: open tasks get what they still show and what is gone; anything no task names is filed" \
  "$(jq -cn --arg shown "$shown" --arg fresh "$fresh" --arg passed "$passed" '{
    update: [{number: 7, shown: [$shown], gone: ["src/a.rs:4:1: gone mutant", "failure: plan"]},
             {number: 8, shown: [], gone: [$passed]}],
    file: [$fresh, "failure: shard"]}')" \
  sh -c '"$@" | jq -c .' - "${night[@]}" route "$work/record-red.json" "$fixtures/tasks.json" "$work/all.txt"

echo '{"items": ["failure: plan"], "planned": null, "missed": []}' > "$work/record-plan-failed.json"
expect "route: with no listing, a named mutant is never called gone, and a named failure files nothing new" \
  '{"update":[{"number":7,"shown":["failure: plan"],"gone":[]}],"file":[]}' \
  sh -c '"$@" | jq -c .' - "${night[@]}" route "$work/record-plan-failed.json" "$fixtures/tasks.json" "$work/absent.txt"

jq -n '[{number: 7, body: "<!-- mutants-night-task -->\nx"}, {number: 9, body: "no marker"},
  {number: 10, body: null}, {number: 11, body: "<!-- mutants-night-task -->", pull_request: {}}]' > "$work/issues.json"
expect "tasks: only issues carrying the task marker, never a pull request" \
  '[{"number":7,"body":"<!-- mutants-night-task -->\nx"}]' \
  sh -c '"$@" | jq -c .' - "${night[@]}" tasks "$work/issues.json"
echo '{"message": "Bad credentials"}' > "$work/issues-error.json"
if "${night[@]}" tasks "$work/issues-error.json" > /dev/null 2>"$work/stderr"; then
  fail "tasks: an API error body is a failure, not an empty task list" "exited 0"
elif grep -qF "Bad credentials" "$work/stderr"; then
  pass "tasks: an API error body is a failure, not an empty task list"
else
  fail "tasks: an API error body is a failure, not an empty task list" "stderr lacks the message:" "$(cat "$work/stderr")"
fi

jq -n --arg tip "$tip" --arg shown "$shown" '{night: 99, sha: $tip, base: null, mode: "full", reason: "no green night",
  items: [$shown, "failure: shard"], planned: [$shown], missed: [$shown]}' > "$work/record-filed.json"
echo '[{"sha": "7777777777777777777777777777777777777777", "number": 3, "author": "dev", "title": "t"}]' > "$work/merges.json"
jq -n --arg shown "$shown" '[$shown, "failure: shard"]' > "$work/items.json"
"${night[@]}" task-body "$work/record-filed.json" "$work/merges.json" "$work/items.json" o/r '' 5 > "$work/task.md"
jq -n --rawfile body "$work/task.md" '[{number: 12, body: $body}]' > "$work/issues-filed.json"
"${night[@]}" tasks "$work/issues-filed.json" > "$work/tasks-filed.json"
expect "task body: a filed task is found again and the next night routes its items to it" \
  "$(jq -cn --arg shown "$shown" '{update: [{number: 12, shown: [$shown, "failure: shard"], gone: []}], file: []}')" \
  sh -c '"$@" | jq -c .' - "${night[@]}" route "$work/record-filed.json" "$work/tasks-filed.json" "$work/absent.txt"

base=$tip base_record=$fixtures/record-green.json decide "$work/planned-three.txt" "$red_tested" "$red_carried" \
  "$fixtures/needs-green.json" > "$work/record-decided.json"
lines "$m1" "$m2" "$m3" > "$work/all-three.txt"
echo '[]' > "$work/tasks-none.json"
expect "route: a verdict's own record files its new survivor" \
  "$(jq -cn --arg m2 "$m2" '{update: [], file: [$m2]}')" \
  sh -c '"$@" | jq -c .' - "${night[@]}" route "$work/record-decided.json" "$work/tasks-none.json" "$work/all-three.txt"

if [ "$failures" -ne 0 ]; then
  echo "$failures failed" >&2
  exit 1
fi
echo "all passed"
