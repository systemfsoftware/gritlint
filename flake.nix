{
  description = "gritlint — GritQL rule packs over the config files of a repository";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane/v0.24.0";
  };

  outputs = { self, nixpkgs, rust-overlay, crane }:
    let
      lib = nixpkgs.lib;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forEachSystem = fn:
        lib.genAttrs (systems ++ [ "x86_64-windows" ]) (system:
          if builtins.elem system systems
          then fn (import nixpkgs { inherit system; overlays = [ (import rust-overlay) ]; })
          else throw "gritlint's flake builds on ${lib.concatStringsSep ", " systems}; ${system} is not one of them");

      # The Rust toolchain is rust-toolchain.toml, so a flake build and
      # `nix develop` compile with what CI compiles with.
      rust = pkgs: pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneFor = pkgs: (crane.mkLib pkgs).overrideToolchain rust;

      workspacePackage = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package;
      gritlintVersion = workspacePackage.version;

      # `rust-version = "1.90"` names the 1.90.0 toolchain.
      msrv =
        let parts = lib.splitString "." workspacePackage.rust-version;
        in lib.concatStringsSep "." (parts ++ lib.optional (builtins.length parts == 2) "0");

      gritlintFor = pkgs:
        let
          gritlint-unwrapped = pkgs.callPackage ./nix/gritlint.nix {
            craneLib = craneFor pkgs;
            version = gritlintVersion;
          };
        in
        {
          inherit gritlint-unwrapped;
          gritlint = pkgs.callPackage ./nix/gritlint-sandbox.nix { gritlint = gritlint-unwrapped; };
        };
    in
    {
      packages = forEachSystem gritlintFor;

      checks = forEachSystem (pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          exported = self.packages.${system};
          # Stands in for the CLI inside the exported wrapper and exits 1 naming
          # every isolation the wrapper failed to give it.
          escapeProbe = pkgs.writeShellScriptBin "gritlint" ''
            escaped=0
            if [ -e "$1" ]; then echo "read $1 outside the working directory"; escaped=1; fi
            if { : > written; } 2>/dev/null; then echo "wrote the working directory"; escaped=1; fi
            if [ -n "''${GRITLINT_PROBE_ENV-}" ]; then echo "kept the caller's environment"; escaped=1; fi
            exit "$escaped"
          '';
          gate = {
            # `nix flake check` evaluates packages without building them; a check builds it.
            gritlint = exported.gritlint;
            # Consumers take `packages.<system>.gritlint` as the sandboxed CLI. The
            # exported wrapper runs the probe from a working directory next to a
            # file it must not see; rebinding the name to an unsandboxed build
            # fails the `override`, and loosening the sandbox fails the probe.
            gritlint-is-sandboxed = pkgs.runCommand "gritlint-is-sandboxed" { } ''
              touch outside && mkdir work && cd work
              if GRITLINT_PROBE_ENV=1 ${escapeProbe}/bin/gritlint "$NIX_BUILD_TOP/outside"; then
                echo "the probe saw no escape without a sandbox, so it proves nothing" >&2
                exit 1
              fi
              rm -f written
              GRITLINT_PROBE_ENV=1 ${exported.gritlint.override { gritlint = escapeProbe; }}/bin/gritlint "$NIX_BUILD_TOP/outside"
              touch $out
            '';
            # `packages.<system>.gritlint-unwrapped` is the compiled CLI itself, not a wrapper.
            gritlint-unwrapped-is-plain = pkgs.runCommand "gritlint-unwrapped-is-plain" { } ''
              bin=${exported.gritlint-unwrapped}/bin/gritlint
              if [ "$(head -c4 "$bin")" != $'\x7fELF' ]; then
                echo "packages.${system}.gritlint-unwrapped: bin/gritlint is not the compiled binary" >&2
                exit 1
              fi
              [ "$("$bin" --version)" = "gritlint ${gritlintVersion}" ]
              touch $out
            '';
          }
          // import ./nix/gritlint-checks.nix {
            inherit (pkgs) lib runCommand;
            craneLib = craneFor pkgs;
            # Language support does not vary by architecture, so one system checks the MSRV.
            craneLibMsrv =
              if system == "x86_64-linux"
              then (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.${msrv}.minimal)
              else null;
            gritlint = exported.gritlint-unwrapped;
          };
        in
        gate
        // lib.optionalAttrs (system == "x86_64-linux") {
          gritlint-bench = pkgs.callPackage ./nix/gritlint-bench.nix {
            craneLib = craneFor pkgs;
            gritlint = exported.gritlint-unwrapped;
            after = lib.attrValues gate;
          };
        });

      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (rust pkgs)
            pkgs.cargo-deny
            pkgs.cargo-nextest
          ];
        };
      });
    };
}
