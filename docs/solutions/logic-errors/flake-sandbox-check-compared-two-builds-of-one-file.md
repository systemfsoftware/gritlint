---
title: A flake check that compares drvPaths of one file cannot catch a broken sandbox
date: 2026-10-10
category: logic-errors
module: gritlint
problem_type: logic_error
component: tooling
symptoms:
  - "checks.<system>.gritlint-is-sandboxed stays green when nix/gritlint-sandbox.nix stops calling bubblewrap"
  - "Three reviewers flagged the guard as asserting derivation identity, not isolation"
root_cause: logic_error
resolution_type: test_fix
severity: high
related_components:
  - flake.nix
  - nix/gritlint-sandbox.nix
  - .github/workflows/ci.yml
tags: [nix, flake-check, bubblewrap, sandbox, override, ci]
---

# A flake check that compares drvPaths of one file cannot catch a broken sandbox

## Problem

The first `gritlint-is-sandboxed` check passed only if `packages.<system>.gritlint.drvPath` equalled a second `callPackage` of the sandbox wrapper (`gritlint-sandbox.nix`) over `gritlint-unwrapped`. Both sides come from the same file, so any edit to that file moves both sides together. A wrapper with no `bwrap` call still passes.

## What Didn't Work

- The drvPath-equality guard PR1 first shipped. It catches only a rebinding of the attribute name, never a loss of isolation.
- A text check such as `grep -- --unshare-all` on the wrapper. That asserts the author's spelling, not what the kernel enforces.

## Solution

Run the exported wrapper over a probe inside the build (`flake.nix`, `checks.gritlint-is-sandboxed`):

```nix
escapeProbe = pkgs.writeShellScriptBin "gritlint" ''
  escaped=0
  if [ -e "$1" ]; then echo "read $1 outside the working directory"; escaped=1; fi
  if { : > written; } 2>/dev/null; then echo "wrote the working directory"; escaped=1; fi
  if [ -n "''${GRITLINT_PROBE_ENV-}" ]; then echo "kept the caller's environment"; escaped=1; fi
  exit "$escaped"
'';
# in runCommand: the unsandboxed probe must report escapes (control), then
GRITLINT_PROBE_ENV=1 ${exported.gritlint.override { gritlint = escapeProbe; }}/bin/gritlint "$NIX_BUILD_TOP/outside"
```

`.override` rebuilds the wrapper exactly as exported, with only its inner binary swapped. If the attribute is rebound to the plain build, `override` fails, because the unwrapped derivation (`gritlint.nix`) takes no `gritlint` argument and no `...`.

## Why This Works

The check observes behaviour: what the probe can see and do inside the wrapper. Each of these breaks fails it, as shown by builds of scratch copies on 2026-10-10: dropping `bwrap`, `--bind` in place of `--ro-bind "$PWD"`, binding `/`, dropping `--clearenv`, and rebinding `packages.gritlint`. The control run stops a probe that sees nothing from passing vacuously.

## Prevention

- bubblewrap runs inside the Nix build sandbox. On GitHub's ubuntu-24.04 images, the step `sysctl -w kernel.apparmor_restrict_unprivileged_userns=0` must run **before** the step that builds `checks.*`, not only before the journey that runs the binary (the `nix` job in `ci.yml`).
- The probe cannot test network isolation: the build sandbox has no peer to reach. `--unshare-all` covers it, but nothing exercises it.
- For a wrapper whose isolation is the product, gate on a probe, never on derivation identity.

## Related Issues

- `docs/plans/2026-10-11-2312-feat-gritlint-standalone-plan.md` R7, KTD2, AE4
