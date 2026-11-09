#!/usr/bin/env python3
"""Converte i pacchetti CC0 di Quaternius in risorse del gioco (ADR 0028). Si lancia con Blender:

    blender -b --disable-autoexec -P scripts/build_animals_trees.py -- <assets_src> <assets>

Scrive:
  - `models/animals/<animale>.glb`, con le clip in ordine alfabetico (vedi `animals::CLIPS`);
  - `models/trees/<albero>.pmesh`, geometria grezza per i riquadri di `nature.rs`:
    corteccia e chioma, ognuna in versione vicina e lontana;
  - `textures/alberi/`, corteccia e foglie (le foglie in toni quasi neutri: il colore lo dà la stagione).
"""
import math
import os
import random
import shutil
import struct
import sys

import bmesh
import bpy
from mathutils import Vector

ANIMALS = {"cow": "Cow", "sheep": "Sheep"}
# Nome nel gioco, modello di origine, altezza (m).
TREES = {
    "birch_a": ("Birch_1", 8.5),
    "birch_b": ("Birch_3", 9.5),
    "leafy_a": ("Tree_6", 8.0),
}
# I tronchi del pacchetto sono tozzi: si stringe tutto l'albero in orizzontale.
SLIM = 0.72
FAR_LEAF_TRIS = 170
FAR_BARK_TRIS = 44
TREE_PACK = "alberi/Textured Stylized Trees - May 2020"


def clean():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def build_animal(src, out, name, source):
    bpy.ops.wm.open_mainfile(filepath=f"{src}/animali/Blends/{source}.blend")
    armature = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = next(o for o in bpy.data.objects if o.type == "MESH")
    # Tutte le clip sull'armatura, altrimenti l'esportazione ne salta qualcuna.
    # Questa versione di Blender esporta come clip le tracce NLA, una per azione.
    data = armature.animation_data_create()
    data.action = None
    for t in list(data.nla_tracks):
        data.nla_tracks.remove(t)
    for action in sorted(bpy.data.actions, key=lambda a: a.name):
        track = data.nla_tracks.new()
        track.name = action.name
        track.strips.new(action.name, int(action.frame_range[0]), action)
    target = f"{out}/models/animals/{name}.glb"
    bpy.ops.export_scene.gltf(
        filepath=target,
        export_format="GLB",
        export_yup=True,
        export_skins=True,
        export_animations=True,
        export_nla_strips=True,
        export_force_sampling=True,
        export_frame_step=2,
    )
    zs = [(mesh.matrix_world @ Vector(c)).z for c in mesh.bound_box]
    print(f"@@ {name}: alto {max(zs) - min(zs):.2f}, base {min(zs):.2f}, clip {sorted(a.name for a in bpy.data.actions)}")


def islands(bm):
    """Gruppi di facce unite: ogni ciuffo di foglie è uno."""
    seen, groups = set(), []
    for start in bm.faces:
        if start.index in seen:
            continue
        stack, group = [start], []
        seen.add(start.index)
        while stack:
            f = stack.pop()
            group.append(f)
            for e in f.edges:
                for g in e.link_faces:
                    if g.index not in seen:
                        seen.add(g.index)
                        stack.append(g)
        groups.append(group)
    return groups


def split_by_material(obj):
    """Restituisce {nome materiale: oggetto} separando la mesh."""
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.separate(type="MATERIAL")
    bpy.ops.object.mode_set(mode="OBJECT")
    return {o.data.materials[0].name: o for o in bpy.data.objects if o.type == "MESH"}


def tris(obj):
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def thin_leaves(obj, target):
    """Versione lontana delle foglie: tiene pochi ciuffi e li ingrandisce per coprire lo stesso."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.faces.ensure_lookup_table()
    groups = islands(bm)
    total = sum(len(f.verts) - 2 for g in groups for f in g)
    keep = min(1.0, target / total)
    random.seed(7)
    random.shuffle(groups)
    count = max(1, int(len(groups) * keep))
    gone = [f for g in groups[count:] for f in g]
    grow = min(2.4, math.sqrt(1.0 / keep))
    for g in groups[:count]:
        verts = {v for f in g for v in f.verts}
        center = sum((v.co for v in verts), Vector()) / len(verts)
        for v in verts:
            v.co = center + (v.co - center) * grow
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    bm.to_mesh(obj.data)
    bm.free()


def write_part(f, obj, scale, offset):
    """Vertici non condivisi (ognuno con la sua normale e UV) e indici."""
    m = obj.data
    m.calc_normals_split()
    m.calc_loop_triangles()
    uv = m.uv_layers.active.data
    # Per vertice della faccia: il modello è low poly e le normali divise rendono meglio.
    verts, idx, seen = [], [], {}
    for t in m.loop_triangles:
        for loop in t.loops:
            v = m.vertices[m.loops[loop].vertex_index].co
            n = m.loops[loop].normal
            u = uv[loop].uv
            # Da Z verso l'alto a Y verso l'alto.
            key = (round(v.x, 4), round(v.y, 4), round(v.z, 4), round(n.x, 2), round(n.y, 2), round(n.z, 2), round(u.x, 4), round(u.y, 4))
            if key not in seen:
                seen[key] = len(verts)
                verts.append((
                    (v.x - offset.x) * scale * SLIM, (v.z - offset.z) * scale, -(v.y - offset.y) * scale * SLIM,
                    n.x, n.z, -n.y, u.x, 1.0 - u.y,
                ))
            idx.append(seen[key])
    f.write(struct.pack("<II", len(verts), len(idx)))
    for v in verts:
        f.write(struct.pack("<8f", *v))
    f.write(struct.pack(f"<{len(idx)}I", *idx))
    return len(idx) // 3


def build_tree(src, out, name, source, height):
    clean()
    bpy.ops.import_scene.fbx(filepath=f"{src}/{TREE_PACK}/FBX/{source}.fbx")
    obj = next(o for o in bpy.data.objects if o.type == "MESH")
    parts = split_by_material(obj)
    bark = next(o for n, o in parts.items() if "Bark" in n)
    leaves = next(o for n, o in parts.items() if "Bark" not in n)
    points = [v.co for o in (bark, leaves) for v in o.data.vertices]
    bottom = min(p.z for p in points)
    top = max(p.z for p in points)
    low = [p for p in bark.data.vertices if p.co.z < bottom + (top - bottom) * 0.05]
    base = Vector((
        sum(v.co.x for v in low) / len(low),
        sum(v.co.y for v in low) / len(low),
        bottom,
    ))
    scale = height / (top - bottom)
    far_bark = bark.copy()
    far_bark.data = bark.data.copy()
    bpy.context.collection.objects.link(far_bark)
    mod = far_bark.modifiers.new("d", "DECIMATE")
    mod.ratio = min(1.0, FAR_BARK_TRIS / tris(bark))
    bpy.context.view_layer.objects.active = far_bark
    bpy.ops.object.modifier_apply(modifier=mod.name)
    far_leaves = leaves.copy()
    far_leaves.data = leaves.data.copy()
    bpy.context.collection.objects.link(far_leaves)
    thin_leaves(far_leaves, FAR_LEAF_TRIS)
    counts = []
    with open(f"{out}/models/trees/{name}.pmesh", "wb") as f:
        f.write(b"PMS1")
        for part in (bark, leaves, far_bark, far_leaves):
            counts.append(write_part(f, part, scale, base))
    print(f"@@ {name}: triangoli corteccia/foglie/lontani {counts}")


def neutral(src_path, dst_path):
    """Porta le foglie a toni neutri conservando il contrasto e l'alfa."""
    img = bpy.data.images.load(src_path)
    w, h = img.size
    px = list(img.pixels)
    lum = [0.0] * (w * h)
    total, n = 0.0, 0
    for i in range(w * h):
        r, g, b, a = px[i * 4:i * 4 + 4]
        lum[i] = 0.3 * r + 0.6 * g + 0.1 * b
        if a > 0.5:
            total += lum[i]
            n += 1
    gain = 0.75 / (total / max(n, 1))
    for i in range(w * h):
        v = lum[i] * gain
        # Un filo di giallo: la tinta di stagione da sola virerebbe all'azzurro.
        px[i * 4] = min(1.0, v * 1.08)
        px[i * 4 + 1] = min(1.0, v * 0.98)
        px[i * 4 + 2] = min(1.0, v * 0.5)
    img.pixels = px
    img.filepath_raw = dst_path
    img.file_format = "PNG"
    img.save()


def main():
    args = sys.argv[sys.argv.index("--") + 1:]
    src, out = args[0], args[1]
    for d in ("models/animals", "models/trees", "textures/alberi"):
        os.makedirs(f"{out}/{d}", exist_ok=True)
    for name, source in ANIMALS.items():
        build_animal(src, out, name, source)
    for name, (source, height) in TREES.items():
        build_tree(src, out, name, source, height)
    clean()
    tex = f"{src}/{TREE_PACK}/Textures"
    shutil.copy(f"{tex}/Birch_Bark.png", f"{out}/textures/alberi/birch_bark.png")
    shutil.copy(f"{tex}/Tree_Bark.jpg", f"{out}/textures/alberi/tree_bark.jpg")
    neutral(f"{tex}/Birch_Leaves_Green.png", f"{out}/textures/alberi/birch_leaves.png")
    neutral(f"{tex}/Tree_Leaves.png", f"{out}/textures/alberi/tree_leaves.png")


main()
