---
title: A Rust binary cross-compiled on one system differs from the same target built natively on another
date: 2026-10-11
category: build-errors
module: gritlint
problem_type: build_error
component: tooling
symptoms:
  - "tarballs-match: aarch64-linux builds gritlint-0.1.0.tgz differently from x86_64-linux"
  - "Both static binaries differ by millions of bytes between the system that builds them natively and the one that cross-compiles them"
root_cause: config_error
resolution_type: config_change
severity: high
related_components:
  - nix/gritlint-static.nix
  - .github/workflows/ci.yml
tags: [nix, crane, cargo, rustc, cross-compilation, reproducible-builds, linker, glibc, metadata]
---

# A Rust binary cross-compiled on one system differs from the same target built natively on another

## Problem

The npm tarball holds two static glibc binaries, `gritlint-linux-x64` and `gritlint-linux-arm64`. Each CI system builds both: one natively, the other through `pkgsCross`. A consumer's lockfile records the tarball's integrity once, so both systems must produce the same bytes (R29). They did not. On PR #15, `tarballs-match` found both binaries different, with millions of differing bytes each.

Three separate inputs depended on the system doing the build, not on the target. Each one had to be removed before the next one showed up.

## Root cause and fix, in the order they appeared

1. **The linker follows the host.** Since rust 1.90, rustc links `x86_64-unknown-linux-gnu` with its bundled `rust-lld`, and on an x86_64 host it also used `rust-lld` for the aarch64 cross target. The aarch64 host linked with GNU ld. `readelf -p .comment` showed `Linker: LLD` only on the x86_64-built binaries. rustc 1.99 accepts `-C linker-features=-lld` and `-C link-self-contained=-linker` only for `x86_64-unknown-linux-gnu`; for aarch64 they are unstable. The portable fix is to choose the linker through the C compiler driver, which takes the last `-fuse-ld`: `-C link-arg=-fuse-ld=bfd` for both targets.

2. **The cross glibc is a different derivation.** The `pkgsCross` glibc and the target system's own glibc have different store paths, and static glibc compiles its prefixes (`gconv`, `locale`, `ld.so.cache`) into `libc.a`, so those paths end up in the binary. Link each target against the target system's native `nixpkgs.legacyPackages.<target>.glibc.static`. It is substituted from cache.nixos.org on the foreign host, never built there.

3. **Cargo's `-C metadata` mixes in host units.** With linker and libc equal, only the Rust objects of the final crate still differed (`.text` and `.eh_frame`, found by diffing GNU ld link maps with store hashes normalized). Cargo leaves rustc's `host:` line out of a target unit's metadata, but its `compute_metadata` mixes in the metadata of every dependency. Build scripts and proc macros are host units, and `hash_rustc_version` adds rustc's host triple to a host unit's hash. So each target crate gets a different `-C metadata` on each system. rustc mixes that hash into every symbol name, and the different names change what LTO inlines and how code is laid out. The fix is a `RUSTC_WRAPPER` that replaces the metadata of target units (rustc called with `--target`) with a hash of host-independent inputs: crate name, package name and version, crate type, target and sorted features. Host units pass through unchanged.

## Guarding the fix

The wrapper's log lets the build check itself. The static build's `postBuild` (`metadataLog` in gritlint-static.nix) fails when no target unit was normalized, which means the wrapper's parsing no longer matches cargo's arguments, and when two cargo hashes map to one normalized hash. The cross-system comparison stays the final gate. It fails when either system's artifact is missing, when a file is in only one of them, or when any file differs.

## Why This Works

A build is reproducible across hosts only when every input that reaches the output is a function of the target, never of the build host:

```
output(target) = f(source, toolchain(target), libc(target), symbol_hash(target))
```

The three fixes remove the three host terms in turn: the linker (`-fuse-ld=bfd` everywhere), the libc derivation (the target's native `glibc.static`), and the symbol hash (`-C metadata` derived from host-independent inputs). Any term left as `f(host)` shows up only when two different hosts build the same target.

## Prevention

Compare the outputs of two different build hosts, not two builds on one host. A rebuild on the same system reproduces all three of these inputs exactly, so `nix build --rebuild` passes when the two systems still disagree.

## Related Issues

- `docs/plans/2026-10-12-2025-feat-gritlint-standalone-plan.md` R29, KTD32, KTD38
- `docs/solutions/logic-errors/flake-sandbox-check-compared-two-builds-of-one-file.md`
