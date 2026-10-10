#!/bin/sh
set -eu

bin=$1
root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d "${TMPDIR:-/tmp}/gritlint-journey.XXXXXX")
trap 'rm -rf "$work"' EXIT

expect() {
    status=0
    "$bin" check >"$work/out" 2>&1 || status=$?
    if [ "$status" -ne "$1" ] || { [ $# -gt 1 ] && ! grep -qF ": $2: " "$work/out"; }; then
        cat "$work/out" >&2
        echo "gate-journey: $PWD: expected exit $1${2:+ naming $2}, got $status" >&2
        exit 1
    fi
}

mkdir -p "$work/build-mode/packages/app"
cd "$work/build-mode"
printf '{"packs": {"typecheck-build-mode": {}}}\n' >gritlint.json
printf '{"files": [], "references": [{"path": "./tsconfig.app.json"}]}\n' >packages/app/tsconfig.json
printf '{"name": "app", "scripts": {"typecheck": "tsc --noEmit"}}\n' >packages/app/package.json
expect 1 typecheck-build-mode/typecheck-build-mode
printf '{"name": "app", "scripts": {"typecheck": "tsc -b"}}\n' >packages/app/package.json
expect 0

mkdir -p "$work/provenance/pkg"
cd "$work/provenance"
cp "$root/gritlint.json" .
published() {
    printf '{"name": "@fixture/pkg", "version": "0.0.0", "repository": {"type": "git", "url": "%s", "directory": "pkg"}, "publishConfig": {"access": "public", "provenance": true}}\n' "$1" >pkg/package.json
}
published git+https://github.com/systemfsoftware/systemfsoftware.git
expect 1 npm-provenance/repository-url
published git+https://github.com/systemfsoftware/gritlint.git
expect 0
