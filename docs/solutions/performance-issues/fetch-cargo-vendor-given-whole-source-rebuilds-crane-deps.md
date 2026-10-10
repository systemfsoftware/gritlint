---
title: fetchCargoVendor given the whole source makes every source edit rebuild crane's dependency derivations
date: 2026-10-10
category: performance-issues
module: gritlint
problem_type: performance_issue
component: tooling
symptoms:
  - "A one-line edit to the CLI's main.rs changed the drvPath of gritlint-vendor and both crane buildDepsOnly derivations"
  - "Every commit would rebuild the GritQL dependency tree despite crane's dependency split"
root_cause: config_error
resolution_type: config_change
severity: high
related_components:
  - nix/gritlint.nix
  - .github/workflows/ci.yml
tags: [nix, crane, fetchCargoVendor, cargo, vendor, drvPath, ci-cache, binary-cache]
---

# fetchCargoVendor given the whole source makes every source edit rebuild crane's dependency derivations

## Problem

gritlint builds with crane over a vendor tree from `rustPlatform.fetchCargoVendor` (crane's own git vendoring cannot handle GritQL's duplicate crate names). The vendor took `src`, the same fileset the package builds from. A source-only edit then changed the drvPath of the vendor and of both `craneLib.buildDepsOnly` derivations, so crane's dependency split saved nothing: every commit recompiled the GritQL crates.

## What Didn't Work

Trusting that `fetchCargoVendor`'s fixed-output hash isolates its dependents. The fixed-output staging derivation does keep a stable output, but the vendor derivation `fetchCargoVendor` returns is input-addressed. It moved with `src`, and crane's `cargoVendorDir` carried that change into every dependency derivation.

## Solution

Give `fetchCargoVendor` a source holding only `Cargo.lock`. The vendored crates depend on nothing else, so the existing `hash` stays valid:

```nix
cargoDeps = rustPlatform.fetchCargoVendor {
  src = lib.fileset.toSource {
    root = ../.;
    fileset = ../Cargo.lock;
  };
  hash = "sha256-…";
};
```

After the change, the same edit changes only `gritlint-unwrapped`'s drvPath. `vendor`, `cargoArtifacts` and `devArtifacts` stay identical.

## Why This Works

Nix derivation hashes cover every input path. A wider `src` makes any file in it an input of the vendor derivation and of everything built over it. A lock-only fileset narrows the vendor's inputs to what actually determines its contents.

## Prevention

- Probe the dependency split before trusting it. Evaluate `.drvPath` of the vendor and of each dependency derivation in two scratch copies that differ by one source line. Every one must match.
- Give each derivation in a crane pipeline the narrowest source that determines its output, not the package's `src`.
- A related constraint from the same PR's CI store cache: a `file://` binary cache refuses `nix copy --no-recursive` of a path whose references it does not hold. A cache that starts from unsigned roots therefore still carries their full closures, including signed paths.

## Related Issues

- `docs/plans/2026-10-10-0452-feat-gritlint-standalone-plan.md` R12, KTD10, KTD13, U5
