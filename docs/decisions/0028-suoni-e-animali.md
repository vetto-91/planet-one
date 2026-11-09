# ADR-0028: Suoni d'ambiente e animali

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.39.0

## Contesto
Fuori dal paese l'isola era muta e immobile: si sentivano solo meteo e campane.

## Decisione
- Tutti i suoni sono sintetizzati all'avvio, come pioggia e campane: nessun file audio e nessuna rete. Gli anelli (onde, ruscello, grilli, piazza, vento in quota) hanno un volume calcolato dal luogo; uccelli e gabbiani sono suoni sparsi a caso.
- Il suono dei passi dipende dal materiale del vertice più vicino (`TerrainField::surface_at`) e dalla quota dei piedi: sopra il terreno è legno, sulle mura pietra, sul mare molo o acqua.
- Il nuovo volume «Ambiente» raccoglie anche meteo e campane, che prima seguivano la musica.
- Mucche e pecore sono modelli CC0 di Quaternius (`Farm Animals Animated`), con le clip prese dai `.blend` perché gli FBX della pecora ne hanno solo due. Gabbiani, uccelli, galline, pesci e farfalle sono fatti nel codice.
- Le latifoglie vengono da `Textured Stylized Trees` (CC0, Quaternius). Il formato `.pmesh` è geometria grezza letta in modo sincrono da `nature.rs`, che fonde le copie per riquadro. Se manca il file si torna alla latifoglia generata. Foglie in toni quasi neutri: il colore lo dà la stagione.
- Gli animali non si salvano: si ricreano secondo ora e stagione. Hanno zone da `isola.txt` e non hanno collider: non possono bloccare il giocatore.

## Conseguenze
- I pacchetti restano in `assets_src/` (fuori da git); per rigenerare i modelli serve `scripts/build_animals_trees.py` con Blender.
- Il bilanciamento dei volumi è una stima: è verificato da test e log, non all'ascolto.
- Da vicino i tronchi delle betulle costano circa 2.500 triangoli a pianta, solo entro 55 m.
