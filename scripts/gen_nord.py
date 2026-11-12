#!/usr/bin/env python3
"""Genera i dati del Continente del Nord, Atto I (ADR 0033): la Piana delle Brume.

Scrive, da un'unica tabella di coordinate:
- assets/world/nord/isola.txt      forma della regione (costa, lago, rio, alture, boschi, nomi dei luoghi)
- assets/world/nord/villaggio.txt  Fontebruma: piazza, cancelli, strade e sentieri
- assets/world/nord/abitanti.txt   (vuoto per ora: gli abitanti arrivano nel passo successivo)
- debug/placements-nord.txt        mura, edifici, lampioni, cartelli e i 16 luoghi

Coordinate in metri: x = Est, z = Sud (il nord è -Z). Il terreno va da -600 a +600; si gioca
entro |x| <= 520 e z >= -572. Il mare è a sud (z > 474): di là c'è l'Isola di Fontevivo.
Fonte: Trama-Planet-One/10_ATTO_I_APPRODO_BOSCO_DEL_SUD_E_SISTEMA_MAPPA.md.
Rilanciare lo script rigenera tutto (i ritocchi a mano nei file generati si perdono).
"""
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORLD = ROOT / "assets/world/nord"
PLACEMENTS = ROOT / "debug/placements-nord.txt"

COAST = 474.0
BOUNDS = (520.0, 572.0)
LAKE = (-110.0, 150.0, 110.0, 80.0)  # centro x z, semiassi
ISLET = (-110.0, 150.0, 13.0)
# Rio delle Betulle: emissario del lago, a livello del mare (lo allaga il mare stesso).
RIO = [(-8, 165), (40, 180), (90, 196), (140, 202), (195, 215), (240, 262), (268, 335), (284, 410), (290, 478)]
RIO_HALF = 4.5

# --- I 16 luoghi (POI) e il paese -------------------------------------------------------------
PLACES = {
    "Approdo delle Brume": (0, 462),
    "Lanterna Spezzata": (30, 250),
    "Nove Querce": (-380, 130),
    "Isolotto del lago": (-110, 150),
    "Segheria vecchia": (228, 292),
    "Ponte coperto": (140, 202),
    "Torre muschiosa": (-235, 20),
    "Campo dell'ambra": (190, -380),
    "Acquedotto sommerso": (-110, 262),
    "Capanna dell'eremita": (-262, 165),
    "Bivacco del nord": (24, -486),
    "Arco di pietra": (0, -548),
    "Podere delle querce cave": (-360, 330),
    "Carbonaie del fumo nero": (335, 330),
    "Palafitte dei giunchi": (-196, 92),
    "Collina di Barnaba": (360, 40),
    "Fontebruma": (0, -250),
}

HILLS = [
    (-235, 20, 46, 9),    # la Torre muschiosa sta su una leggera altura
    (360, 40, 62, 17),    # la collina tonda di Barnaba
    (-420, -260, 90, 8),
    (300, -220, 80, 6),
    (-300, 420, 70, 5),
]

# --- Sentieri (terra battuta, 3 m: sul terreno a passo 1,5 m si vedono bene) -------------------
MAIN = [(0, 458), (6, 420), (14, 380), (24, 330), (30, 290), (30, 250), (44, 215), (56, 189),
        (60, 170), (52, 120), (40, 60), (28, 0), (18, -60), (8, -120), (0, -170), (0, -192)]
NORTH = [(0, -308), (4, -350), (12, -400), (20, -450), (24, -486), (12, -520), (0, -548)]
PATHS = [
    MAIN,
    NORTH,
    # lago: pontile di Olmo sulla riva est
    [(52, 120), (30, 138), (8, 148)],
    # ovest: Torre muschiosa, eremita, Nove Querce
    [(28, 0), (-40, 18), (-110, 30), (-180, 24), (-235, 30)],
    [(-180, 24), (-225, 80), (-262, 150)],
    [(-262, 165), (-310, 150), (-380, 130)],
    # palafitte (dal sentiero dell'eremita fin dove finisce la terra)
    [(-225, 80), (-206, 92)],
    # sud del lago: acquedotto e podere
    [(30, 290), (-30, 285), (-110, 280), (-200, 300), (-290, 320), (-350, 328)],
    # est: ponte coperto, segheria, carbonaie
    [(60, 170), (100, 176), (140, 190)],
    [(140, 214), (185, 255), (222, 284)],
    [(140, 214), (200, 228), (262, 262), (300, 300), (330, 325)],
    # collina di Barnaba
    [(40, 60), (120, 50), (220, 44), (300, 40), (345, 40)],
    # campo dell'ambra
    [(12, -400), (80, -390), (140, -382), (184, -380)],
]
BRIDGES = [((52.5, 178), (58.5, 193)), ((140, 192), (140, 212))]

# --- Fontebruma ---------------------------------------------------------------------------------
FB = (0.0, -250.0)
WALL = [(-50, -195), (50, -195), (50, -305), (-50, -305)]
GATES = [("Sud", 0, -195), ("Nord", 0, -305)]
STREETS = [
    ("grande", 4.2, [(0, -192), (0, -220), (0, -250), (0, -280), (0, -308)]),
    ("piazza", 17.0, [(0, -250), (0, -249.99)]),
    ("piccola", 3.0, [(-44, -250), (-20, -250), (0, -250), (20, -250), (44, -250)]),
    ("piccola", 2.6, [(-30, -250), (-30, -222)]),
    ("piccola", 2.6, [(30, -250), (30, -282)]),
]
# (tipo, x, z, rotazione in gradi): il fronte degli edifici guarda (sin, cos) della rotazione.
VILLAGE = [
    # fila ovest della via principale, fronte a est
    ("locanda", -9.5, -222, 90), ("casa-grande", -9.5, -236.5, 90),
    ("casa-media", -9.5, -265, 90), ("fabbro", -10.5, -280, 90), ("casa-piccola", -9.0, -295, 90),
    # fila est, fronte a ovest
    ("caserma", 9.5, -208, 270), ("forno", 9.0, -222, 270), ("casa", 9.5, -235, 270),
    ("casa-media", 9.5, -265, 270), ("casa-grande", 9.5, -279, 270), ("casa-piccola", 9.0, -294, 270),
    # via trasversale
    ("chiesa", -30, -264, 0), ("stalla", 31, -259, 0),
    ("casa-piccola", -37, -243, 180), ("casa", 22, -242, 180), ("casa-piccola", 38, -243, 180),
    ("casa", -36, -216, 90), ("casa-piccola", 36, -290, 270),
    # la Fonte Grigia e il mercato
    ("pozzo", 5.5, -246, 0), ("bancarella-erbe", -6, -256, 0), ("bancarella-frutta", 6.5, -256, 0),
    ("carretto", 22, -253.5, 80), ("botti", -16, -253, 0), ("casse", 14, -246.5, 20),
    ("pagliaio", 40, -262, 0), ("panni", -22, -232, 90),
]
LAMPS = [(3, -200), (-3, -214), (3, -228), (-3, -242), (9, -250), (-9, -250), (3, -258), (-3, -272),
         (3, -286), (-3, -300), (-24, -252), (24, -248), (-40, -248), (40, -252),
         # fuori dalle porte e lungo il sentiero
         (4, -188), (-4, -312), (33, 252), (-4.5, 452), (4, 436)]
SIGNS = [("fontebruma", 4, -186, 0), ("fontebruma", -4, -314, 180), ("approdo", 6, 443, 180),
         ("lanterna", 26, 256, 180), ("lago", 48, 122, 270), ("torre-muschiosa", 24, 4, 270),
         ("collina", 44, 56, 90), ("arco", 6, -500, 0)]

# --- I luoghi: costruzioni e oggetti (tipi già esistenti nel gioco) -------------------------------
POI = [
    # Approdo delle Brume: molo, barca, rimessa
    ("molo", 0, 468, 0), ("barca", -4.5, 472, 6), ("barca", 4.6, 476, 352),
    ("casa-piccola", -14, 448, 90), ("botti", 10, 457, 0), ("casse", 12, 452, 30), ("reti", -8, 457, 0),
    # Lanterna Spezzata: la vecchia stazione di posta, con i predoni accampati
    ("casa-media", 20, 244, 90), ("casse", 26, 238, 10), ("carretto", 36, 244, 200), ("banditi", 58, 238, 250),
    # Nove Querce: santuario silvestre
    ("santuario", -380, 130, 90),
    # Lago: pontile di Olmo e isolotto
    ("barca", 2, 151, 80), ("reti", 10, 144, 0), ("botti", 11, 150, 0), ("casse", -110, 152, 0),
    ("casa-piccola", 20, 139, 250),
    # Segheria vecchia sul Rio
    ("stalla", 226, 292, 70), ("casse", 218, 284, 0), ("carretto", 224, 302, 130), ("botti", 232, 300, 0),
    # Ponte coperto dei contrabbandieri
    ("casse", 146, 188, 0), ("casse", 134, 216, 40),
    # Torre muschiosa
    ("torre-rovina", -235, 20, 135),
    # Campo dell'ambra
    ("botti", 190, -380, 0), ("casse", 194, -376, 20), ("pagliaio", 186, -386, 0), ("carretto", 198, -384, 300),
    # Acquedotto sommerso
    ("rovine", -110, 262, 0),
    # Capanna dell'eremita
    ("casa-piccola", -266, 168, 110), ("botti", -260, 173, 0),
    # Bivacco del nord (disertori)
    ("banditi", 30, -490, 200),
    # Arco di pietra
    ("arco", 0, -548, 0),
    # Podere delle querce cave
    ("casa-grande", -363, 332, 80), ("pagliaio", -352, 340, 0), ("pagliaio", -356, 322, 0), ("orto", -370, 344, 0),
    # Carbonaie: le due capanne una di fronte all'altra, a venti passi
    ("casa-piccola", 327, 330, 90), ("casa-piccola", 345, 330, 270), ("carbonaia", 336, 318, 0),
    ("carbonaia", 330, 344, 0), ("carbonaia", 342, 343, 0),
    # Palafitte dei giunchi bianchi
    ("palafitta", -198, 98, 300), ("palafitta", -188, 104, 330), ("casa-grande", -218, 84, 120),
    # Collina di Barnaba
    ("casa-piccola", 356, 46, 250), ("casse", 362, 34, 0), ("botti", 352, 38, 0),
]


def forests():
    """Bosco fitto su tutta la piana: cerchi sovrapposti, querce a ovest e a sud, betulle a nord e a est."""
    out = []
    step = 150
    for iz in range(-4, 4):
        for ix in range(-4, 5):
            x = ix * step + (75 if iz % 2 else 0)
            z = iz * step + 60
            if abs(x) > 560 or z > COAST - 40:
                continue
            # niente bosco in mezzo al lago né sul paese
            if math.hypot((x - LAKE[0]) / LAKE[2], (z - LAKE[1]) / LAKE[3]) < 0.7:
                continue
            if math.hypot(x - FB[0], z - FB[1]) < 60:
                continue
            h = math.sin(x * 0.013 + 1.7) + math.cos(z * 0.011 + 0.4)
            kind = "querce" if h > 0.45 else "betulle" if h < -0.45 else "querce-betulle"
            dens = 0.5 + 0.12 * math.sin(x * 0.031 + z * 0.017)
            out.append((x, z, 108, round(dens, 2), kind))
    return out


def fmt(pts):
    return ", ".join(f"{x:.2f} {z:.2f}" for x, z in pts)


def write_spec():
    t = ["# Continente del Nord, Atto I: la Piana delle Brume (generato da scripts/gen_nord.py).",
         "# Coordinate in metri: x = Est, z = Sud (il nord è -Z). Il mare è a sud.",
         "#",
         "# costa: z della riva a sud   confine: |x| massimo e z minimo (nord) dove si può andare",
         f"costa: {COAST:.0f}", f"confine: {BOUNDS[0]:.0f} {BOUNDS[1]:.0f}",
         "# lago-grande: x z semiasse_x semiasse_z   isolotto: x z raggio",
         "lago-grande: %g %g %g %g" % LAKE, "isolotto: %g %g %g" % ISLET,
         "# canale: semi_larghezza | x z, ...   (il Rio delle Betulle, dal lago al mare)",
         f"canale: {RIO_HALF} | {fmt(RIO)}", "#", "# altura: x z raggio altezza"]
    t += ["altura: %g %g %g %g" % h for h in HILLS]
    t += ["#", "# ponte: x1 z1 x2 z2"]
    t += ["ponte: %g %g %g %g" % (a + b) for a, b in BRIDGES]
    t += ["#", "# bosco: x z raggio densità tipo (querce, betulle, querce-betulle)"]
    t += ["bosco: %g %g %g %g %s" % f for f in forests()]
    t += ["", "gabbiani: 0 470 5", "", "# Nomi sulla mappa."]
    t += [f"luogo: {name} | {x:g} {z:g}" for name, (x, z) in PLACES.items()]
    (WORLD / "isola.txt").write_text("\n".join(t) + "\n")


def write_village():
    t = ["# Fontebruma, il paese nel bosco (generato da scripts/gen_nord.py).",
         "# Le MURA sono i piazzamenti \"muro\" di debug/placements-nord.txt.",
         "nome: Fontebruma", f"piazza: {FB[0]:.1f} {FB[1]:.1f}", ""]
    t += [f"cancello: {n} {x:.1f} {z:.1f}" for n, x, z in GATES]
    t += ["", "# Strade del paese."]
    t += [f"strada: {k} {w} | {fmt(p)}" for k, w, p in STREETS]
    t += ["", "# Sentieri della piana (terra battuta)."]
    t += [f"strada: sentiero {3.2 if p is MAIN or p is NORTH else 2.8} | {fmt(p)}" for p in PATHS]
    (WORLD / "villaggio.txt").write_text("\n".join(t) + "\n")


# --- Abitanti (Trama-Planet-One/10, sezione 4) ----------------------------------------------------
# nome, (tipo di casa, x, z della casa, letto), ruolo, modello, lavoro (x, z, dx, dz), orari, altro
VILLAGERS = [
    ("Gaspare", ("casa-piccola", -14, 448, 0), "pescatore", "civile-m-verde", (5.0, 461.0, -1, -1), "4 23 24", []),
    # Fontebruma
    ("Anziano Vieri", ("casa-grande", -9.5, -236.5, 0), "custode della fonte", "prete", (3.4, -248.6, -1, 1), "7 20 22", []),
    ("Dalia", ("locanda", -9.5, -222, 0), "locandiera", "civile-f-rosa", (-4.4, -222.0, 1, 0), "6 23 24", []),
    ("Brando", ("casa-grande", 9.5, -279, 0), "carpentiere", "civile-m-rosso", (4.4, -279.0, -1, 0), "7 19 22",
     ["sera: 4.0 -252.5 -1 0"]),
    ("Mirta", ("casa-media", 9.5, -265, 0), "erborista", "civile-f-viola", (-6.0, -257.4, 0, 1), "7 18 21",
     ["negozio: erborista"]),
    ("Corrado", ("fabbro", -10.5, -280, 0), "fabbro", "civile-m-rosso", (-4.6, -280.0, 1, 0), "7 19 22",
     ["negozio: armaiolo"]),
    ("Luce", ("casa", 22, -242, 0), "staffetta", "arciera", (-3.6, -244.5, 1, 0), "6 19 22",
     ["sera: -5.0 -253.0 1 0"]),
    ("Sereno", ("locanda", -9.5, -222, 1), "mercante", "civile-m-ocra", (6.5, -257.4, 0, 1), "8 19 23",
     ["negozio: viandante"]),
    ("Capitano Dario", ("caserma", 9.5, -208, 0), "capitano delle guardie", "guardia", (3.6, -203.0, -1, 1), "6 22 23",
     ["guardia: si"]),
    ("Guardia Greta", ("caserma", 9.5, -208, 1), "guardia", "arciera", (0, 0, 0, 1), None,
     ["guardia: si", "cancello: Sud", "turno: 1"]),
    ("Guardia Tancredi", ("caserma", 9.5, -208, 2), "guardia", "guardia", (0, 0, 0, 1), None,
     ["guardia: si", "cancello: Sud", "turno: 2"]),
    ("Guardia Vespero", ("caserma", 9.5, -208, 3), "guardia", "guardia", (0, 0, 0, 1), None,
     ["guardia: si", "cancello: Sud", "turno: 3"]),
    # Sul lago e nel bosco
    ("Olmo", ("casa-piccola", 20, 139, 0), "pescatore", "civile-m-verde", (8.5, 149.5, -1, 0), "5 19 21", []),
    ("Nereo", ("casa-piccola", -266, 168, 0), "eremita", "civile-m-ocra", (-261.0, 171.5, 1, -1), "7 20 22", []),
    # Podere delle querce cave
    ("Giona", ("casa-grande", -363, 332, 0), "apicoltore", "civile-m-ocra", (-353.5, 337.0, 1, 0), "7 19 21", []),
    ("Ada", ("casa-grande", -363, 332, 1), "vasaia", "civile-f-viola", (-367.0, 341.5, 0, 1), "6 19 21", []),
    ("Milo", ("casa-grande", -363, 332, 2), "ragazzo", "civile-m-verde", (-357.0, 326.0, 1, 0), "8 18 20", []),
    # Carbonaie del fumo nero
    ("Rocco", ("casa-piccola", 327, 330, 0), "carbonaio", "civile-m-rosso", (332.5, 321.0, 1, 0), "6 20 22", []),
    ("Brenno", ("casa-piccola", 345, 330, 0), "carbonaio e fonditore", "civile-m-verde", (339.0, 339.5, -1, 0), "6 20 22", []),
    # Palafitte dei giunchi bianchi
    ("Tullio", ("casa-grande", -218, 84, 0), "costruttore di chiatte", "civile-m-ocra", (-207.0, 93.0, 1, 1), "6 19 21", []),
    ("Sira", ("casa-grande", -218, 84, 1), "tessitrice", "civile-f-rosa", (-211.0, 90.0, 1, 0), "7 19 21", []),
    ("Nonna Calliope", ("casa-grande", -218, 84, 2), "anziana", "civile-f-viola", (-213.5, 87.0, 1, 1), "8 18 20", []),
    # Collina di Barnaba
    ("Barnaba", ("casa-piccola", 356, 46, 0), "eremita della collina", "civile-m-ocra", (351.0, 41.0, -1, 0), "6 21 23", []),
]


def home_index(kind, x, z):
    """Indice (da 0) dell'edificio di quel tipo più vicino a (x, z), nell'ordine dei piazzamenti."""
    same = [(k, px, pz) for k, px, pz, _ in VILLAGE + POI if k == kind]
    best = min(range(len(same)), key=lambda i: math.hypot(same[i][1] - x, same[i][2] - z))
    assert math.hypot(same[best][1] - x, same[best][2] - z) < 1.0, (kind, x, z)
    return best


def write_villagers():
    t = ["# Abitanti di Fontebruma e della piana (generato da scripts/gen_nord.py; stesso formato di",
         "# assets/world/abitanti.txt). Persona e voce per la IA: assets/ai/<nome>.txt.",
         "# Gaspare aspetta con la barca all'Approdo delle Brume per riportarti sull'isola.", ""]
    for name, (kind, hx, hz, bed), role, model, work, hours, extra in VILLAGERS:
        t.append(f"[{name}]")
        t.append(f"casa: {kind} {home_index(kind, hx, hz)} {bed}")
        t.append(f"ruolo: {role}")
        t.append(f"modello: {model}")
        t += extra
        t.append("lavoro: %g %g %g %g" % work)
        if hours:
            t.append(f"orari: {hours}")
        t.append("")
    (WORLD / "abitanti.txt").write_text("\n".join(t))


def write_placements():
    t = ["# Piazzamenti del Continente del Nord (generati da scripts/gen_nord.py; stesso formato di",
         "# debug/placements.txt). x = Est (+) / Ovest (-), z = Sud (+) / Nord (-).", ""]
    n = [0]

    def put(kind, x, z, rot=None, note=""):
        n[0] += 1
        t.append(f"[P-{n[0]:04d}] tipo: {kind} | modalità: singolo")
        t.append("quando: generato da gen_nord.py")
        t.append(f"punto: x {x:.2f}, z {z:.2f}")
        t.append("quantità: 1   seed: %d" % (n[0] * 7919 % 1000000))
        if note:
            t.append(f"nota: {note}")
        t.append(f"punti: ({x:.2f},{z:.2f})")
        if rot is not None:
            t.append(f"rotazioni: {rot:g}")
        t.append("")

    for x, z in WALL:
        put("muro", x, z, 0)
    for kind, x, z, rot in VILLAGE:
        put(kind, x, z, rot, "fontebruma")
    for x, z in LAMPS:
        put("lampione", x, z, 0)
    for key, x, z, rot in SIGNS:
        put("cartello", x, z, rot, key)
    for kind, x, z, rot in POI:
        put(kind, x, z, rot, "luogo")
    PLACEMENTS.write_text("\n".join(t) + "\n")
    return n[0]


def main():
    WORLD.mkdir(parents=True, exist_ok=True)
    write_spec()
    write_village()
    write_villagers()
    count = write_placements()
    print(f"Continente del Nord: {len(PLACES)} luoghi, {len(forests())} boschi, {count} piazzamenti")


if __name__ == "__main__":
    main()
