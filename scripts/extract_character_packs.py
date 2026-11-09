#!/usr/bin/env python3
"""Estrae i pacchetti CC0 di Quaternius da assets_src/zips/ in assets_src/<nome>/ (ADR 0015).

Tiene solo i tipi di file che servono alla pipeline (.gltf .glb .bin .png .txt): i .blend e gli
altri eseguibili/progetti non vengono estratti. Ogni zip finisce in una cartella propria, e i
glTF vengono controllati: gli URI esterni devono restare dentro la cartella del pacchetto.

Uso: python3 -I scripts/extract_character_packs.py <cartella assets_src>
"""
import json
import sys
import zipfile
from pathlib import Path, PurePosixPath

KEEP = {".gltf", ".glb", ".bin", ".png", ".txt"}
PACKS = {
    "UAL_Standard.zip": "ual",
    "BaseCharacters_Standard.zip": "base",
    "Outfits_Standard.zip": "outfits",
}


def safe_uri(uri: str) -> bool:
    """Un URI esterno è ammesso solo se relativo e senza risalite."""
    if uri.startswith("data:"):
        return True
    p = PurePosixPath(uri.replace("%20", " "))
    return "://" not in uri and not p.is_absolute() and ".." not in p.parts


def main() -> None:
    root = Path(sys.argv[1]).resolve()
    for zip_name, folder in PACKS.items():
        src = root / "zips" / zip_name
        if not src.exists():
            print(f"manca {src}")
            continue
        dest = root / folder
        dest.mkdir(exist_ok=True)
        kept = skipped = 0
        with zipfile.ZipFile(src) as z:
            for info in z.infolist():
                if info.is_dir():
                    continue
                name = PurePosixPath(info.filename)
                if name.is_absolute() or ".." in name.parts or name.suffix.lower() not in KEEP:
                    skipped += 1
                    continue
                target = dest.joinpath(*name.parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(z.read(info))
                kept += 1
        bad = 0
        for gltf in dest.rglob("*.gltf"):
            data = json.loads(gltf.read_text(encoding="utf-8"))
            for item in data.get("buffers", []) + data.get("images", []):
                uri = item.get("uri")
                if uri and not safe_uri(uri):
                    print(f"URI RIFIUTATO in {gltf}: {uri}")
                    bad += 1
        print(f"{zip_name}: {kept} file estratti, {skipped} ignorati, {bad} URI rifiutati")
        if bad:
            sys.exit(1)


if __name__ == "__main__":
    main()
