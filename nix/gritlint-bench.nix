# Times PR1's gritlint against this build on a pinned public corpus and fails
# when the two reports, or the current build's single-threaded report, differ.
{
  lib,
  runCommand,
  fetchFromGitHub,
  craneLib,
  gritlint,
  # Built first, so the timings run on a builder the gate has finished with.
  after,
}:

let
  baselineRev = "80e983b629cc939a38acad0c52d1bf4a50b1bd60";
  baseline =
    (builtins.getFlake "github:systemfsoftware/gritlint/${baselineRev}")
    .packages.x86_64-linux.gritlint-unwrapped;

  corpusRev = "0b17b9e2e43c2f95726470aee188fd9e2bf1baf6";
  corpus = fetchFromGitHub {
    owner = "Effect-TS";
    repo = "effect";
    rev = corpusRev;
    hash = "sha256-Cfm8ZP7F3lE+3LmO4+IuJBcUTPX+qXVyRIX4tU04bxM=";
  };

  bench = craneLib.buildPackage (
    gritlint.passthru.devArgs
    // {
      pname = "gritlint-bench";
      cargoArtifacts = null;
      cargoExtraArgs = "--locked -p gritlint_bench";
      doCheck = false;
    }
  );
in
runCommand "gritlint-bench" { inherit after; } ''
  mkdir $out
  {
    echo "### gritlint benchmark"
    echo
    echo "Corpus [Effect-TS/effect@${lib.substring 0 8 corpusRev}](https://github.com/Effect-TS/effect/tree/${corpusRev}); baseline PR1 \`${lib.substring 0 8 baselineRev}\`."
    echo
    ${bench}/bin/gritlint_bench \
      --baseline ${baseline}/bin/gritlint \
      --candidate ${gritlint}/bin/gritlint \
      --corpus ${corpus} \
      --config ${../crates/gritlint_bench/effect.gritlint.json} \
      --rounds 15
  } > $out/summary.md
''
