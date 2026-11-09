#!/usr/bin/env python3
"""Compone i personaggi del gioco dai pacchetti CC0 di Quaternius (ADR 0015).

Prima: `python3 -I scripts/extract_character_packs.py <assets_src>`.
Uso:   `python3 -I scripts/build_characters.py <assets_src> <cartella assets/models/characters>`

I pacchetti condividono lo stesso scheletro da 65 ossa, quindi non serve Blender: ogni personaggio è
il glTF di un abito a cui si aggiungono testa, capelli e barba con la propria `skin` (le matrici di bind
restano quelle del file di origine: testa e capelli seguono le ossa dell'abito). Scrive:
  - `<modello>.gltf` + `.bin` per ogni aspetto, con texture condivise in `textures/`;
  - `animations.gltf` + `.bin` con le sole clip usate (solo rotazioni, più la traslazione del bacino,
    così le animazioni non deformano le proporzioni dei modelli).
"""
import copy
import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

OUT_PREFIX = "Modular Character Outfits - Fantasy[Standard]/Exports/glTF (Godot-Unreal)/Outfits"
BASE_PREFIX = "Universal Base Characters[Standard]/Base Characters/Godot - UE"
HAIR_PREFIX = "Universal Base Characters[Standard]/Hairstyles/Rigged to Head Bone/glTF (Godot -Unreal)"
UAL_FILE = "Universal Animation Library[Standard]/Unreal-Godot/UAL1_Standard.glb"

# Ordine delle clip in animations.gltf: deve restare uguale a `figure::CLIPS`.
CLIPS = [
    "Idle_Loop", "Walk_Loop", "Jog_Fwd_Loop", "Sprint_Loop", "Jump_Start", "Jump_Loop", "Jump_Land",
    "Sword_Idle", "Sword_Attack", "Punch_Jab", "Hit_Chest", "Death01", "Idle_Talking_Loop", "Interact",
    "Spell_Simple_Shoot",
]
TEX_SIZE = 1024
TINT = {  # tinta del tessuto chiaro degli abiti da contadino
    "ocra": (0.92, 0.72, 0.38), "rosso": (0.78, 0.30, 0.26), "verde": (0.42, 0.60, 0.36),
    "viola": (0.55, 0.42, 0.72), "rosa": (0.90, 0.56, 0.60), "nero": (0.16, 0.16, 0.20),
}
HAIR = {"castano": (0.26, 0.16, 0.09), "nero": (0.07, 0.06, 0.06), "biondo": (0.72, 0.56, 0.30), "grigio": (0.55, 0.55, 0.55)}

# modello: (sesso, abito, variante texture dell'abito, parti tolte, capelli, colore capelli)
MODELS = {
    "giocatore": ("Male", "Ranger", "ranger", ["Head_Hood"], ["Hair_SimpleParted"], "castano"),
    "viandante": ("Male", "Peasant", "peasant_2", [], ["Hair_Long", "Hair_Beard"], "grigio"),
    "ladro": ("Male", "Ranger", "ranger_3", [], ["Hair_Buzzed"], "nero"),
    "guardia": ("Male", "Ranger", "ranger_blu", ["Acc_Pauldron"], ["Hair_Buzzed"], "castano"),
    "prete": ("Male", "Peasant", "peasant_nero", [], ["Hair_Buzzed"], "grigio"),
    "civile-m-ocra": ("Male", "Peasant", "peasant_ocra", [], ["Hair_SimpleParted", "Hair_Beard"], "castano"),
    "civile-m-rosso": ("Male", "Peasant", "peasant_rosso", [], ["Hair_Buzzed", "Hair_Beard"], "nero"),
    "civile-m-verde": ("Male", "Peasant", "peasant_verde", [], ["Hair_SimpleParted"], "biondo"),
    "arciera": ("Female", "Ranger", "ranger_3", ["Head_Hood"], ["Hair_Long"], "castano"),
    "civile-f-viola": ("Female", "Peasant", "peasant_viola", [], ["Hair_Buns"], "grigio"),
    "civile-f-rosa": ("Female", "Peasant", "peasant_rosa", [], ["Hair_Long"], "castano"),
}
COMPONENT = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
NUM = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
DTYPE = {5120: "b", 5121: "B", 5122: "<h", 5123: "<H", 5125: "<I", 5126: "<f"}


class Gltf:
    def __init__(self, path: Path):
        self.path = path
        if path.suffix == ".glb":
            with open(path, "rb") as f:
                f.read(12)
                length, _ = struct.unpack("<I4s", f.read(8))
                self.j = json.loads(f.read(length))
                length, _ = struct.unpack("<I4s", f.read(8))
                self.blob = f.read(length)
        else:
            self.j = json.loads(path.read_text(encoding="utf-8"))
            self.blob = (path.parent / self.j["buffers"][0]["uri"]).read_bytes()

    def view(self, bv_index: int) -> bytes:
        bv = self.j["bufferViews"][bv_index]
        o = bv.get("byteOffset", 0)
        return self.blob[o : o + bv["byteLength"]]

    def elem_size(self, acc: dict) -> int:
        return COMPONENT[acc["componentType"]] * NUM[acc["type"]]

    def array(self, acc_index: int) -> np.ndarray:
        """Accessor come array numpy (n, componenti)."""
        acc = self.j["accessors"][acc_index]
        bv = self.j["bufferViews"][acc["bufferView"]]
        n = NUM[acc["type"]]
        csize = COMPONENT[acc["componentType"]]
        stride = bv.get("byteStride", csize * n)
        start = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
        dt = np.dtype(DTYPE[acc["componentType"]])
        if stride == csize * n:
            data = np.frombuffer(self.blob, dtype=dt, count=acc["count"] * n, offset=start)
            return data.reshape(acc["count"], n).copy()
        out = np.empty((acc["count"], n), dtype=dt)
        for i in range(acc["count"]):
            out[i] = np.frombuffer(self.blob, dtype=dt, count=n, offset=start + i * stride)
        return out


class Writer:
    """Accumula buffer view e accessor nel glTF di destinazione."""

    def __init__(self, j: dict, blob: bytes):
        self.j = j
        self.blob = bytearray(blob)
        self.memo: dict = {}

    def _append(self, data: bytes, target=None) -> int:
        while len(self.blob) % 4:
            self.blob.append(0)
        view = {"buffer": 0, "byteOffset": len(self.blob), "byteLength": len(data)}
        if target is not None:
            view["target"] = target
        self.blob.extend(data)
        self.j["bufferViews"].append(view)
        return len(self.j["bufferViews"]) - 1

    def add_array(self, data: bytes, ctype: int, kind: str, count: int, target=None, extra=None) -> int:
        acc = {"bufferView": self._append(data, target), "componentType": ctype, "count": count, "type": kind}
        if extra:
            acc.update(extra)
        self.j["accessors"].append(acc)
        return len(self.j["accessors"]) - 1

    def copy_accessor(self, src: Gltf, index: int) -> int:
        key = (str(src.path), "acc", index)
        if key in self.memo:
            return self.memo[key]
        acc = copy.deepcopy(src.j["accessors"][index])
        arr = src.array(index)
        acc["bufferView"] = self._append(arr.tobytes(), src.j["bufferViews"][acc["bufferView"]].get("target"))
        acc.pop("byteOffset", None)
        self.j["accessors"].append(acc)
        self.memo[key] = len(self.j["accessors"]) - 1
        return self.memo[key]

    def texture(self, src: Gltf, tex: int, tex_map) -> int:
        key = (str(src.path), "tex", tex)
        if key in self.memo:
            return self.memo[key]
        t = src.j["textures"][tex]
        img = src.j["images"][t["source"]]
        uri = tex_map(src, img["uri"])
        imgs = self.j.setdefault("images", [])
        idx = next((i for i, m in enumerate(imgs) if m.get("uri") == uri), None)
        if idx is None:
            imgs.append({"uri": uri, "mimeType": "image/png", "name": uri})
            idx = len(imgs) - 1
        new = {"source": idx}
        if "sampler" in t:
            sampler = src.j["samplers"][t["sampler"]]
            samplers = self.j.setdefault("samplers", [])
            if sampler in samplers:
                new["sampler"] = samplers.index(sampler)
            else:
                samplers.append(sampler)
                new["sampler"] = len(samplers) - 1
        self.j.setdefault("textures", []).append(new)
        self.memo[key] = len(self.j["textures"]) - 1
        return self.memo[key]

    def material(self, src: Gltf, mi: int, tex_map, factor=None) -> int:
        key = (str(src.path), "mat", mi)
        if key in self.memo:
            return self.memo[key]
        m = copy.deepcopy(src.j["materials"][mi])
        pbr = m.get("pbrMetallicRoughness", {})
        for owner, field in [(pbr, "baseColorTexture"), (pbr, "metallicRoughnessTexture"), (m, "normalTexture"), (m, "occlusionTexture"), (m, "emissiveTexture")]:
            if field in owner:
                owner[field]["index"] = self.texture(src, owner[field]["index"], tex_map)
        if factor is not None:
            pbr["baseColorFactor"] = [factor[0], factor[1], factor[2], 1.0]
            m["pbrMetallicRoughness"] = pbr
        self.j["materials"].append(m)
        self.memo[key] = len(self.j["materials"]) - 1
        return self.memo[key]


def dominant_joints(src: Gltf, prim: dict) -> np.ndarray:
    joints = src.array(prim["attributes"]["JOINTS_0"])
    weights = src.array(prim["attributes"]["WEIGHTS_0"]).astype(float)
    return joints[np.arange(len(joints)), weights.argmax(axis=1)]


def kept_attributes(prim: dict) -> dict:
    """Solo gli attributi usati dal gioco (i pacchetti ne hanno altri, ignorati da Bevy)."""
    used = ("POSITION", "NORMAL", "TANGENT", "TEXCOORD_0", "JOINTS_0", "WEIGHTS_0")
    return {a: i for a, i in prim["attributes"].items() if a in used}


def add_part(w: Writer, src: Gltf, armature: int, name_to_node: dict, tex_map, *, head_only=False, factor=None):
    """Aggiunge a `w` le mesh con skin di `src`; `head_only` tiene solo testa e collo del corpo."""
    j = src.j
    for node in j["nodes"]:
        if "mesh" not in node or "skin" not in node:
            continue
        skin = j["skins"][node["skin"]]
        names = [j["nodes"][i]["name"] for i in skin["joints"]]
        new_skin = {
            "joints": [name_to_node[n] for n in names],
            "inverseBindMatrices": w.copy_accessor(src, skin["inverseBindMatrices"]),
            "skeleton": name_to_node["root"],
        }
        w.j["skins"].append(new_skin)
        skin_idx = len(w.j["skins"]) - 1
        prims = []
        for prim in j["meshes"][node["mesh"]]["primitives"]:
            mat_name = j["materials"][prim["material"]]["name"]
            is_body = head_only and mat_name.startswith("MI_Superhero")
            is_hair = mat_name.startswith("MI_Hair")
            mat = w.material(src, prim["material"], tex_map, factor if is_hair else None)
            if is_body:
                dom = dominant_joints(src, prim)
                keep_joint = [names.index("Head"), names.index("neck_01")]
                idx = src.array(prim["indices"]).astype(np.uint32).reshape(-1, 3)
                mask = np.isin(dom[idx], keep_joint).all(axis=1)
                idx = idx[mask]
                used, remap = np.unique(idx, return_inverse=True)
                new_idx = remap.reshape(-1, 3).astype(np.uint32)
                attrs = {}
                for attr, acc_i in kept_attributes(prim).items():
                    arr = src.array(acc_i)[used]
                    acc = j["accessors"][acc_i]
                    extra = {"normalized": True} if acc.get("normalized") else {}
                    if attr == "POSITION":
                        extra = {"min": [float(v) for v in arr.min(axis=0)], "max": [float(v) for v in arr.max(axis=0)]}
                    attrs[attr] = w.add_array(arr.tobytes(), acc["componentType"], acc["type"], len(arr), 34962, extra)
                indices = w.add_array(new_idx.tobytes(), 5125, "SCALAR", new_idx.size, 34963)
                prims.append({"attributes": attrs, "indices": indices, "material": mat, "mode": 4})
            else:
                attrs = {a: w.copy_accessor(src, i) for a, i in kept_attributes(prim).items()}
                prims.append({"attributes": attrs, "indices": w.copy_accessor(src, prim["indices"]), "material": mat, "mode": 4})
        w.j["meshes"].append({"name": node["name"], "primitives": prims})
        w.j["nodes"].append({"name": "part_" + node["name"], "mesh": len(w.j["meshes"]) - 1, "skin": skin_idx})
        w.j["nodes"][armature].setdefault("children", []).append(len(w.j["nodes"]) - 1)


# ---- texture ----------------------------------------------------------------------------------

def tint_cloth(img: Image.Image, tint) -> Image.Image:
    """Tinge il tessuto chiaro e lascia cuoio e parti scure come sono."""
    a = np.asarray(img.convert("RGB")).astype(float) / 255.0
    lum = a.mean(axis=2)
    mask = np.clip((lum - 0.30) / 0.28, 0.0, 1.0)[..., None]
    out = a * (1.0 - mask) + lum[..., None] * np.array(tint) * 1.25 * mask
    return Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8))


def darken_all(img: Image.Image, tint) -> Image.Image:
    a = np.asarray(img.convert("RGB")).astype(float) / 255.0
    lum = a.mean(axis=2)[..., None]
    return Image.fromarray((np.clip(lum * np.array(tint) * 2.2, 0, 1) * 255).astype(np.uint8))


def green_to_blue(img: Image.Image) -> Image.Image:
    """Le parti verdi del ranger diventano blu acciaio (divisa delle guardie)."""
    a = np.asarray(img.convert("RGB")).astype(float) / 255.0
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    mask = np.clip((g - np.maximum(r, b)) / 0.10, 0.0, 1.0)[..., None]
    lum = a.mean(axis=2)
    blue = np.stack([lum * 0.55, lum * 0.72, lum * 1.15], axis=2)
    return Image.fromarray((np.clip(a * (1.0 - mask) + blue * mask, 0, 1) * 255).astype(np.uint8))


def prepare_textures(src_root: Path, tex_dir: Path) -> dict:
    tex_dir.mkdir(parents=True, exist_ok=True)
    textures = next(src_root.joinpath("outfits").glob("*/Textures"))
    base = lambda sub, name: Image.open(textures / sub / name).convert("RGB").resize((TEX_SIZE, TEX_SIZE), Image.LANCZOS)
    variants = {
        "peasant": base("Peasant", "T_Peasant_BaseColor.png"),
        "peasant_2": base("Peasant", "T_Peasant_2_BaseColor.png"),
        "ranger": base("Ranger", "T_Ranger_BaseColor.png"),
        "ranger_3": base("Ranger", "T_Ranger_3_BaseColor.png"),
    }
    for key in ("ocra", "rosso", "verde", "viola", "rosa"):
        variants["peasant_" + key] = tint_cloth(variants["peasant"], TINT[key])
    variants["peasant_nero"] = darken_all(variants["peasant"], TINT["nero"])
    variants["ranger_blu"] = green_to_blue(variants["ranger"])
    for name, img in variants.items():
        img.save(tex_dir / f"abito_{name}.png", optimize=True)
    return variants


def make_tex_map(variant: str, tex_dir: Path):
    def mapper(src: Gltf, uri: str) -> str:
        name = uri.replace("%20", " ").split("/")[-1].replace("_png.png", ".png")
        low = name.lower()
        if "peasant" in low or "ranger" in low:
            if "basecolor" in low:
                return f"textures/abito_{variant}.png"
        target = tex_dir / name
        if not target.exists():
            source = src.path.parent / name
            size = 512 if "eye" in low else TEX_SIZE
            mode = "RGB"
            Image.open(source).convert(mode).resize((size, size), Image.LANCZOS).save(target, optimize=True)
        return f"textures/{name}"

    return mapper


# ---- modelli ----------------------------------------------------------------------------------

def build_model(name: str, spec, src_root: Path, out_dir: Path) -> None:
    sex, outfit, variant, drop, hair, hair_col = spec
    outfit_path = next(src_root.joinpath("outfits").glob("*/Exports/glTF*/Outfits")) / f"{sex}_{outfit}.gltf"
    src = Gltf(outfit_path)
    j = copy.deepcopy(src.j)
    tex_dir = out_dir / "textures"
    tex_map = make_tex_map(variant, tex_dir)
    # Abito: si riparte da zero e si ricopiano le sue mesh con la stessa routine (texture rimappate).
    node_names = {n.get("name"): i for i, n in enumerate(j["nodes"])}
    armature = node_names["Armature"]
    mesh_nodes = [i for i, n in enumerate(j["nodes"]) if "mesh" in n]
    j["nodes"][armature]["children"] = [c for c in j["nodes"][armature]["children"] if c not in mesh_nodes]
    for i in mesh_nodes:
        for key in ("mesh", "skin"):
            j["nodes"][i].pop(key, None)
    keep_nodes = len(j["nodes"])
    for field, value in [("meshes", []), ("skins", []), ("materials", []), ("textures", []), ("images", []), ("samplers", []), ("accessors", []), ("bufferViews", [])]:
        j[field] = value
    j.pop("animations", None)
    w = Writer(j, b"")
    name_to_node = {n["name"]: i for i, n in enumerate(j["nodes"][:keep_nodes]) if "name" in n}
    # L'abito senza le parti tolte (cappuccio, spallacci).
    outfit_view = Gltf(outfit_path)
    outfit_view.j = copy.deepcopy(outfit_view.j)
    for n in outfit_view.j["nodes"]:
        if "mesh" in n and any(d in n["name"] for d in drop):
            n.pop("mesh")
    add_part(w, outfit_view, armature, name_to_node, tex_map)
    # Testa dal corpo intero (occhi e sopracciglia compresi) e capelli.
    body = Gltf(next(src_root.joinpath("base").glob("*/Base Characters/Godot - UE")) / f"Superhero_{sex}_FullBody.gltf")
    add_part(w, body, armature, name_to_node, tex_map, head_only=True, factor=HAIR[hair_col])
    hair_dir = next(src_root.joinpath("base").glob("*/Hairstyles/Rigged to Head Bone/glTF*"))
    for h in hair:
        add_part(w, Gltf(hair_dir / f"{h}.gltf"), armature, name_to_node, tex_map, factor=HAIR[hair_col])
    # Animazione fittizia: fa sì che Bevy crei AnimationPlayer e bersagli sull'armatura.
    pelvis = name_to_node["pelvis"]
    times = w.add_array(struct.pack("<f", 0.0), 5126, "SCALAR", 1, extra={"min": [0.0], "max": [0.0]})
    rot = j["nodes"][pelvis].get("rotation", [0.0, 0.0, 0.0, 1.0])
    values = w.add_array(struct.pack("<4f", *rot), 5126, "VEC4", 1)
    j["animations"] = [{
        "name": "bind",
        "samplers": [{"input": times, "output": values, "interpolation": "LINEAR"}],
        "channels": [{"sampler": 0, "target": {"node": pelvis, "path": "rotation"}}],
    }]
    (out_dir / f"{name}.bin").write_bytes(bytes(w.blob))
    j["buffers"] = [{"uri": f"{name}.bin", "byteLength": len(w.blob)}]
    (out_dir / f"{name}.gltf").write_text(json.dumps(j, separators=(",", ":")), encoding="utf-8")
    tris = sum(j["accessors"][p["indices"]]["count"] // 3 for m in j["meshes"] for p in m["primitives"])
    print(f"{name}: {tris} triangoli, {len(j['meshes'])} mesh, {len(w.blob) / 1e6:.1f} MB")


def build_animations(src_root: Path, out_dir: Path) -> None:
    ual_path = next(src_root.joinpath("ual").glob("*/Unreal-Godot/UAL1_Standard.glb"))
    src = Gltf(ual_path)
    j = {
        "asset": src.j["asset"],
        "scene": 0,
        "scenes": [{"nodes": src.j["scenes"][0]["nodes"]}],
        "nodes": [{k: v for k, v in n.items() if k not in ("mesh", "skin")} for n in src.j["nodes"]],
        "accessors": [],
        "bufferViews": [],
        "animations": [],
    }
    w = Writer(j, b"")
    by_name = {a["name"]: a for a in src.j["animations"]}
    for clip in CLIPS:
        a = by_name[clip]
        samplers, channels = [], []
        for ch in a["channels"]:
            node_name = src.j["nodes"][ch["target"]["node"]]["name"]
            path = ch["target"]["path"]
            if path == "scale" or (path == "translation" and node_name != "pelvis"):
                continue
            s = a["samplers"][ch["sampler"]]
            samplers.append({"input": w.copy_accessor(src, s["input"]), "output": w.copy_accessor(src, s["output"]), "interpolation": s.get("interpolation", "LINEAR")})
            channels.append({"sampler": len(samplers) - 1, "target": ch["target"]})
        j["animations"].append({"name": clip, "samplers": samplers, "channels": channels})
    (out_dir / "animations.bin").write_bytes(bytes(w.blob))
    j["buffers"] = [{"uri": "animations.bin", "byteLength": len(w.blob)}]
    (out_dir / "animations.gltf").write_text(json.dumps(j, separators=(",", ":")), encoding="utf-8")
    print(f"animations: {len(CLIPS)} clip, {len(w.blob) / 1e6:.1f} MB")


def main() -> None:
    src_root, out_dir = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    prepare_textures(src_root, out_dir / "textures")
    for name, spec in MODELS.items():
        build_model(name, spec, src_root, out_dir)
    build_animations(src_root, out_dir)


if __name__ == "__main__":
    main()
