# A consumer's install of the npm tarball, in the build sandbox with no
# network: `pnpm add --offline` from the tarball file, then gate-journey.sh
# against the installed `node_modules/.bin/gritlint`, then the launcher under a
# node that reports another platform. Both binaries' ELF headers are read here
# too, so a swapped or dynamic binary fails even on the system that cannot run
# it.
{
  lib,
  runCommand,
  nodejs_24,
  pnpm_12,
  binutils-unwrapped,
  # `packages.<system>.gritlint-tarball`: one `.tgz` file
  tarball,
}:

let
  # gate-journey.sh copies the repository's gritlint.json from beside it.
  journey = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../gritlint.json
      ../scripts/tools/gate-journey.sh
    ];
  };
in
runCommand "gritlint-tarball-journey"
  {
    nativeBuildInputs = [
      nodejs_24
      pnpm_12
      binutils-unwrapped
    ];
  }
  ''
    export HOME=$NIX_BUILD_TOP/home
    export pnpm_config_manage_package_manager_versions=false
    mkdir -p "$HOME" project
    cd project
    printf '{"name": "journey", "private": true}\n' > package.json
    pnpm add --offline --store-dir "$NIX_BUILD_TOP/store" file:${tarball}

    sh ${journey}/scripts/tools/gate-journey.sh "$PWD/node_modules/.bin/gritlint"

    launcher=node_modules/@systemfsoftware/gritlint/bin/gritlint.js
    for host in "platform darwin" "platform win32" "arch ia32" "arch riscv64"; do
      set -- $host
      printf 'Object.defineProperty(process, "%s", { value: "%s" })\n' "$1" "$2" > other.cjs
      status=0
      node --require ./other.cjs "$launcher" check > out 2> err || status=$?
      if [ "$status" -ne 2 ] || [ -s out ] || ! grep -qF 'this package ships linux-x64 and linux-arm64' err; then
        cat out err >&2
        echo "gritlint-tarball-journey: $1 $2: expected exit 2 naming both platforms, got $status" >&2
        exit 1
      fi
    done

    elf() {
      bin=node_modules/@systemfsoftware/gritlint/bin/$1
      readelf -h "$bin" | grep -qE "Machine: +$2\$" || {
        echo "gritlint-tarball-journey: $bin is not a $2 binary" >&2
        exit 1
      }
      if readelf -lW "$bin" | grep -q INTERP; then
        echo "gritlint-tarball-journey: $bin names a dynamic loader" >&2
        exit 1
      fi
    }
    elf gritlint-linux-x64 'Advanced Micro Devices X86-64'
    elf gritlint-linux-arm64 AArch64

    touch $out
  ''
