# gritlint as two static glibc binaries, `gritlint-linux-x64` and
# `gritlint-linux-arm64`, for the npm tarball. Each system builds both: the
# target that matches it natively, the other through `pkgsCross`. The tarball
# records its integrity once, so the two systems must agree byte for byte.
#
# Static glibc keeps glibc's allocator; musl's measured 80x slower on the
# bench corpus (plan Q2). It cannot use NSS or `dlopen`, and gritlint does
# neither.
{
  lib,
  pkgs,
  # system -> nixpkgs cross-compiling from this system to it, with rust-overlay
  pkgsFor,
  # system -> nixpkgs native to it; its glibc is substituted, never built here
  nativePkgsFor,
  crane,
  toolchainFile,
  # `packages.<system>.gritlint-unwrapped`, for its release arguments and vendor tree
  release,
}:

let
  targets = [
    {
      suffix = "linux-x64";
      system = "x86_64-linux";
    }
    {
      suffix = "linux-arm64";
      system = "aarch64-linux";
    }
  ];

  inherit (release) vendor;

  # Cargo gives every unit a `-C metadata` hash, and rustc mixes that hash into
  # every symbol name the crate defines. For a target unit, cargo leaves out
  # rustc's `host:` line, but it mixes in the metadata of every dependency
  # (cargo `src/compiler/build_runner/compilation_files.rs:849-856`). Build
  # scripts and proc macros are host units, and their hash includes the host
  # (`:929-934`). So the same target crate gets a different hash on each
  # system. For target units only (`--target` is given), this wrapper swaps
  # cargo's value for a hash of host-independent inputs: the crate name, the
  # package name and version, the crate type, the target and the sorted
  # features. Host units and every other argument pass through unchanged,
  # `-C extra-filename` included. The wrapper changes only the symbol
  # disambiguators, never a codegen flag.
  rustcWrapper = pkgs.writeShellScript "gritlint-rustc-metadata" ''
    set -eu
    rustc=$1
    shift
    target="" crate="" type="" meta="" prev=""
    features=()
    for arg; do
      case $prev in
        --target) target=$arg ;;
        --crate-name) crate=$arg ;;
        --crate-type) type="$type$arg," ;;
        --cfg) case $arg in feature=*) features+=("$arg") ;; esac ;;
        -C) case $arg in metadata=*) meta=''${arg#metadata=} ;; esac ;;
      esac
      prev=$arg
    done
    if [ -z "$meta" ]; then
      exec "$rustc" "$@"
    fi
    if [ -z "$target" ]; then
      echo "$GRITLINT_RUSTC_BUILD passthrough $crate $meta" >>"$GRITLINT_RUSTC_METADATA_LOG"
      exec "$rustc" "$@"
    fi
    normalized=$(
      {
        printf '%s\n' "$crate" "''${CARGO_PKG_NAME:-}" "''${CARGO_PKG_VERSION:-}" "$type" "$target"
        printf '%s\n' "''${features[@]}" | LC_ALL=C sort
      } | sha256sum | cut -c1-16
    )
    args=()
    prev=""
    for arg; do
      if [ "$prev" = -C ] && [ "$arg" = "metadata=$meta" ]; then
        arg=metadata=$normalized
      fi
      args+=("$arg")
      prev=$arg
    done
    echo "$GRITLINT_RUSTC_BUILD normalized $crate $meta $normalized $target" >>"$GRITLINT_RUSTC_METADATA_LOG"
    exec "$rustc" "''${args[@]}"
  '';

  # The log lives in the target directory, so the package build inherits the
  # dependency build's entries along with its artifacts. Within one cargo
  # build, two cargo hashes are two units, and they must not share a
  # normalized hash. Across the two builds, a workspace crate is built once as
  # crane's dummy and again from its source, under a new cargo hash. So each
  # build checks only its own entries.
  metadataLog = {
    RUSTC_WRAPPER = rustcWrapper;
    preBuild = ''
      mkdir -p "''${CARGO_TARGET_DIR:-target}"
      export GRITLINT_RUSTC_METADATA_LOG=$(realpath "''${CARGO_TARGET_DIR:-target}")/rustc-metadata.log
      export GRITLINT_RUSTC_BUILD=$pname
      touch "$GRITLINT_RUSTC_METADATA_LOG"
    '';
    postBuild = ''
      log=$GRITLINT_RUSTC_METADATA_LOG
      own() { awk -v build="$pname" -v kind="$1" '$1 == build && $2 == kind' "$log"; }
      first=$(own normalized | awk 'NR == 1')
      if [ -z "$first" ]; then
        echo "rustc wrapper: no target unit was normalized in $pname" >&2
        exit 1
      fi
      echo "rustc wrapper, a normalized target unit: $first"
      host=$(own passthrough | awk 'NR == 1')
      echo "rustc wrapper, a passthrough host unit: ''${host:-none rebuilt in this build}"
      shared=$(own normalized | awk '{ print $5, $4 }' | sort -u | awk '{ n[$1]++ } END { for (m in n) if (n[m] > 1) print m }')
      if [ -n "$shared" ]; then
        echo "rustc wrapper: two target units share a normalized metadata:" >&2
        for m in $shared; do own normalized | awk -v m="$m" '$5 == m' | sort -u >&2; done
        exit 1
      fi
      echo "rustc wrapper: $(own normalized | awk '{ print $4 }' | sort -u | wc -l) target units in $pname; normalized set so far $(awk '$2 == "normalized" { print $3, $5, $6 }' "$log" | sort -u | sha256sum | cut -c1-16)"
    '';
  };

  buildFor =
    target:
    let
      targetPkgs = if target.system == pkgs.stdenv.hostPlatform.system then pkgs else pkgsFor target.system;
      platform = targetPkgs.stdenv.hostPlatform;
      rustTarget = platform.rust.rustcTarget;
      craneLib = (crane.mkLib targetPkgs).overrideToolchain (
        p: (p.rust-bin.fromRustupToolchainFile toolchainFile).override { targets = [ rustTarget ]; }
      );
      # The vendor tree is a per-system derivation, and its path reaches panic
      # locations and C `assert` strings; both systems must emit the same bytes.
      remap = "/gritlint-vendor";
      cFlags = "-ffile-prefix-map=${vendor}=${remap}";
      cEnvTarget = lib.replaceStrings [ "-" ] [ "_" ] rustTarget;
      args = release.releaseArgs // metadataLog // {
        pname = "gritlint-${target.suffix}";
        # Always explicit, even for the native target: cargo then keeps the
        # target's flags off build scripts and proc macros. Linking those
        # against a static libc makes them crash.
        CARGO_BUILD_TARGET = rustTarget;
        "CARGO_TARGET_${platform.rust.cargoEnvVarTarget}_RUSTFLAGS" = lib.concatStringsSep " " [
          "-C target-feature=+crt-static"
          # libc.a compiles its own store path into the binary, and a cross
          # glibc is another derivation from the target's native one. Both
          # systems link the target's native static glibc.
          "-C link-arg=-L${(nativePkgsFor target.system).glibc.static}/lib"
          # rustc picks rust-lld or GNU ld by the host it runs on, and the two
          # lay a binary out differently. GNU ld on every host: rust 1.99 accepts
          # its linker-selecting flags only for x86_64-unknown-linux-gnu, so the
          # choice goes to the C compiler driver, which takes the last `-fuse-ld`.
          "-C link-arg=-fuse-ld=bfd"
          "--remap-path-prefix=${vendor}=${remap}"
        ];
        "CFLAGS_${cEnvTarget}" = cFlags;
        "CXXFLAGS_${cEnvTarget}" = cFlags;
      };
    in
    craneLib.buildPackage (
      args
      // {
        cargoArtifacts = craneLib.buildDepsOnly (
          args
          // {
            buildPhaseCargoCommand = "cargoWithProfile build --locked -p gritlint";
            doCheck = false;
          }
        );
        cargoExtraArgs = "--locked -p gritlint";
        doCheck = false;
      }
    );

  built = map (target: target // { package = buildFor target; }) targets;
in
pkgs.runCommand "gritlint-binaries-${release.version}"
  {
    passthru = lib.listToAttrs (map (target: lib.nameValuePair target.suffix target.package) built);
  }
  ''
    mkdir $out
    ${lib.concatMapStrings (target: ''
      cp ${target.package}/bin/gritlint $out/gritlint-${target.suffix}
    '') built}
  ''
