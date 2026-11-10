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
| 5.w2 — Interazioni e appoggio | Oggetti a parete con lato d'uso, dialoghi solo col mirino, appoggio a terra esatto all'avvio, pannello F4 stabile | `v0.0.13.0` |
| 5.m — Multipiattaforma | Windows e macOS (CI), impostazioni per piattaforma, piazzamenti incorporati | `v0.0.14.0` |
| 6.1 — IA locale (testo) | Dialoghi con Ollama solo in locale: "Parliamo d'altro…", repliche suggerite e testo libero, interruttore nel menu | `v0.0.15.0` |
| 6.2 — IA locale (voce) | Risposte dei dialoghi IA lette con Piper in locale, voce per personaggio, `gen_tts_voci.py` locale | `v0.0.16.0` |
| 7.1 — Villaggio | Fontevivo: mura, cancelli, selciato, case, chiesa, cartelli, nome sulla mappa | `v0.0.17.0` |
| 7.2 — Abitanti | 6 abitanti e 4 guardie con saluto scritto, chat IA e voce, routine giornaliera | `v0.0.18.0` |
| 7.3 — Mercato e armi | Negozi con le monete, armi/scudo/arco, mercato in piazza, campi | `v0.0.19.0` |
| 7.4 — Porto e campane | Strada del sud, faro, porticciolo con molo e barche, campane della chiesa | `v0.0.20.0` |
| 8.1 — Calendario (Fase 1) | Data dal 21 marzo anno 0, stagioni, sole e orari stagionali, velocità del tempo; mani ai personaggi, abitanti sul molo | `v0.0.21.0` |
| 8.2 — Stagioni visibili | Tinte di erba e chiome, fiori, neve a terra/tetti/alberi, foglie che cadono, luce stagionale | `v0.0.22.0` |
| 8.3 — Guardie e mura | Guardie su tre turni, casa delle guardie, camminamenti e scale sulle mura, guardie di ronda | `v0.0.23.0` |
| 8.4 — Meteo | Sereno, nuvoloso, pioggia, temporale, nebbia e neve per data, temperatura, vento, particelle, neve a terra, suoni locali | `v0.0.24.0` |
| 9.1 — Personaggi veri | Modelli umani con scheletro CC0, posa di riposo, oggetti alle mani | `v0.0.25.0` |
| 9.2 — Mondo PBR | Texture fotografiche CC0 su terreno ed edifici, abeti veri, rendering HDR con ombre a cascata e occlusione | `v0.0.26.0` |
| 9.2.1 — Rifinitura PBR | Tende, ferro e legno ovunque, sotto-menu impostazioni grafiche | `v0.0.27.0` |
| 9.3 — Animazioni | Camminata, corsa, salto, attacchi, colpiti e morte per tutti i personaggi | `v0.0.28.0` |
| 9.4 — Inventario e FPS | Inventario con categorie, ricerca e peso, barra a 10 tasti assegnabile con icone, VSync e limite FPS | `v0.0.29.0` |
| 10.1 — Mura percorribili | Camminamento continuo, torri attraversabili, ponti sui cancelli, ronde | `v0.0.30.0` |
| 10.2 — Interni degli edifici | Case e chiesa visitabili con più piani, scale, sottotetto, arredi; abitanti a letto | `v0.0.31.0` |
| 10.3 — Chiavi e shell | Abitanti che salgono ai piani, serrature e chiavi, shell dei trucchi | `v0.0.32.0` |
| 10.4 — Microfono locale | Riconoscimento vocale nella chat con whisper.cpp, solo in locale | `v0.0.33.0` |
| 10.5 — Contenitori e faro | Inventari di casse, armadi, botti e scaffali; faro visitabile | `v0.0.34.0` |
| 10.6 — Statistiche e peso | Statistiche del personaggio, limite di peso, zaini ed effetti | `v0.0.35.0` |
| 10.7 — Menu e salvataggi | Menu principale, personaggi, salvataggi di testo permanenti | `v0.0.36.0` |
| 11.1 — Terreno e acque | Alture, scogliere, spiagge, ruscello, laghetto, onde, sentieri | `v0.0.37.0` |
| 11.2 — Vegetazione | Boschi, cespugli, rocce, fiori ed erba alta a riquadri | `v0.0.38.0` |
| 11.3 — Suoni e animali | Suoni d'ambiente sintetizzati, passi per superficie, animali, latifoglie CC0 | `v0.0.39.0` |
| 11.4 — Paese vissuto | Fumo dai comignoli, torce sulle mura, carretti, botti, casse, panni e reti | `v0.0.40.0` |
| 11.5 — Luoghi da scoprire | Campi, fattoria, mulino, torre, rovine, grotta, banditi, santuario, nomi sulla mappa | `v0.0.41.0` |
| 12.1 — Correzioni R1 | Crash F4, ora locale, campana, piazza, collisioni degli animali, caduta | `v0.0.42.0` |
| 12.2 — Scale e torri | Rampe sotto i gradini, fori nei solai, faro più largo, prova di camminata automatica | `v0.0.43.0` |
| 12.3 — Suoni e acqua | Passi nuovi, brusio, mare, nuoto, schizzi e pesci | `v0.0.44.0` |
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
- **Asset**: personaggi glTF CC0 Quaternius fatti (ADR 0015); mondo e materiali PBR fatti (ADR 0016); animazioni fatte (ADR 0017).
- **Fuori scope v1**: multiplayer. Web build (WASM) da valutare dopo la stabilità su Linux.

## Decisioni architetturali

Vedi [docs/decisions/](decisions/README.md) (ADR-lite). Stack: Rust + Bevy 0.20.1, Avian 0.8, bevy_egui 0.43.

## Convenzioni di sviluppo

- Ogni task è un commit atomico con messaggio `feat(area): descrizione` (o `fix`/`chore`).
- Prima di ogni commit: `cargo fmt` e `cargo clippy`.
- Verifica manuale con `cargo run`, controllando i log `[INPUT]`, `[QUEST]`, `[CURSOR]`, ecc.
- Gli asset binari pesanti non vanno nel repo (usare LFS o release).
