# Roadmap — Planet One

Roadmap a fasi, con stato reale. Versioning e convenzioni: vedi [VERSIONING.md](VERSIONING.md).
Il dettaglio di ogni rilascio è in [CHANGELOG.md](../CHANGELOG.md); i tag Git `vX.Y.Z.W` sono la fonte di verità.

Strategia: **vertical slice prima**. Una demo giocabile (1 quest, 1 NPC, 1 nemico, inventario minimale) viene prima di contenuti e ottimizzazioni.

## Fasi completate

| Fase | Contenuto | Tag |
|---|---|---|
| 0 — Bootstrap | Finestra Bevy, camera, cubo placeholder, piano terra | `v0.0.0.1`–`v0.0.0.2` |
| 1 — Movimento | WASD in terza persona, orbita camera, gravità, salto, corsa, menu Esc, reset scena, FPS | `v0.0.1.0`–`v0.0.1.4` |
| 2 — Scena | Terreno da heightmap (mesh + collider Avian), cielo procedurale, luci, hot-reload asset | `v0.0.2.0`–`v0.0.2.5` |
| 2.x — Character controller | Kinematico Avian con MoveAndSlide, coyote time, jump buffer, ground-check multi-ray | `v0.0.2.1`–`v0.0.2.5` |
| 3 — Entità | NPC "Viandante" con dialogo, nemico in patrol, interazione E | `v0.0.3.0` |
| 4 — HUD / Inventario / Combattimento | Barra vita, crosshair, inventario con pickup, melee, morte e respawn | `v0.0.4.0`–`v0.0.4.9` |
| 4.x — Camera e input | Prima persona (RMB/V), barre nemico/player, fullscreen F11, cattura cursore, log input | `v0.0.4.2`–`v0.0.4.9` |
| 5 — Vertical slice | Quest "Denaro Rubato", bussola, obiettivi (TAB), loot | `v0.0.5.0` |
| 5.x — Rifiniture | Diario quest (J), quest in cartelle con dialoghi in file, texture erba e cielo, fumetti NPC/ladro, ciclo giorno/notte (T Aspetta), skin dei personaggi, targhette, tema UI | `v0.0.6.0` |
| 5.y — Isola e rifiniture debug | Terreno a isola (300 m) con mare, F4 col mirino e Ctrl, rotazione con freccia rossa, niente compenetrazioni, niente interazioni attraverso i muri | `v0.0.11.0` |
| 5.z — Casetta e costa | Interazioni da dentro, finestre a vetro scorrevole + scuri, vista attraverso banco e vetri, mare solo attorno all'isola, barriera di riva, spiaggia di sabbia, F3 leggero | `v0.0.12.0` |
| 5.y — Mondo vivo | Colonna sonora con volume, casetta-chiosco esplorabile (porta, interni, stufa con fumo), routine giorno/notte del Viandante, voci dei personaggi, versione nel menu | `v0.0.7.0` |
| 5.z — Mondo vivo, rifiniture | Interazione col mirino, casetta con serranda/interruttore/finestre e luci con ombre, routine del Viandante v2, voci normalizzate con ducking della musica | `v0.0.8.0` |
| 5.w — Mappa e strumenti | Mappa dall'alto (M) con bussola e zoom, modalità debug piazzamenti (F4) con file `debug/placements.txt`, vetri e scuri spessi, il Viandante chiude porta e finestre | `v0.0.9.0` |
| 5.x — Strumenti debug e oggetti veri | Poligono/griglia/linea/gomma/sposta in F4, mappa spostabile, musica notturna, alberi/fiori/panchina/lampione da `debug/placements.txt`, tetto spesso, ronda notturna per la luce | `v0.0.10.0` |

## Backlog (fasi future, non ancora pianificate nel dettaglio)

- **Game over / schermata di vittoria**: non ancora implementate nella slice.
- **Salvataggio su file** (serde): rimandato; la slice non salva su disco.
- **Input gamepad**: opzionale.
- **Audio**: musica di sottofondo con volume nel menu (bevy_audio) fatta; restano effetti sonori e voci dei personaggi (testi in `docs/voci/`).
- **UI**: restano da valutare i confini tra egui (attuale) e bevy_ui.
- **Asset**: sostituire i placeholder con modelli glTF CC0 (Kenney, Quaternius) esportati da Blender con shading cel.
- **Fuori scope v1**: multiplayer. Web build (WASM) da valutare dopo la stabilità su Linux.

## Decisioni architetturali

Vedi [docs/decisions/](decisions/README.md) (ADR-lite). Stack: Rust + Bevy 0.20.1, Avian 0.8, bevy_egui 0.43.

## Convenzioni di sviluppo

- Ogni task è un commit atomico con messaggio `feat(area): descrizione` (o `fix`/`chore`).
- Prima di ogni commit: `cargo fmt` e `cargo clippy`.
- Verifica manuale con `cargo run`, controllando i log `[INPUT]`, `[QUEST]`, `[CURSOR]`, ecc.
- Gli asset binari pesanti non vanno nel repo (usare LFS o release).
