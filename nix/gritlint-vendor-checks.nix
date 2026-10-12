# Two laws of the cargo vendor in `gritlint.nix`, kept as functions so that a
# fixture vendor can be checked by the same code.
{
  lib,
  runCommand,
  python3,
}:

{
  # A version PR rewrites only the workspace members' Cargo.lock entries, the
  # ones with no `source`. The vendor must not move with them: a vendor that did
  # (fetchCargoVendor's fixed output held the whole lock) broke main on #18.
  ignoresWorkspaceVersions =
    { vendorFor, lock }:
    let
      members = builtins.filter (p: !(p ? source)) lock.package;
      moved = lock // {
        package = map (p: if p ? source then p else p // { version = "${p.version}-moved"; }) lock.package;
      };
    in
    assert lib.assertMsg (members != [ ]) "gritlint-vendor-ignores-workspace-versions: the lock has no workspace members to move";
    assert lib.assertMsg ((vendorFor moved).drvPath == (vendorFor lock).drvPath)
      "gritlint-vendor-ignores-workspace-versions: the vendor's derivation changes with the workspace members' versions, so the next version PR breaks it";
    runCommand "gritlint-vendor-ignores-workspace-versions" { } "touch $out";

  # Every locked git crate is vendored exactly once. For each one that the
  # engine's marzano-language manifest takes by path, every vendored file equals
  # the file at that path at the pinned rev, and every file its `include` names
  # there is vendored. The checkout also holds upstream grammars under the same
  # names and versions, and cargo drops a directory with its own manifest;
  # either mistake still builds.
  engineCopies =
    {
      vendor,
      engineSource,
      lock,
    }:
    runCommand "gritlint-vendor-engine-copies" { nativeBuildInputs = [ python3 ]; } ''
      python3 ${./gritlint-vendor-engine-copies.py} ${engineSource} ${vendor} ${builtins.toFile "Cargo.lock.json" (builtins.toJSON lock)}
      touch $out
    '';
}
