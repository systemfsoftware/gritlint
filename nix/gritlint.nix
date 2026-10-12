# gritlint, compiled from Cargo.lock with crane.
#
# Five facts about the pinned engine drive this file.
#
# 1. Cargo.lock is the only place the biomejs/gritql rev appears; it is read
#    out of the lock here for the engine checkout below.
# 2. Sixteen grammar crates appear twice in the gritql checkout under one
#    name and version: the engine's manifests build the copies in
#    `resources/language-metavariables`, and the git submodules in
#    `resources/language-submodules` hold the upstream ones. crane vendors a
#    git checkout by keeping the first `find` hit per name and version, an
#    order that differs between filesystems, so `cargoDeps` deletes the
#    upstream copies before crane scans the checkout. The two `xtask` crates
#    under `vendor/` also share a name and version, but they are workspace
#    members there and no lock entry names them, so they stay and are never
#    linked. Three locked crates live only in the engine's submodules, which
#    crane fetches.
# 3. crane copies a git crate's files as `cargo package --list` names them,
#    and cargo leaves out any directory holding its own Cargo.toml. The
#    grammars `tree-sitter-php` and `tree-sitter-md` keep their sources in
#    such directories (`php/src`, `tree-sitter-markdown/src`), so `cargoDeps`
#    deletes the nested manifests under `resources/language-metavariables`;
#    no lock entry names them.
# 4. marzano-language reads `include_str!("../../../resources/node-types/…")`
#    from outside its crate, which from `<vendor>/<source>/<crate>/src` is the
#    vendor root. crane links each crate from its own store path, so `vendor`
#    copies the engine's source directory and puts those files beside it,
#    from a checkout of the pinned rev.
# 5. The root Cargo.toml patches the engine's `tree-sitter-typescript` with
#    `vendor/tree-sitter-typescript`, a path crate. crane's dummy source stubs
#    every crate's sources, which leaves marzano-language without the
#    grammar's functions in the dependency builds, so `common` copies the real
#    crate back over the stub. It comes from its own store path, so a
#    workspace edit does not rebuild the dependencies.
#
# The vendor carries no fixed-output hash: crane fetches each registry crate
# by its Cargo.lock checksum and the engine with `builtins.fetchGit` at the
# locked rev, and reads nothing else from the lock. A version bump rewrites
# only the workspace members' lock entries, so it leaves the vendor as it was.
#
# The package links its dependencies with fat LTO, so it builds against
# release-profile dependency artifacts; clippy, the tests and `cargo tree`
# share dev-profile ones (`passthru`).
{
  lib,
  craneLib,
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
      ../apps/gritlint-e2e
      ../crates/gritlint_core
      ../crates/gritlint_bench
      ../packs
      ../npm/gritlint/configuration_schema.json
      ../npm/gritlint/report_schema.json
      ../vendor
    ];
  };

  cargoDeps = craneLib.vendorCargoDeps {
    cargoLock = ../Cargo.lock;
    overrideVendorGitCheckout =
      packages: checkout:
      if lib.any (p: lib.hasPrefix "git+${engineRepo}?" p.source) packages then
        checkout.overrideAttrs (old: {
          preInstall = (old.preInstall or "") + ''
            rm -r resources/language-submodules
            find resources/language-metavariables -mindepth 3 -name Cargo.toml -delete
          '';
        })
      else
        checkout;
  };

  # The registry crates are linked as they are; only the engine's crates need
  # its resources beside them, so only their source directory is copied.
  vendor = runCommand "gritlint-vendor" { } ''
    mkdir -p $out/resources
    for source in ${cargoDeps}/*/; do
      source=''${source%/}
      if [ -n "$(find "$source/" -maxdepth 1 -name 'marzano-language-*')" ]; then
        cp -rL "$source" "$out/"
      else
        ln -s "$source" "$out/"
      fi
    done
    cp -r ${engineSource}/resources/node-types $out/resources/node-types
    substitute ${cargoDeps}/config.toml $out/config.toml --replace-fail ${cargoDeps} $out
  '';

  grammar = lib.fileset.toSource {
    root = ../vendor/tree-sitter-typescript;
    fileset = ../vendor/tree-sitter-typescript;
  };

  common = {
    pname = "gritlint";
    inherit version src;
    strictDeps = true;
    cargoVendorDir = vendor;
    extraDummyScript = ''
      rm -r $out/vendor/tree-sitter-typescript
      cp -r --no-preserve=mode ${grammar} $out/vendor/tree-sitter-typescript
    '';
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
      releaseArgs = common;
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
