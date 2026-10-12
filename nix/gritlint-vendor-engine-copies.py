import filecmp
import fnmatch
import json
import sys
import tomllib
from pathlib import Path

engine, vendor, lock_json = (Path(arg) for arg in sys.argv[1:4])
manifest = engine / "crates" / "language" / "Cargo.toml"
locked = {
    (package["name"], package["version"])
    for package in json.loads(lock_json.read_text())["package"]
    if package.get("source", "").startswith("git+")
}
workspace_version = tomllib.loads((engine / "Cargo.toml").read_text())["workspace"]["package"]["version"]
not_compared = {Path("Cargo.toml"), Path(".cargo-checksum.json")}
sources = [source for source in vendor.iterdir() if source.is_dir()]


def vendored_copies(crate):
    return [source / crate for source in sources if (source / crate).is_dir()]


def files_under(root):
    return {path.relative_to(root) for path in root.rglob("*") if path.is_file()}


def included(file, patterns):
    for pattern in (pattern.strip("/") for pattern in patterns):
        if "/" in pattern:
            prefixes = ("/".join(file.parts[: depth + 1]) for depth in range(len(file.parts)))
            if any(fnmatch.fnmatchcase(prefix, pattern) for prefix in prefixes):
                return True
        elif any(fnmatch.fnmatchcase(part, pattern) for part in file.parts):
            return True
    return False


failures = [
    f"{name}-{version}: expected one vendored copy, found {len(vendored_copies(f'{name}-{version}'))}"
    for name, version in sorted(locked)
    if len(vendored_copies(f"{name}-{version}")) != 1
]
checked = 0
for key, dependency in tomllib.loads(manifest.read_text())["dependencies"].items():
    if not isinstance(dependency, dict) or "path" not in dependency:
        continue
    name = dependency.get("package", key)
    copy = manifest.parent / dependency["path"]
    package = tomllib.loads((copy / "Cargo.toml").read_text())["package"]
    version = package["version"] if isinstance(package["version"], str) else workspace_version
    if (name, version) not in locked:
        if any(locked_name == name for locked_name, _ in locked):
            failures.append(f"{name}: {dependency['path']} declares {version}, which is not the version Cargo.lock locks")
        continue
    crate = f"{name}-{version}"
    vendored = vendored_copies(crate)
    if len(vendored) != 1:
        continue
    files = files_under(vendored[0]) - not_compared
    if not files:
        failures.append(f"{crate}: the vendored copy has no files to compare")
    for file in sorted(files):
        if not (copy / file).is_file():
            failures.append(f"{crate}: {file} is not in {dependency['path']} in the engine at the pinned rev")
        elif not filecmp.cmp(vendored[0] / file, copy / file, shallow=False):
            failures.append(f"{crate}: {file} differs from {dependency['path']} in the engine at the pinned rev")
    patterns = package.get("include")
    if patterns:
        for file in sorted(files_under(copy) - files - not_compared):
            if included(file, patterns):
                failures.append(f"{crate}: {file} from {dependency['path']} is in its include list but not vendored")
    checked += 1

if checked == 0:
    failures.append("no locked crate comes from marzano-language's path dependencies")
for failure in failures:
    print(failure, file=sys.stderr)
print(f"checked {checked} engine crates against marzano-language's path dependencies")
sys.exit(1 if failures else 0)
