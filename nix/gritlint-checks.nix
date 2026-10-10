# The Rust gate as flake checks. Each one fails the build on what the
# monorepo's `scripts/tools/gate-rust.sh` failed on, except `cargo deny check`,
# whose advisories need the network and so run as a CI step.
{
  lib,
  runCommand,
  craneLib,
  craneLibMsrv,
  gritlint,
  # `packages.<system>.gritlint`: the journey reads what a consumer runs.
  sandboxed,
}:

let
  inherit (gritlint.passthru) devArgs devArtifacts;

  unwrapped = "${gritlint}/bin/gritlint";

  # `cargo check` with exactly the `rust-version` toolchain, over its own
  # dependency artifacts so a source edit recompiles only the workspace.
  msrvArgs = devArgs // {
    buildPhaseCargoCommand = "cargoWithProfile check --locked --workspace --all-targets";
  };

  journey = craneLib.buildPackage (
    devArgs
    // {
      pname = "gritlint-e2e";
      cargoArtifacts = devArtifacts;
      cargoExtraArgs = "--locked -p gritlint-e2e";
      doCheck = false;
    }
  );
in
{
  gritlint-fmt = craneLib.cargoFmt {
    inherit (devArgs) pname version src;
    doInstallCargoArtifacts = false;
  };

  gritlint-clippy = craneLib.cargoClippy (
    devArgs
    // {
      cargoArtifacts = devArtifacts;
      cargoClippyExtraArgs = "--workspace --all-targets -- -D warnings";
      doInstallCargoArtifacts = false;
    }
  );

  gritlint-nextest = craneLib.cargoNextest (
    devArgs
    // {
      cargoArtifacts = devArtifacts;
      cargoNextestExtraArgs = "--workspace";
      doInstallCargoArtifacts = false;
    }
  );

  gritlint-packs = runCommand "gritlint-packs" { } ''
    cp -r ${../packs} packs
    chmod -R u+w packs
    ${unwrapped} test packs
    touch $out
  '';

  gritlint-agent-journey = runCommand "gritlint-agent-journey" { } ''
    mkdir journey
    ${journey}/bin/gritlint-e2e ${sandboxed}/bin/gritlint "$PWD/journey"
    touch $out
  '';

  gritlint-schema = runCommand "gritlint-schema" { } ''
    ${unwrapped} schema > schema.json
    if ! diff -u ${../npm/gritlint/configuration_schema.json} schema.json; then
      echo "npm/gritlint/configuration_schema.json is stale: write \`gritlint schema\` over it" >&2
      exit 1
    fi
    ${unwrapped} schema --report > report.json
    if ! diff -u ${../npm/gritlint/report_schema.json} report.json; then
      echo "npm/gritlint/report_schema.json is stale: write \`gritlint schema --report\` over it" >&2
      exit 1
    fi
    touch $out
  '';

  gritlint-vendor-sync =
    let
      engine = "${gritlint.passthru.engineSource}/resources/language-metavariables/tree-sitter-typescript";
      vendored = ../vendor/tree-sitter-typescript;
      crate = lib.importTOML ../vendor/tree-sitter-typescript/Cargo.toml;
      roots = [
        "Cargo.toml"
        "LICENSE"
      ]
      ++ crate.package.include;
    in
    runCommand "gritlint-vendor-sync" { } ''
      { (cd ${engine} && find ${lib.escapeShellArgs roots} -type f); (cd ${vendored} && find . -type f | sed 's|^\./||'); } |
        sort -u | grep -vxE 'README\.md|common/scanner\.h' > files
      stale=0
      while read -r f; do
        if ! cmp -s "${engine}/$f" "${vendored}/$f"; then
          echo "stale: vendor/tree-sitter-typescript/$f" >&2
          stale=1
        fi
      done < files
      if [ "$stale" -ne 0 ]; then
        echo "vendor/tree-sitter-typescript differs from the engine's grammar at the pinned rev: copy that rev's files over it and re-apply the common/scanner.h change (its README)" >&2
        exit 1
      fi
      touch $out
    '';

  gritlint-decide-loops = runCommand "gritlint-decide-loops" { } ''
    if grep -nE '^[[:space:]]*(for|while|loop)[[:space:](]' ${../crates/gritlint_core/src/decide.rs}; then
      echo "a decide module contains an imperative loop; use iterator combinators" >&2
      exit 1
    fi
    touch $out
  '';

  gritlint-engine-features = craneLib.mkCargoDerivation (
    devArgs
    // {
      pnameSuffix = "-engine-features";
      cargoArtifacts = null;
      doInstallCargoArtifacts = false;
      buildPhaseCargoCommand = ''
        cargo tree --locked --offline -e features --workspace --prefix none > features.txt
        if grep -E 'marzano-(core|util) feature "(ai_builtins|embeddings|external_functions|grit_tracing|network_requests)"' features.txt; then
          echo "a network-bearing engine feature is enabled" >&2
          exit 1
        fi
      '';
      installPhaseCommand = "touch $out";
    }
  );
}
// lib.optionalAttrs (craneLibMsrv != null) {
  gritlint-msrv = craneLibMsrv.mkCargoDerivation (
    msrvArgs
    // {
      pnameSuffix = "-msrv";
      cargoArtifacts = craneLibMsrv.buildDepsOnly (msrvArgs // { doCheck = false; });
      doInstallCargoArtifacts = false;
      installPhaseCommand = "touch $out";
    }
  );
}
