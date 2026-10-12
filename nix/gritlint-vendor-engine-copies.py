import filecmp
import json
import sys
import tomllib
from pathlib import Path

engine, vendor, lock_json = (Path(arg) for arg in sys.argv[1:4])
manifest = engine / "crates" / "language" / "Cargo.toml"
locked = {
    package["name"]: package["version"]
    for package in json.loads(lock_json.read_text())["package"]
    if package.get("source", "").startswith("git+")
}
sources = [source for source in vendor.iterdir() if source.is_dir()]

failures = []
checked = 0
for key, dependency in tomllib.loads(manifest.read_text())["dependencies"].items():
    if not isinstance(dependency, dict) or "path" not in dependency:
        continue
    name = dependency.get("package", key)
    if name not in locked:
        continue
    copy = manifest.parent / dependency["path"]
    crate = f"{name}-{locked[name]}"
    vendored = [source / crate for source in sources if (source / crate).is_dir()]
    if len(vendored) != 1:
        failures.append(f"{crate}: expected one vendored copy, found {len(vendored)}")
        continue
    files = [
        path.relative_to(vendored[0])
        for path in vendored[0].rglob("*")
        if path.is_file() and path.relative_to(vendored[0]).as_posix() not in ("Cargo.toml", ".cargo-checksum.json")
    ]
    if not files:
        failures.append(f"{crate}: the vendored copy has no files to compare")
    for file in files:
        if not (copy / file).is_file():
            failures.append(f"{crate}: {file} is not in {dependency['path']} in the engine at the pinned rev")
        elif not filecmp.cmp(vendored[0] / file, copy / file, shallow=False):
            failures.append(f"{crate}: {file} differs from {dependency['path']} in the engine at the pinned rev")
    checked += 1

if checked == 0:
    failures.append("no locked crate comes from marzano-language's path dependencies")
for failure in failures:
    print(failure, file=sys.stderr)
print(f"checked {checked} engine crates against marzano-language's path dependencies")
sys.exit(1 if failures else 0)
