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
A=$(commit A src/lib.rs tests/t.rs Cargo.lock .cargo/mutants.toml .github/workflows/mutants.yml flake.lock \
  scripts/tools/mutants-plan.sh scripts/tools/mutants-night.sh)
B=$(commit B src/lib.rs)
g update-ref refs/remotes/origin/main "$B"
g switch -q -c side "$A"
D=$(commit D src/lib.rs)
head_after() {
  g switch -q --detach "$B"
  commit "$@"
}
H_rs=$(head_after rs src/lib.rs)
H_tests=$(head_after tests tests/t.rs Cargo.lock)
H_nested=$(head_after nested other/.cargo/mutants.toml)
in_repo() { (cd "$repo" && "${plan[@]}" "$@"); }
lines() { printf '%s\n' "$@"; }

expect "guard: main's tip on main passes" "" in_repo guard refs/heads/main main "" ""
expect "guard: a commit main reaches passes, with a night run id" "" in_repo guard refs/heads/main main "$A" 38447766912
expect_refusal "guard: a dispatch on a branch is refused" "mutants-off-main" in_repo guard refs/heads/side main "" ""
expect_refusal "guard: a pull request ref is refused" "mutants-off-main" in_repo guard refs/pull/7/merge main "" ""
expect_refusal "guard: a commit only a branch reaches is refused" "mutants-sha-off-main" \
  in_repo guard refs/heads/main main "$D" ""
expect_refusal "guard: a 40-hex sha that is no commit is refused" "mutants-sha-off-main" \
  in_repo guard refs/heads/main main 0123456789abcdef0123456789abcdef01234567 ""
expect_refusal "guard: an abbreviated sha is refused" "mutants-sha-invalid" in_repo guard refs/heads/main main "${A:0:12}" ""
expect_refusal "guard: an option-shaped sha is refused" "mutants-sha-invalid" in_repo guard refs/heads/main main --all ""
expect_refusal "guard: a night that is not a run id is refused" "mutants-night-invalid" \
  in_repo guard refs/heads/main main "" "12a"

expect "mode: a night diffs from the green night" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green night $A")" in_repo mode schedule "$A" "$H_rs"
expect "mode: a night with no green night runs everything" \
  "$(lines mode=full base= "reason=no green night")" in_repo mode schedule "" "$H_rs"
expect "mode: a night whose green night is not an ancestor runs everything" \
  "$(lines mode=full base= "reason=green night $D is not an ancestor of $H_rs")" in_repo mode schedule "$D" "$H_rs"
for input in .cargo/mutants.toml .github/workflows/mutants.yml scripts/tools/mutants-plan.sh \
  scripts/tools/mutants-night.sh flake.lock; do
  H=$(head_after input "$input")
  expect "mode: a night after $input changed runs everything" \
    "$(lines mode=full base= "reason=$input changed since green night $A")" in_repo mode schedule "$A" "$H"
done
expect "mode: a night after only tests and Cargo.lock changed stays in-diff" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green night $A")" in_repo mode schedule "$A" "$H_tests"
expect "mode: only the root .cargo/mutants.toml forces everything" \
  "$(lines mode=in-diff "base=$A" "reason=diff from green night $A")" in_repo mode schedule "$A" "$H_nested"
expect "mode: a dispatch tests everything, whatever the green night" \
  "$(lines mode=full base= "reason=a dispatch tests every mutant of its packages")" \
  in_repo mode workflow_dispatch "$A" "$H_rs"
expect_refusal "mode: a push is no trigger" "unexpected event 'push'" in_repo mode push "$A" "$H_rs"

expect "packages: unset plans every package the listing knows" "$(lines --package=core --package=util)" \
  "${plan[@]}" packages "$fixtures/list.json" ""
expect "packages: a named package, once each" "--package=util" \
  "${plan[@]}" packages "$fixtures/list.json" '["util","util"]'
expect_refusal "packages: an empty array is refused" "non-empty JSON array" \
  "${plan[@]}" packages "$fixtures/list.json" '[]'
expect_refusal "packages: an unknown package is refused" "known: core, util" \
  "${plan[@]}" packages "$fixtures/list.json" '["core","nope"]'
expect_refusal "packages: a bare name is not JSON" "not JSON" "${plan[@]}" packages "$fixtures/list.json" util

tested=6666666666666666666666666666666666666666
expect "resume: completed runs on main that tested this commit, by title or as a push" "$(lines 31 30 35)" \
  "${plan[@]}" resume-runs "$fixtures/runs-resume.json" 32 1 "$tested"
expect "resume: a re-run attempt also resumes its own run" "$(lines 32 31 30 35)" \
  "${plan[@]}" resume-runs "$fixtures/runs-resume.json" 32 2 "$tested"
expect_refusal "resume: an API error body is a failure" "Bad credentials" \
  "${plan[@]}" resume-runs "$fixtures/runs-error.json" 32 1 "$tested"
expect "timing source: completed runs on main, newest first" "$(lines 42 41)" \
  "${plan[@]}" timing-source "$fixtures/runs-timing.json"
expect "artifacts: unexpired plans and mutants.out, including the unsharded name" \
  "$(lines '1 mutants-plan-1' '2 mutants.out-1-0' '3 mutants.out')" "${plan[@]}" artifacts "$fixtures/artifacts.json"
expect_refusal "artifacts: an API error body is a failure" "Not Found" \
  "${plan[@]}" artifacts "$fixtures/artifacts-error.json"

outcomes() {
  local dir=$work/$1
  shift
  mkdir -p "$dir"
  jq -n --args '{outcomes: ([{scenario: "Baseline", phase_results: [{duration: 90}]}]
    + [$ARGS.positional as $p | range(0; $p | length; 2) | [$p[.], $p[. + 1]] | {scenario: {Mutant: {name: "\(.[0]):1:1: replace f with g"}},
      phase_results: [{duration: (.[1] | tonumber)}]}])}' "$@" > "$dir/outcomes.json"
  echo "$dir"
}
s0=$(outcomes timed/mutants.out-1-0 src/a.rs 10 src/a.rs 20)
s1=$(outcomes timed/mutants.out-1-1 src/b.rs 30)
s2=$(outcomes timed/mutants.out-1-2 src/b.rs 90)
expect "timings: per-file means, and the most a succeeded shard spent beside its mutants (baseline included)" \
  '{"source":"measured","overhead":270,"mean":37.5,"per_file":{"src/a.rs":15,"src/b.rs":60}}' \
  sh -c '"$@" | jq -c .' - "${plan[@]}" timings "$fixtures/jobs.json" "$s0/" "$s1" "$s2"
expect "timings: nothing measured falls back to the labelled model" \
  '{"source":"model from run 38447766912","overhead":471,"mean":19,"per_file":{}}' \
  sh -c '"$@" | jq -c .' - "${plan[@]}" timings "$fixtures/jobs.json" "$work/absent"

echo '{"source":"measured","overhead":100,"mean":20,"per_file":{"src/a.rs":50}}' > "$work/timings.json"
lines 'src/a.rs:1:1: m1' 'src/b.rs:1:1: m2' 'src/a.rs:2:1: m3' 'src/b.rs:2:1: m4' > "$work/four.txt"
: > "$work/none.txt"
expect "shards: one shard when the whole list fits" \
  "$(lines n=1 'shards=[0]' predicted=240 over=false minutes=4 source=measured)" \
  "${plan[@]}" shards "$work/timings.json" "$work/four.txt" 300
expect "shards: the fewest round-robin shards whose heaviest fits the budget exactly" \
  "$(lines n=2 'shards=[0,1]' predicted=200 over=false minutes=6 source=measured)" \
  "${plan[@]}" shards "$work/timings.json" "$work/four.txt" 200
expect "shards: one mutant per shard, flagged over, when one mutant cannot fit" \
  "$(lines n=4 'shards=[0,1,2,3]' predicted=150 over=true minutes=9 source=measured)" \
  "${plan[@]}" shards "$work/timings.json" "$work/four.txt" 120
expect "shards: nothing left to test starts no shard" \
  "$(lines n=0 'shards=[]' predicted=0 over=false minutes=0 source=measured)" \
  "${plan[@]}" shards "$work/timings.json" "$work/none.txt" 600

echo '{"checkout":{"outcome":"success"},"tools":{"outcome":"failure"},"list":{"outcome":"skipped"}}' > "$work/failed.json"
echo '{"checkout":{"outcome":"success"},"mutants":{"outcome":"cancelled"}}' > "$work/cancelled.json"
echo '{"checkout":{"outcome":"failure"},"tools":{"outcome":"skipped"}}' > "$work/setup.json"
echo '{"checkout":{"outcome":"success"},"tools":{"outcome":"success"}}' > "$work/green.json"
codes='tools=mutants-plan-setup list=mutants-plan-list mutants=mutants-run'
expect "job report: a green job writes only its wall time" "**plan wall time:** 75s" \
  "${plan[@]}" job-report plan 100 175 "$codes" "dispatch" "$work/green.json"
expect "job report: the first failed step's code from the closed set" \
  "$(lines '**plan wall time:** 75s' '' '### plan failed: `mutants-plan-setup`' '' '- **Step:** `tools` (failure)' \
    '- **Reproduce:** `dispatch`')" \
  "${plan[@]}" job-report plan 100 175 "$codes" "dispatch" "$work/failed.json"
expect "job report: a cancelled step is a timeout" \
  "$(lines '**shard 3 wall time:** 3600s' '' '### shard 3 failed: `mutants-timeout`' '' \
    '- **Step:** `mutants` (cancelled)' '- **Reproduce:** `dispatch`')" \
  "${plan[@]}" job-report "shard 3" 0 3600 "$codes" "dispatch" "$work/cancelled.json"
expect "job report: a step outside the closed set is unclassified and named" \
  "$(lines '**plan wall time:** 1s' '' '### plan failed: `unclassified`' '' '- **Step:** `checkout` (failure)' \
    '- **Reproduce:** `dispatch`')" \
  "${plan[@]}" job-report plan 1 2 "$codes" "dispatch" "$work/setup.json"

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
