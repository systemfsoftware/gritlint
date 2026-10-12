#!/usr/bin/env bash
set -uo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0
cases=0

export HOME=$work/home GIT_CONFIG_NOSYSTEM=1
mkdir -p "$HOME"
git config --global user.name "Intent Gate"
git config --global user.email "intent-gate@example.invalid"

cd "$work"
git init -q -b main repo
cd repo
echo base >base.txt
git add base.txt
git commit -q -m base
base=$(git rev-parse HEAD)

expect_gate() {
  local name=$1 expected=$2 status=0
  shift 2
  git checkout -q -b "case-$((++cases))" "$base"
  for file; do
    mkdir -p "$(dirname "$file")"
    echo "$name" >"$file"
  done
  git add "$@"
  git commit -q -m "$name"
  sh "$here/intent-gate.sh" "$base" HEAD >"$work/out" 2>&1 || status=$?
  if [ "$status" -eq "$expected" ]; then
    echo "ok - $name"
  else
    echo "not ok - $name: expected exit $expected, got $status" >&2
    cat "$work/out" >&2
    failures=$((failures + 1))
  fi
}

expect_gate "a crates change with no intent" 1 crates/x.rs
expect_gate "the same change with an intent" 0 crates/x.rs .changeset/note.md
expect_gate "a nested changelog is not an intent" 1 crates/x.rs .changeset/changelogs/x.md
expect_gate "a change outside the gated trees" 0 docs/x.md

exit $((failures > 0))
