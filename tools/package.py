import hashlib, json, sys, tomllib, zipfile
from pathlib import Path

root = Path(sys.argv[1]).resolve()
compat = json.loads((root / "compatibility.json").read_text())
bundle = next(p for p in (root / "mixins").iterdir() if (p / "horizon-mixin.toml").is_file())
manifest = tomllib.loads((bundle / "horizon-mixin.toml").read_text())
mod_id, version = manifest["mod"]["id"], manifest["mod"]["version"]
if manifest["compatibility"]["horizon_revision"] != compat["horizon_revision"]:
    sys.exit("horizon-mixin.toml and compatibility.json pin different Horizon commits")
files = {"horizon-native.json": (json.dumps({
    "schema": 1, "id": mod_id, "version": version, "game": compat["game"],
    "horizon": compat["horizon_revision"], "mixins": compat["horizon_mixins_revision"],
    "hooks": compat["hooks"]}, indent=2, sort_keys=True) + "\n").encode()}
for path in sorted(bundle.rglob("*")):
    rel = path.relative_to(bundle).as_posix()
    if path.is_file() and (path.suffix in (".rs", ".toml") or rel.startswith("data/")):
        files[f"mixins/{mod_id}/{rel}"] = path.read_bytes()
files[f"mixins/{mod_id}/LICENSE"] = (root / "LICENSE").read_bytes()
out = root / "dist" / f"{mod_id}-{version}.hnmod"
out.parent.mkdir(exist_ok=True)
with zipfile.ZipFile(out, "w") as archive:
    for name, content in sorted(files.items()):
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o100644 << 16
        archive.writestr(info, content)
print(json.dumps({"file": str(out), "size": out.stat().st_size,
                  "sha256": hashlib.sha256(out.read_bytes()).hexdigest(), "files": sorted(files)}, indent=2))
