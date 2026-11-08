# CLAUDE.md — Planet One

RPG 3D open source, cel-shaded, terza persona. Rust + Bevy 0.20.1, fisica Avian 0.8, UI egui (bevy_egui 0.43).

## Stato e roadmap

- Roadmap e stato reale delle fasi: [docs/ROADMAP.md](docs/ROADMAP.md).
- Ultima fase completata: Fase 5, vertical slice "Denaro Rubato" (`v0.0.5.0`); ultimo rilascio `v0.0.14.0`.
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
- Branch principale: `main`. Tag solo su `main`, dopo verifica manuale della build.

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

- `src/main.rs`: stati dell'app (`Playing`, `Paused`, `Journal`, `Wait`, `Map`, `Resetting`) e cablaggio dei sistemi.
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
- `src/world_props.rs`: alberi, fiori, panchine, lampioni veri creati da `debug/placements.txt`.
- `src/audio.rs`: musica di sottofondo giorno/notte con dissolvenza (OGG in `assets/audio/music/`), volume musiche salvato in `~/.config/planet-one/settings.txt`.
- `src/hut.rs`: casetta-chiosco del Viandante (mesh con UV dalla foto `assets/textures/capanna.png`, fondamenta e scale di pietra, interni, oggetti con stato `PropState`: porta, finestre, stufa con fumo, luce, serranda) e routine giornaliera del negoziante (`Keeper`). Gli oggetti si usano col mirino (`Interactable.aim`, vedi `entities::interact`).
- `src/map.rs`: mappa (M): camera ortografica dall'alto sulla scena reale + overlay egui (bussola, segnalino); `MapView` per lo zoom.
- `src/debug_place.rs`: modalità debug piazzamenti (F4), file `debug/placements.txt` (riletto all'avvio).
- `src/devtools.rs`: variabili d'ambiente di debug (`PLANET_ONE_HOUR/POS/LOOK`) per provare il gioco e fare screenshot.
- `src/props.rs`: `SnapToGround` (appoggio a terra con raycast).
- `src/voice.rs`: voci dei personaggi (`assets/audio/voci/`, mappa in `assets/quests/<quest>/voci.txt` generata da `scripts/export_voci.py`).
- `src/version.rs`: `GAME_VERSION` mostrata nel menu Esc; aggiornare a ogni rilascio.
- `docs/voci/` + `scripts/export_voci.py`: battute per creare le voci; rigenerare dopo aver cambiato `dialoghi.txt`.
- `src/ui_style.rs`: palette, targhette (`plate`) e tema egui condiviso (stesso stile dei dialoghi).
- `src/menu.rs`: UI egui (menu Esc, dialogo, FPS, diario J, bussola, fumetti).

## Regole di lavoro

- Scrivere in italiano nelle note e nei documenti per l'utente; il codice, i commit e le identificazioni tecniche restano in inglese.
- Mantenere la scelta di avvio: fullscreen di default, F11 per alternare, cursore catturato solo in fullscreen e in gioco.
- Le decisioni significative vanno in `docs/decisions/` come ADR (template nel file del registro).
- Non ottimizzare prima della Fase 5 di performance: la priorità è la giocabilità.

## Repository e sincronizzazione

- Remote GitHub: repo privato `planet-one`. Non committare `target/`, `.claude/`, `context/`, log (vedi `.gitignore`).
- Piattaforme: Linux (tarball costruito in locale), Windows e macOS (costruiti da GitHub Actions al push del tag, `.github/workflows/release.yml`, ADR 0004; non provabili da Linux).
- Backup su Google Drive: `scripts/sync_drive.sh` (esclude `target/`, worktree e `context/`); provare prima con `--dry-run`.
