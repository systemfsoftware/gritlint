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
      args = release.releaseArgs // {
        pname = "gritlint-${target.suffix}";
        # Always explicit, even for the native target: cargo then keeps the
        # target's flags off build scripts and proc macros. Linking those
        # against a static libc makes them crash.
        CARGO_BUILD_TARGET = rustTarget;
        "CARGO_TARGET_${platform.rust.cargoEnvVarTarget}_RUSTFLAGS" = lib.concatStringsSep " " [
          "-C target-feature=+crt-static"
          # Only the target link sees the static libc.
          "-C link-arg=-L${targetPkgs.glibc.static}/lib"
          "--remap-path-prefix=${vendor}=${remap}"
        ];
        "CFLAGS_${lib.replaceStrings [ "-" ] [ "_" ] rustTarget}" = cFlags;
        "CXXFLAGS_${lib.replaceStrings [ "-" ] [ "_" ] rustTarget}" = cFlags;
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
