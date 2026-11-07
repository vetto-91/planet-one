#!/usr/bin/env python3
"""Genera docs/voci/viandante.txt e docs/voci/ladro.txt dai testi del gioco.

Fonte unica: assets/quests/denaro_rubato/dialoghi.txt (più le battute di
riserva del Viandante in src/entities.rs e le costanti in src/quests). Rilanciare
dopo ogni modifica ai dialoghi:

    python3 scripts/export_voci.py
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIALOGHI = ROOT / "assets/quests/denaro_rubato/dialoghi.txt"
OUT = ROOT / "docs/voci"

# --- valori dei segnaposto, letti dal codice ---
quest_src = (ROOT / "src/quests/denaro_rubato/mod.rs").read_text()
VARS = {
    "total": re.search(r"LOOT_TOTAL: u32 = (\d+)", quest_src).group(1),
    "reward": re.search(r"REWARD_COINS: u32 = (\d+)", quest_src).group(1),
}

# --- parser del formato di dialoghi.txt ---
sections: dict[str, dict[str, list[str]]] = {}
cur = None
for raw in DIALOGHI.read_text().splitlines():
    line = raw.strip()
    if not line or line.startswith("#"):
        continue
    m = re.fullmatch(r"\[(.+)\]", line)
    if m:
        cur = sections.setdefault(m.group(1).strip(), {"lines": [], "choices": []})
    elif cur is not None:
        if line.startswith("?"):
            cur["choices"].append(line[1:].strip())
        else:
            cur["lines"].append(line)


def fill(text: str) -> str:
    for k, v in VARS.items():
        text = text.replace("{" + k + "}", v)
    return text


# battute di riserva del Viandante (usate solo se mancano sezioni nel file)
ent = (ROOT / "src/entities.rs").read_text()
block = re.search(r"lines: &\[(.*?)\],", ent, re.S).group(1)
FALLBACK = re.findall(r'"([^"]+)"', block)

DIREZIONI = ["nord", "nord-est", "est", "sud-est", "sud", "sud-ovest", "ovest", "nord-ovest"]
DISTANZE = list(range(30, 65, 5))  # oggi il ladro sta a ~40-45 m dal chiosco

# ---------------- Viandante ----------------
rows: list[tuple[str, str]] = []  # (quando, testo)


def add(when: str, key: str):
    lines = sections[key]["lines"]
    for i, t in enumerate(lines, 1):
        rows.append((f"{when} (riga {i}/{len(lines)})", fill(t)))


add("Primo incontro, quest da accettare", "idle")
seek = sections["seek_thief"]["lines"]
rows.append(("Quest accettata, ladro vivo, riga 1/2: COMPONIBILE (vedi sotto)", "Il ladro si trova a {direzione}, circa {N} metri."))
rows.append(("Quest accettata, ladro vivo, riga 2/2", fill(seek[1])))
add("Quest accettata, direzione del ladro non disponibile", "seek_thief_nohint")
add("Ladro sconfitto, da raccogliere le monete", "collecting")
add("Monete riportate: ricompensa", "returning")
add("Quest completata, ogni dialogo successivo", "completed")
rows.append(("Il giocatore si allontana durante il dialogo (fumetto)", fill(sections["saluto"]["lines"][0])))
fallback_rows = [(f"Riserva, riga {i}/{len(FALLBACK)} (solo se mancano sezioni)", t) for i, t in enumerate(FALLBACK, 1)]

HEAD = """{nome}: battute per la voce
{sep}
Formato: ID | quando viene detta | testo esatto
Le battute sono in ordine di storia. Il testo è quello del gioco, da leggere così com'è.
Suggerimento di tono (modificabile): {tono}
File generato da scripts/export_voci.py a partire da assets/quests/denaro_rubato/dialoghi.txt.
Non modificare a mano: cambia i dialoghi nel gioco e rilancia lo script.

"""

v = HEAD.format(
    nome="VIANDANTE",
    sep="=" * 40,
    tono="voce adulta, calda e un po' roca, da negoziante ambulante cordiale e leggermente stanco.",
)
n = 1
for when, text in rows:
    v += f"V-{n:03d} | {when} | {text}\n"
    n += 1
v += "\n--- Testi componibili (direzione e distanza del ladro) ---\n"
v += "Nel gioco la frase cambia con la posizione del ladro: registrare i pezzi e montarli\n"
v += "'Il ladro si trova a ' + direzione + ', ' + 'circa N metri' (pausa breve dopo la direzione).\n"
v += "Oggi, dal chiosco, il ladro risulta a nord-est, a circa 40-45 m.\n\n"
for d in DIREZIONI:
    v += f"V-{n:03d} | Direzione | Il ladro si trova a {d}\n"
    n += 1
for m in DISTANZE:
    v += f"V-{n:03d} | Distanza | circa {m} metri\n"
    n += 1
v += "\n--- Opzionali: battute di riserva (nel codice, usate solo se mancano sezioni) ---\n"
for when, text in fallback_rows:
    v += f"V-{n:03d} | {when} | {text}\n"
    n += 1
v += "\n--- Fuori da questo file ---\n"
v += "Le scelte del giocatore (" + "; ".join(f'"{c}"' for c in sections["idle"]["choices"]) + ") non sono del Viandante.\n"

# ---------------- Ladro ----------------
l = HEAD.format(
    nome="LADRO",
    sep="=" * 40,
    tono="voce sfrontata e beffarda, sicura di sé, un po' teatrale.",
)
l += f"L-001 | Appena vede il giocatore, a una dozzina di metri (fumetto, una sola volta) | {fill(sections['ladro_avvistato']['lines'][0])}\n"
l += "\n--- Battute future (non ancora nel gioco) ---\n"
l += "Nessuna per ora: aggiungerle in assets/quests/denaro_rubato/dialoghi.txt e rilanciare lo script.\n"

OUT.mkdir(parents=True, exist_ok=True)
(OUT / "viandante.txt").write_text(v)
(OUT / "ladro.txt").write_text(l)
print("scritti", OUT / "viandante.txt", "e", OUT / "ladro.txt")
