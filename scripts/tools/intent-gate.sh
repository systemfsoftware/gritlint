#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: intent-gate.sh <base> <head>" >&2
    exit 2
fi

base=$1
head=$2

changed=$(git diff --name-only "$base...$head")
gated=$(printf '%s\n' "$changed" | grep -E '^(crates|apps|packs)/' || true)
if [ -z "$gated" ]; then
    exit 0
fi

intents=$(git diff --name-only --diff-filter=A "$base...$head" -- ':(glob).changeset/*.md' | grep -v '^\.changeset/README\.md$' || true)
if [ -n "$intents" ]; then
    exit 0
fi

printf '%s\n' "$gated" >&2
echo "add a .changeset/*.md intent" >&2
exit 1
