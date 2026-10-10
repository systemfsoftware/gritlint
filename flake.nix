{
  description = "gritlint — GritQL rule packs over the config files of a repository";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
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

      gritlintVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

      gritlintFor = pkgs:
        let
          toolchain = rust pkgs;
          gritlint-unwrapped = pkgs.callPackage ./nix/gritlint.nix {
            rustPlatform = pkgs.makeRustPlatform { cargo = toolchain; rustc = toolchain; };
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
          sandboxed = pkgs.callPackage ./nix/gritlint-sandbox.nix { gritlint = exported.gritlint-unwrapped; };
        in
        {
          # `nix flake check` evaluates packages without building them; a check builds it.
          gritlint = exported.gritlint;
          # Consumers take `packages.<system>.gritlint` as the sandboxed CLI, so
          # no later edit may bind that name to anything else.
          gritlint-is-sandboxed =
            if exported.gritlint.drvPath == sandboxed.drvPath
            then pkgs.runCommand "gritlint-is-sandboxed" { } "touch $out"
            else throw "flake.nix: packages.${system}.gritlint is not nix/gritlint-sandbox.nix over packages.${system}.gritlint-unwrapped";
        });

      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          packages = [
            (rust pkgs)
            pkgs.cargo-deny
          ];
        };
      });
    };
}
