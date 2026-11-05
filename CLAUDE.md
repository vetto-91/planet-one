# CLAUDE.md — Planet One

RPG 3D open source, cel-shaded, terza persona. Rust + Bevy 0.20.1, fisica Avian 0.8, UI egui (bevy_egui 0.43).

## Stato e roadmap

- Roadmap e stato reale delle fasi: [docs/ROADMAP.md](docs/ROADMAP.md).
- Ultima fase completata: Fase 5, vertical slice "Denaro Rubato" (`v0.0.5.0`).
- Storia dei rilasci: [CHANGELOG.md](CHANGELOG.md). Schema versioni: [docs/VERSIONING.md](docs/VERSIONING.md).

## Comandi

```bash
cargo run            # dev
cargo run --release  # ottimizzata
cargo fmt            # formattazione (obbligatoria prima di ogni commit)
cargo clippy         # linter (deve essere pulito prima di ogni commit)
```

Su Linux servono `libudev`, `alsa` e un driver Vulkan.

## Versioning

- Formato `vMAJOR.MINOR.FEATURE.BUGFIX`.
- `Cargo.toml` riporta solo i primi tre segmenti (es. `0.0.5` per `v0.0.5.0`). Il quarto vive nei tag e in CHANGELOG.
- Tag solo su `master`, dopo verifica manuale della build.

## Commit e branch

- Un commit per feature o fix. Messaggio in inglese, formato `feat(area): descrizione`, `fix(area): ...`, `chore(area): ...`.
- Branch: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`.
- Non committare: `context/` (esportazioni di sessione), `target/`, database o dati di opencode.

## Verifica

Il gioco non ha ancora test automatici. La verifica è manuale con `cargo run`, con questi controlli:
- nessun panic nei log;
- log di riferimento: `[INPUT]`, `[EVENT]`, `[CURSOR]`, `[QUEST]`; `grep "\[QUEST\]" run.log`;
- comportamento descritto nel README (controlli, quest "Denaro Rubato").

## Architettura

- `src/main.rs`: stati dell'app (`Playing`, `Paused`, `Journal`, `Wait`, `Resetting`) e cablaggio dei sistemi.
- `src/scene.rs`: costruzione e reset della scena.
- `src/terrain.rs`: heightmap → mesh e collider, hot-reload.
- `src/player.rs`: movimento, salto, corsa, ground-check (in `FixedUpdate`, character controller kinematico).
- `src/camera.rs`: orbita, terza e prima persona.
- `src/entities.rs`: NPC, nemico, dialogo.
- `src/combat.rs`, `src/inventory.rs`, `src/hud.rs`: combattimento, inventario, HUD.
- `src/quests/mod.rs`: parser dei dialoghi, voci del diario, fumetti (barks), registro quest.
- `src/quests/<quest>/mod.rs` + `assets/quests/<quest>/dialoghi.txt`: logica e testi (modificabili a mano) di ogni quest. Vedi ADR 0003.
- `src/sky.rs`: cielo da `assets/textures/sky.png` convertito in cubemap a runtime.
- `src/daynight.rs`: orologio (1 h = 60 s), sole/luna (luci e dischi), stato `Wait` (tasto T).
- `src/figure.rs`: mesh a parallelepipedo con UV per faccia per le skin in `assets/textures/{player,ladro,viandante}.png`.
- `src/menu.rs`: UI egui (menu Esc, dialogo, FPS, diario J, bussola, fumetti).

## Regole di lavoro

- Scrivere in italiano nelle note e nei documenti per l'utente; il codice, i commit e le identificazioni tecniche restano in inglese.
- Mantenere la scelta di avvio: fullscreen di default, F11 per alternare, cursore catturato solo in fullscreen e in gioco.
- Le decisioni significative vanno in `docs/decisions/` come ADR (template nel file del registro).
- Non ottimizzare prima della Fase 5 di performance: la priorità è la giocabilità.
