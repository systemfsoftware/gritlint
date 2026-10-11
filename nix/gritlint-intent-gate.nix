# scripts/tools/intent-gate.sh over a scratch history: a change under
# `crates/` needs a new `.changeset/*.md`, and a changelog is not one.
{ runCommand, git }:
runCommand "gritlint-intent-gate" { nativeBuildInputs = [ git ]; } ''
  export HOME=$TMPDIR/home
  mkdir -p "$HOME"
  git config --global user.name "Intent Gate"
  git config --global user.email "intent-gate@example.invalid"

  gate=${../scripts/tools/intent-gate.sh}

  work=$TMPDIR/repo
  mkdir -p "$work"
  cd "$work"
  git init -q -b main
  echo base > base.txt
  git add base.txt
  git commit -q -m base
  base=$(git rev-parse HEAD)

  check() {
    expected=$1
    label=$2
    head=$3
    status=0
    sh "$gate" "$base" "$head" > "$work/out" 2>&1 || status=$?
    if [ "$status" -ne "$expected" ]; then
      cat "$work/out" >&2
      echo "case $label: expected exit $expected, got $status" >&2
      exit 1
    fi
  }

  # (a) a crates change with no intent
  git checkout -q -b case-a "$base"
  mkdir -p crates
  echo 'pub fn x() {}' > crates/x.rs
  git add crates/x.rs
  git commit -q -m "crates change"
  check 1 a "$(git rev-parse HEAD)"

  # (b) the same change with an intent
  git checkout -q -b case-b "$base"
  mkdir -p crates .changeset
  echo 'pub fn x() {}' > crates/x.rs
  echo intent > .changeset/note.md
  git add crates/x.rs .changeset/note.md
  git commit -q -m "crates change with intent"
  check 0 b "$(git rev-parse HEAD)"

  # (c) a nested changelog is not an intent
  git checkout -q -b case-c "$base"
  mkdir -p crates .changeset/changelogs
  echo 'pub fn x() {}' > crates/x.rs
  echo changelog > .changeset/changelogs/x.md
  git add crates/x.rs .changeset/changelogs/x.md
  git commit -q -m "crates change with changelog"
  check 1 c "$(git rev-parse HEAD)"

  # (d) a change outside the gated trees
  git checkout -q -b case-d "$base"
  mkdir -p docs
  echo docs > docs/x.md
  git add docs/x.md
  git commit -q -m "docs change"
  check 0 d "$(git rev-parse HEAD)"

  touch $out
''
