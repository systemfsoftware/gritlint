# gritlint, compiled from Cargo.lock with crane.
#
# Three facts about the pinned engine drive this file.
#
# 1. Cargo.lock is the only place the biomejs/gritql rev appears; it is read
#    out of the lock here for the engine checkout below.
# 2. The gritql checkout holds two crates with one name in several places
#    (`tree-sitter-c-sharp` 0.23.1 and 0.20.0, `tree-sitter-php`), and three
#    locked crates live only in its submodules. `importCargoLock` takes the
#    first `find` hit by name, so its vendor tree differed between machines
#    under one derivation hash. `fetchCargoVendor` picks deterministically,
#    and its fixed-output hash fails loudly if the vendor tree ever changes,
#    so it vendors here rather than crane's own git vendoring. A re-pin
#    updates `cargoHash`.
# 3. marzano-language reads `include_str!("../../../resources/node-types/…")`
#    from outside its crate, which from `<vendor>/source-git-0/<crate>/src`
#    is the vendor root. `vendor` copies those files there from a checkout
#    of the pinned rev.
#
# The package links its dependencies with fat LTO, so it builds against
# release-profile dependency artifacts; clippy, the tests and `cargo tree`
# share dev-profile ones (`passthru`).
{
  lib,
  craneLib,
  rustPlatform,
  runCommand,
  version,
}:

let
  engineRepo = "https://github.com/biomejs/gritql";

  # `builtins.match` is POSIX ERE, so the `+`, `.` and `?` are escaped.
  engineRevs = lib.unique (
    builtins.concatLists (
      builtins.filter (hit: hit != null) (
        map (builtins.match "source = \"git\\+https://github\\.com/biomejs/gritql\\?rev=([0-9a-f]+)#.*\"") (
          lib.splitString "\n" (builtins.readFile ../Cargo.lock)
        )
      )
    )
  );

  engineRev =
    assert lib.assertMsg (builtins.length engineRevs == 1)
      "nix/gritlint.nix: Cargo.lock must pin exactly one biomejs/gritql rev, found ${toString (builtins.length engineRevs)}";
    builtins.head engineRevs;

  engineSource = builtins.fetchGit {
    url = engineRepo;
    rev = engineRev;
  };

  # Only what cargo and clippy read: a flake, workflow or docs edit must not
  # move the src hash and rebuild the Rust output.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../clippy.toml
      ../apps/gritlint
      ../crates/gritlint_core
      ../packs
    ];
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    src = lib.fileset.toSource {
      root = ../.;
      fileset = ../Cargo.lock;
    };
    hash = "sha256-j96CiB8XTiRqyZ79LjUnqyOsmkAnwmu2/niYeBnSmdI=";
  };

  # The registry crates are linked as they are; only the git crates need the
  # engine's resources beside them, so only they are copied.
  vendor = runCommand "gritlint-vendor" { } ''
    mkdir -p $out/resources
    ln -s ${cargoDeps}/source-registry-0 $out/source-registry-0
    cp -r ${cargoDeps}/source-git-0 $out/source-git-0
    cp -r ${engineSource}/resources/node-types $out/resources/node-types
    substitute ${cargoDeps}/.cargo/config.toml $out/config.toml --subst-var-by vendor $out
  '';

  common = {
    pname = "gritlint";
    inherit version src;
    strictDeps = true;
    cargoVendorDir = vendor;
  };

  cargoArtifacts = craneLib.buildDepsOnly (
    common
    // {
      buildPhaseCargoCommand = "cargoWithProfile build --locked -p gritlint";
      doCheck = false;
    }
  );

  devArgs = common // {
    CARGO_PROFILE = "dev";
  };

  devArtifacts = craneLib.buildDepsOnly devArgs;
in
craneLib.buildPackage (
  common
  // {
    inherit cargoArtifacts;
    cargoExtraArgs = "--locked -p gritlint";
    # The tests run as `checks.<system>.gritlint-nextest` against dev-profile
    # artifacts, not here under fat LTO.
    doCheck = false;

    passthru = {
      inherit
        engineRev
        engineSource
        vendor
        cargoArtifacts
        devArgs
        devArtifacts
        ;
    };

    meta = {
      description = "Run GritQL rule packs over the config files of a repository";
      homepage = "https://github.com/systemfsoftware/gritlint";
      license = lib.licenses.mit;
      mainProgram = "gritlint";
      platforms = lib.platforms.unix;
    };
  }
)
