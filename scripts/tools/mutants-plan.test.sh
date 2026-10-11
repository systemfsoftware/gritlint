#!/usr/bin/env bash
set -uo pipefail
export LC_ALL=C

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
plan=("bash" "$here/mutants-plan.sh")
fixtures=$here/mutants-plan-fixtures
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

expect_refusal() {
  local name=$1 message=$2
  shift 2
  if "$@" >"$work/stdout" 2>"$work/stderr"; then
    fail "$name" "exited 0 with:" "$(cat "$work/stdout")"
  elif [ -n "$message" ] && ! grep -qF -- "$message" "$work/stderr"; then
    fail "$name" "stderr lacks '$message':" "$(cat "$work/stderr")"
  else
    pass "$name"
  fi
}

expect "green: newest successful push run on main, past a newer failure, a dispatch and another branch" \
  2222222222222222222222222222222222222222 "${plan[@]}" green-sha "$fixtures/runs-green.json"
expect "green: nothing when no main run succeeded" "" "${plan[@]}" green-sha "$fixtures/runs-none-green.json"
expect "green: nothing when the workflow never ran" "" "${plan[@]}" green-sha "$fixtures/runs-empty.json"
expect_refusal "green: an API error body is a failure, not 'no green run'" "Bad credentials" \
  "${plan[@]}" green-sha "$fixtures/runs-error.json"
expect_refusal "green: a run without head_sha is a failure" "run 106 has no head_sha" \
  "${plan[@]}" green-sha "$fixtures/runs-no-sha.json"

repo=$work/repo
git init -q -b main "$repo"
g() { git -C "$repo" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
commit() {
  local message=$1 file
  shift
  for file in "$@"; do
    mkdir -p "$repo/$(dirname "$file")"
    echo "$message" >> "$repo/$file"
  done
  g add -A && g commit -q -m "$message" && g rev-parse HEAD
}
A=$(commit A src/lib.rs tests/t.rs Cargo.lock .cargo/mutants.toml .github/workflows/mutants.yml)
B=$(commit B src/lib.rs)
g switch -q -c side "$A"
D=$(commit D src/lib.rs)
head_after() {
  g switch -q --detach "$B"
  commit "$@"
}
H_rs=$(head_after rs src/lib.rs)
H_toml=$(head_after toml .cargo/mutants.toml)
H_workflow=$(head_after workflow .github/workflows/mutants.yml)
H_tests=$(head_after tests tests/t.rs Cargo.lock)
H_nested=$(head_after nested other/.cargo/mutants.toml)
mode() { (cd "$repo" && "${plan[@]}" mode "$@"); }
lines() { printf '%s\n' "$@"; }

expect "mode: a push diffs from the green run" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green main run $A")" mode push "" "" "$A" "$H_rs"
expect "mode: a push with no green run runs full" \
  "$(lines mode=full base= "reason=no green main run")" mode push "" "" "" "$H_rs"
expect "mode: a push whose green run is not an ancestor runs full" \
  "$(lines mode=full base= "reason=green main run $D is not an ancestor of $H_rs")" mode push "" "" "$D" "$H_rs"
expect "mode: a push that changes .cargo/mutants.toml runs full" \
  "$(lines mode=full base= "reason=.cargo/mutants.toml changed since green main run $A")" mode push "" "" "$A" "$H_toml"
expect "mode: a push that changes the mutants workflow runs full" \
  "$(lines mode=full base= "reason=.github/workflows/mutants.yml changed since green main run $A")" \
  mode push "" "" "$A" "$H_workflow"
expect "mode: a push that changes only tests and Cargo.lock stays in-diff" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green main run $A")" mode push "" "" "$A" "$H_tests"
expect "mode: only the root .cargo/mutants.toml forces a full run" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green main run $A")" mode push "" "" "$A" "$H_nested"
expect "mode: a dispatched full run is full" \
  "$(lines mode=full base= "reason=dispatched full run")" mode workflow_dispatch full "" "$A" "$H_rs"
expect "mode: a dispatched in-diff base wins over a config change and the green run" \
  "$(lines mode=in-diff "base=$A" "reason=dispatched in-diff run from the given base")" \
  mode workflow_dispatch in-diff "${A:0:7}" "$B" "$H_workflow"
expect "mode: a dispatched in-diff run without base diffs from the green run" \
  "$(lines mode=in-diff "base=$B" "reason=diff from green main run $B")" mode workflow_dispatch in-diff "" "$B" "$H_toml"
expect "mode: a dispatched in-diff run without base or green run is full" \
  "$(lines mode=full base= "reason=no green main run")" mode workflow_dispatch in-diff "" "" "$H_rs"
expect_refusal "mode: a dispatched base that is not an ancestor is refused" "is not an ancestor of $H_rs" \
  mode workflow_dispatch in-diff "$D" "$A" "$H_rs"
expect_refusal "mode: a dispatched base that is no commit is refused" "is not a commit" \
  mode workflow_dispatch in-diff 0123456789abcdef "$A" "$H_rs"
expect_refusal "mode: an unknown dispatch mode is refused" "unknown mode 'incremental'" \
  mode workflow_dispatch incremental "" "$A" "$H_rs"

expect "shards: one mutant, one shard" "$(lines capacity=9 n=1 'shards=[0]')" "${plan[@]}" shards 10 1
expect "shards: a full shard is still one" "$(lines capacity=9 n=1 'shards=[0]')" "${plan[@]}" shards 10 9
expect "shards: one past a shard is two" "$(lines capacity=9 n=2 'shards=[0,1]')" "${plan[@]}" shards 10 10
expect "shards: the full list at fe886c5" "$(lines capacity=9 n=53 "shards=[$(seq -s, 0 52)]")" \
  "${plan[@]}" shards 475 475
expect "shards: nothing left to test starts no shard" "$(lines capacity=9 n=0 'shards=[]')" \
  "${plan[@]}" shards 475 0
expect_refusal "shards: a listing with no mutant fails, in-diff or full" "selects no mutant" "${plan[@]}" shards 0 0

expect "prior runs: finished runs on this branch at this commit" 10 \
  "${plan[@]}" prior-runs "$fixtures/runs-at-sha.json" 20 1 main 6666666666666666666666666666666666666666
expect "prior runs: a re-run attempt also resumes its own run" "$(lines 20 10)" \
  "${plan[@]}" prior-runs "$fixtures/runs-at-sha.json" 20 2 main 6666666666666666666666666666666666666666
expect_refusal "prior runs: an API error body is a failure" "Bad credentials" \
  "${plan[@]}" prior-runs "$fixtures/runs-error.json" 20 1 main 6666666666666666666666666666666666666666
expect "artifacts: unexpired plans and mutants.out, including the unsharded name" \
  "$(lines '1 mutants-plan-1' '2 mutants.out-1-0' '3 mutants.out')" "${plan[@]}" artifacts "$fixtures/artifacts.json"
expect_refusal "artifacts: an API error body is a failure" "Not Found" \
  "${plan[@]}" artifacts "$fixtures/artifacts-error.json"

m1='src/a.rs:1:1: replace f -> bool with true'
m2='src/a.rs:2:5: replace && with || in g'
m3='src/b.rs:3:9: replace h -> Vec<String> with vec![]'
m4='src/b.rs:4:1: replace * with / in k'
m5='src/c.rs:5:1: replace l with ()'
mkdir -p "$work/r1" "$work/r2" "$work/r3"
lines "$m1" "$m2" > "$work/r1/caught.txt"
lines "$m3" > "$work/r1/missed.txt"
lines "$m2" > "$work/r2/caught.txt"
lines "$m4" > "$work/r2/unviable.txt"
lines "$m5" > "$work/r2/timeout.txt"
lines "$m1" > "$work/r3/previously_caught.txt"
"${plan[@]}" merge "$work/merged" "$work/r1" "$work/r2" "$work/r3" "$work/absent"
expect "merge: --iterate skips every name any run tested, once each" "$(lines "$m1" "$m2" "$m3" "$m4" "$m5")" \
  cat "$work/merged/previously_caught.txt"
expect "merge: caught is the union" "$(lines "$m1" "$m2")" cat "$work/merged/caught.txt"
expect "merge: missed carries forward" "$m3" cat "$work/merged/missed.txt"
expect "merge: timeouts carry forward" "$m5" cat "$work/merged/timeout.txt"
"${plan[@]}" merge "$work/first"
expect "merge: the first run at a commit seeds empty lists" "" cat "$work/first/previously_caught.txt" "$work/first/caught.txt"

mkdir -p "$work/s1" "$work/s2" "$work/carried"
lines "$m1" "$m2" > "$work/s1/caught.txt"
lines "$m2" > "$work/s2/caught.txt"
lines "$m4" > "$work/s2/missed.txt"
lines "$m3" > "$work/carried/missed.txt"
expect "summary: a name two shard attempts caught counts once; carried lists are named" \
  "$(lines '| Outcome | Tested in this run | Carried from earlier runs at this commit |' \
    '| --- | --- | --- |' '| caught | 2 | 0 |' '| unviable | 0 | 0 |' '| missed | 1 | 1 |' '| timeout | 0 | 0 |' \
    '' '### Missed' '' '```text' "$m3" "$m4" '```')" \
  "${plan[@]}" summary "$work/carried" "$work/s1" "$work/s2"
expect "summary: a shard's own table carries nothing" \
  "$(lines '| Outcome | Tested in this run | Carried from earlier runs at this commit |' \
    '| --- | --- | --- |' '| caught | 1 | 0 |' '| unviable | 0 | 0 |' '| missed | 1 | 0 |' '| timeout | 0 | 0 |' \
    '' '### Missed' '' '```text' "$m4" '```')" \
  "${plan[@]}" summary "" "$work/s2"

for status in 0 2 3; do
  expect "exit-ok: $status leaves the run green" "" "${plan[@]}" exit-ok "$status"
done
for status in 1 4 70; do
  expect_refusal "exit-ok: $status fails the run" "" "${plan[@]}" exit-ok "$status"
done

if [ "$failures" -ne 0 ]; then
  echo "$failures failed" >&2
  exit 1
fi
echo "all passed"
