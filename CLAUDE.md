# CLAUDE.md — Planet One

RPG 3D open source, grafica realistica PBR in corso (ADR 0015), terza persona. Rust + Bevy 0.20.1, fisica Avian 0.8, UI egui (bevy_egui 0.43).

## Stato e roadmap

- Roadmap e stato reale delle fasi: [docs/ROADMAP.md](docs/ROADMAP.md).
- Ultima fase completata: Fase 5, vertical slice "Denaro Rubato" (`v0.0.5.0`); ultimo rilascio `v0.0.65.0`.
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

- `cargo test`: test logici e geometrici, compresa la prova di camminata con fisica vera (`walk_test.rs`).
- `scripts/beta_check.sh`: copioni del pilota automatico in `prove/*.txt` (rapporto in `prove/out/RAPPORTO.md`); un copione nuovo per ogni bug riprodotto.
- Verifica manuale con `cargo run`:
- nessun panic nei log;
- log di riferimento: `[INPUT]`, `[EVENT]`, `[CURSOR]`, `[QUEST]`; `grep "\[QUEST\]" run.log`;
- comportamento descritto nel README (controlli, quest "Denaro Rubato").

## Architettura

- `src/main.rs`: stati dell'app (`MainMenu`, `Loading`, `Playing`, `Paused`, `Journal`, `Wait`, `Map`, `Resetting`) e cablaggio dei sistemi.
- `src/scene.rs`: costruzione e reset della scena.
- `src/region.rs`: regioni del mondo (Isola di Fontevivo, Continente del Nord): misura e passo del terreno, file dei dati, punto d'arrivo; viaggio in `save::start_travel` (ADR 0033). Dati del continente in `assets/world/nord/` e `debug/placements-nord.txt`, generati da `scripts/gen_nord.py`.
- `src/terrain.rs`: heightmap → mesh e collider, hot-reload; `src/water.rs`: alture, scogliere, ruscello, onde da `assets/world/isola.txt` (ADR 0026).
- `src/player.rs`: movimento, salto, corsa, ground-check (in `FixedUpdate`, character controller kinematico).
- `src/camera.rs`: orbita, terza e prima persona.
- `src/entities.rs`: NPC, nemico, dialogo.
- `src/inventory_ui.rs`: inventario (tasto I), filtri, icone egui; icone in `assets/textures/icone/` da `scripts/make_item_icons.py`. Ricompense uniche delle secondarie (Lanterna di Tiro, Mantello, Guanti, Arco Maestro, Scudo dell'Orlo, forno di Marta) in `inventory.rs`/`shop.rs` (ADR 0051).
- `src/combat.rs`, `src/inventory.rs`, `src/hud.rs`: combattimento, inventario, HUD.
- `src/quests/mod.rs`: parser dei dialoghi, voci del diario, fumetti (barks), registro quest.
- `src/quests/<quest>/mod.rs` + `assets/quests/<quest>/dialoghi.txt`: logica e testi (modificabili a mano) di ogni quest. Vedi ADR 0003.
- `src/quests/missions.rs` + `assets/quests/prologo/missioni.txt` e `secondarie.txt`: missioni a tappe guidate dai file di testo (storia principale del Prologo, ADR 0034; secondarie, ADR 0036; `eventi.txt`, ADR 0037); scelte `scegli: testo => effetti` e oggetti di trama `inventory::Relic` (ADR 0044).
- `src/sky.rs`: cielo da `assets/textures/sky.png` convertito in cubemap a runtime.
- `src/weather.rs`: meteo deterministico per data (sereno, nuvoloso, pioggia, temporale, nebbia, neve), temperatura, vento, nebbia di distanza, particelle di pioggia/neve, neve a terra, suoni sintetizzati in locale (ADR 0014).
- `src/seasons.rs`: stagioni visibili (tinte erba/chiome, fiori, neve a terra e sui tetti, foglie che cadono) da `look(giorno)`, ADR 0012.
- `src/daynight.rs`: orologio (1 h = 60 s), data e stagioni (`GameDate`), alba/tramonto stagionali (ADR 0011), sole/luna (luci e dischi), stato `Wait` (tasto T).
- `src/figure.rs`: personaggi glTF CC0 con scheletro comune in `assets/models/characters/` (composti da `scripts/build_characters.py` partendo da `assets_src/`, non in git), clip di riposo e oggetti agganciati alle ossa delle mani (ADR 0015).
- `src/pbr.rs`: materiali PBR condivisi (texture CC0 Poly Haven in `assets/textures/pbr/`, `scripts/fetch_pbr_textures.py`); `src/trees.rs`: abeti con corteccia e rami a carte (ADR 0016).
- `src/par.rs`: calcolo in parallelo deterministico (`par::map`, `par::collect`) per la generazione all'avvio; mesh e texture solo GPU con `pbr::gpu_only`/`pbr::load_gpu` (ADR 0056).
- `src/graphics.rs`: impostazioni grafiche del menu Esc (`GfxOptions` in `AudioSettings`), applicate a camera, sole e particelle.
- `src/animation.rs`: macchina a stati delle animazioni per tutti i personaggi, velocità misurata dallo spostamento (ADR 0017).
- `src/nature.rs`: boschi, cespugli, rocce, fiori ed erba a riquadri, da `assets/world/isola.txt` (ADR 0027).
- `src/ambience.rs`: suoni d'ambiente e passi per superficie, sintetizzati in locale (ADR 0028); `src/animals.rs`: mucche, pecore, galline, gabbiani, uccelli, pesci, farfalle, dati in `assets/world/isola.txt`; modelli da `scripts/build_animals_trees.py` (Blender) partendo da `assets_src/`.
- `src/walk_test.rs`: prova di camminata con fisica vera (solo test) per scale e torri (ADR 0030).
- `src/village_life.rs`: fumo dei camini del paese e torce sulle mura; carretti, botti, casse, panni e reti sono piazzamenti (`buildings::model`, `scripts/gen_paese.py`).
- `src/places.rs`: luoghi da scoprire e banditi che ricompaiono (ADR 0029), posizioni da `scripts/gen_luoghi.py`, nomi in `isola.txt` (`luogo:`); luoghi scoperti (`Known`) e viaggio veloce dalla mappa (ADR 0037).
- `src/world_props.rs`: alberi, fiori, panchine, lampioni veri creati da `debug/placements.txt`.
- `src/audio.rs`: musica di sottofondo giorno/notte con dissolvenza (OGG in `assets/audio/music/`), volume musiche salvato in `~/.config/planet-one/settings.txt`; audio posizionale (ascoltatore sulla camera, `spatial_once`, ADR 0032).
- `src/hut.rs`: casetta-chiosco del Viandante (mesh con UV dalla foto `assets/textures/capanna.png`, fondamenta e scale di pietra, interni, oggetti con stato `PropState`: porta, finestre, stufa con fumo, luce, serranda) e routine giornaliera del negoziante (`Keeper`). Gli oggetti si usano col mirino (`Interactable.aim`, vedi `entities::interact`).
- `src/map.rs`: mappa (M): camera ortografica dall'alto sulla scena reale + overlay egui (bussola, segnalino); `MapView` per lo zoom.
- `src/debug_place.rs`: modalità debug piazzamenti (F4), file `debug/placements.txt` (riletto all'avvio).
- `src/autopilot.rs`: pilota automatico (`PLANET_ONE_SCRIPT`, ADR 0031): copioni con `vai`, `guarda`, `premi`, `verifica`, stato JSON, foto; `src/bugreport.rs`: F12 «Segnala bug» e copia delle ultime righe del log.
- `src/devtools.rs`: variabili d'ambiente di debug (`PLANET_ONE_HOUR/POS/LOOK`) per provare il gioco e fare screenshot.
- `src/props.rs`: `SnapToGround` (appoggio a terra con raycast).
- `src/voice.rs`: voci dei personaggi (`assets/audio/voci/`, mappa in `assets/quests/<quest>/voci.txt` generata da `scripts/export_voci.py`).
- `src/localtime.rs`: ora locale della macchina per salvataggi, debug e log (`libc::localtime_r` su Unix).
- `src/version.rs`: `GAME_VERSION` mostrata nel menu Esc; aggiornare a ogni rilascio.
- `scripts/gen_voci_abitanti.py`: clip delle prime battute degli abitanti e dei congedi (Piper locale), `voice::greeting_clips`/`farewell_clip`.
- `docs/voci/` + `scripts/export_voci.py`: battute per creare le voci; rigenerare dopo aver cambiato `dialoghi.txt`.
- `src/ui_style.rs`: palette, targhette (`plate`) e tema egui condiviso (stesso stile dei dialoghi).
- `src/ai/`: dialoghi con IA SOLO LOCALE (Ollama su loopback, niente TLS/cloud; ADR 0005): `ollama.rs` client, `persona.rs` prompt da `assets/ai/<nome>.txt`, `tts.rs` voce con Piper locale (ADR 0006), `stt.rs` microfono con whisper.cpp locale (ADR 0022), `ui.rs` chat; stato `AppState::Chat`.
- `src/main_menu.rs`, `src/settings_ui.rs`, `src/save.rs`: menu principale, impostazioni e salvataggi di testo (ADR 0025).
- `src/character.rs`: statistiche, classi, portata ed effetti (ADR 0024).
- `src/skills.rs`: abilità che crescono con l'uso, livello, Nodi d'Oro e talenti (`train`, `reward`, `rank`; ADR 0038); pannello in `character.rs`.
- `src/vitals.rs`: vigore e magia (`Vitals`, `spend`); barre in `hud.rs` (ADR 0039). `src/thievery.rs`: scasso dei bauli (`AppState::Lockpick`) e borseggio (`F`/`G` alle spalle).
- `src/magic.rs`: cinque incantesimi (`Z` sceglie, `Q` lancia), dardi, lume, scudo (`ward_factor`; ADR 0041).
- `src/house.rs`: la casa del giocatore sul Poggio del Naufrago (terreno fisso, legname e pietra col tasto `R`, due tappe, piazzamento `casa-naufrago` in memoria; ADR 0042).
- `src/archery.rs`: la Prova dell'Arco, tre anelli lungo il ruscello da centrare in fila (`Trial`; ADR 0043).
- `src/island_life.rs`: casse delle mareggiate, bottino che torna, banditi del pedaggio (ADR 0046); voci e luoghi esplorati in `places.rs`. `src/island_events.rs`: le cose nel mondo degli eventi raccontati (relitto, bottiglia, farfalle, lampione, ronda, torre, serpente di luce, nave fantasma; ADR 0053).
- `src/awakening.rs`: il risveglio sulla spiaggia, creazione del personaggio nella scena (età, indole, nome; `AppState::Awakening`; ADR 0047); Ettore che scende dal faro. Battute per età `> [giovane] …` nelle missioni.
- `src/cutscene.rs`: scene raccontate (dissolvenza e righe) e viaggio alla fine; la partenza dal Prologo (ADR 0048). `src/lanterns.rs`: la Sera delle Lanterne (piazza piena, lanterne); la Porta Sommersa in `places.rs`. `src/side_scenes.rs`: scene delle secondarie con persone sul posto (duello e strada del Sud, Silvia alla grotta, il paese al molo, predoni del mulino, cena alla locanda; ADR 0052); Camera di Risonanza e grata del pozzo in `places.rs`; casa di pietra (`house::STONE_HOUSE`); tappe aggiunte e migrazione dei salvataggi in `missions::MOVES`.
- `src/diving.rs`: l'apnea (talento attivo del Nuoto, `Ctrl` in acqua profonda), il fiato e la cassa sommersa al largo del relitto (ADR 0055). Legnaia e porticina sono piazzamenti (`buildings::model`); le carte da sfilare in tasca agli abitanti in `karma::POCKET_CLUES`; `tempo: <ora> estate` in `missions::jump`.
- `src/crossing.rs`: la traversata (ADR 0054): due arrivi all'Approdo delle Brume (molo e fonte a gradoni sotto la quercia), la via sotto il mare nei due sensi (gradini oltre la Porta Sommersa, fonte sul continente).
- `src/companions.rs`: compagni Silvia e Guardia Dino (seguono, combattono, si feriscono e si ritirano; viaggiano con te sul continente, `Traveler`; ramo in `villagers::villager_routine`; battute in `assets/world/compagni.txt`; sezione `[compagni]` nei salvataggi; ADR 0049). Armature in `inventory.rs`, Scatto in `player.rs`, Disarmo in `combat.rs`, Calma in `magic.rs`.
- `src/sfx.rs`: suoni sintetizzati di incantesimi, anelli e serrature (`sfx::play`); voci delle battute del Prologo da `scripts/gen_voci_prologo.py` (`voice::line_clip`, ADR 0050). Battute del gioco fuori dalle missioni in `assets/world/battute.txt` (`quests::bark_line`, `[tutti]` per ogni abitante; i fumetti trovano da soli la voce); lessico di pronuncia di Piper in `assets/audio/voci/pronuncia.txt`.
- `src/karma.rs` + `assets/world/segreti.txt`: karma, fiducia per abitante, regali e segreti confidati (`deed`, `trust`; ADR 0040); sblocchi, indizi (`CLUES`) e valori (`deed_as`, `praise`; ADR 0045); scheda «Volti e segreti» in `menu.rs`.
- `src/tailing.rs`: pedinare un abitante la sera (`pedina:` in `abitanti.txt`, fase `Secret` della routine; ADR 0045).
- `src/containers.rs`: inventari dei mobili (`AppState::Container`, capienze, contenuto iniziale; ADR 0023).
- `src/shell.rs`: shell dei trucchi (tasto `\`, `AppState::Shell`, `Cheats` non salvata; ADR 0021).
- `src/interiors.rs`: arredi, solai e scale degli edifici (ADR 0019), porte e luci in `world_props.rs`, spiazzi in `terrain.rs`, letti in `abitanti.txt` (`casa:`).
- `src/village.rs` e `src/buildings.rs`: villaggio murato (mura da piazzamenti `muro`, cancelli, edifici procedurali con parti per materiale PBR e UV in metri; ADR 0007, ADR 0016; dati in `assets/world/villaggio.txt`).
- `src/villagers.rs`: abitanti e guardie (turni, `Phase::Rest` alla caserma, posti calcolati dai cancelli e dai camminamenti; ADR 0013) (dati in `assets/world/abitanti.txt`, persona IA in `assets/ai/`), routine e percorsi sulle strade (ADR 0008).
- `src/shop.rs`, `src/equipment.rs`, `src/inventory.rs`, `src/combat.rs`: negozi con le monete, armi/scudo/arco impugnabili e frecce (ADR 0009).
- `src/bells.rs`: campana della chiesa sintetizzata e rintocchi alle ore; porto, faro e molo in `buildings.rs`/`world_props.rs` (ADR 0010, `scripts/gen_porto.py`).
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
