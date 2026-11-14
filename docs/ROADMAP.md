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
| 12.4 — Personaggi e combattimento | Nemici che attaccano, banditi di notte, andatura, lanterne, prompt IA | `v0.0.45.0` |
| 12.5 — Interfaccia | Icone degli oggetti a terra, barra rapida nell'inventario, R e Tab nei contenitori | `v0.0.46.0` |
| 12.6 — Voci | Saluti degli abitanti e congedi con voce, generati in locale con Piper | `v0.0.47.0` |
| 12.7 — Rifiniture | Guardie, ombre delle luci, case sul pendio, prima persona | `v0.0.48.0` |
| 12.8 — Tetti | Fessura fra muro e falda chiusa, prova di chiusura | `v0.0.49.0` |
| 12.9 — Mura | Cunei fra mura e torri d'angolo chiusi | `v0.0.50.0` |
| 12.10 — Scuri e camini | Scuri apribili nelle case, camino sopra il focolare | `v0.0.51.0` |
| 12.11 — Focolari e scale | Focolare acceso/spento con fumo, scuri a orario, scale delle mura fuori dalle torri | `v0.0.52.0` |
| 12.12 — Grotta e spada | Grotta con corridoio e camera, arma riposta in corsa | `v0.0.53.0` |
| 12.13 — Rifiniture | Rampa della grotta, spada sulla schiena in corsa, stati salvati di focolari e scuri | `v0.0.54.0` |
| 12.14 — Scuri e gallerie | Scuri aperti dagli abitanti con gesto, gallerie sotterranee della grotta | `v0.0.55.0` |
| 13.0 — Beta test: strumenti | Pilota automatico, stato JSON, verifiche, F12, beta_check con foto di riferimento | `v0.0.56.0` |
| 13.1 — Beta test: bug bloccanti | Morte unica, rete sotto il mondo, imbocco delle gallerie, Viandante al banco, scuri della casetta | `v0.0.57.0` |
| 13.2 — Beta test: primo giro dei sistemi | Turni delle ronde e del faro, nuoto, banditi che non ricompaiono subito, nuovi copioni | `v0.0.58.0` |
| 13.3 — Beta test: prove automatiche e grafica | Pilota senza finestra, a casse mute e senza input, a 720p–4K con FPS; ombre col cielo coperto, pioggia e neve, scala dell'interfaccia, insegne, IA locale col contesto giusto | `v0.0.59.0` |
| 13.4 — Beta test: audio e camera | Audio posizionale (ADR 0032), passi sull'erba, voci che non saturano, camera fuori dalle gronde, intoppi segnalati dal pilota | `v0.0.60.0` |
| 14.1 — Continente del Nord: regione esplorabile | Regioni separate e viaggio con Gaspare (ADR 0033); Piana delle Brume: approdo, bosco di querce e betulle, Grande Lago, Fontebruma, i 16 luoghi | `v0.0.61.0` |
| 14.2 — Continente del Nord: abitanti | I 12 di Fontebruma, Olmo, Nereo, le tre famiglie isolate, Barnaba: case, orari, voci, personalità IA, negozi | `v0.0.61.0` |
| 14.3 — Isola: storia principale del Prologo | Motore di missioni a tappe da file (ADR 0034); Il Nome sulla Sabbia, Le Voci delle Pietre Antiche, Sotto l'Arco dei Sei Gradoni, Oltre il Velo del Nord; il continente si apre con la storia | `v0.0.62.0` |
| 14.4 — Isola: bivi e missioni secondarie | Bivi di «Denaro Rubato» (ADR 0035); undici missioni secondarie degli abitanti (ADR 0036); restano sei secondarie che chiedono sistemi nuovi | `v0.0.62.0` |
| 14.5 — Isola: eventi, luoghi e viaggio veloce | Undici eventi casuali, altre tre secondarie (14 in tutto), luoghi scoperti sulla mappa e viaggio veloce (ADR 0037) | `v0.0.62.0` |
| 14.6 — Isola: abilità e livelli | Sedici abilità che crescono con l'uso (undici allenabili), livello del personaggio, punti statistica, Nodi d'Oro e talenti a ranghi (ADR 0038) | `v0.0.62.0` |
| 14.7 — Isola: vigore, magia, scasso e borseggio | Barre di magia, vita e vigore sopra la barra degli oggetti; serrature dei bauli col gioco dei perni; borseggio e borseggio inverso; missione QS-16 (ADR 0039) | `v0.0.62.0` |
| 14.8 — Isola: karma e segreti | Karma globale, fiducia per abitante, regali, i 26 segreti confidati agli amici, scheda «Volti e segreti» nel diario, prezzi e IA locale che ne tengono conto (ADR 0040) | `v0.0.62.0` |
| 14.9 — Isola: incantesimi | Cinque incantesimi che spendono la magia e allenano Magie, scelti con Z e lanciati con Q (ADR 0041) | `v0.0.62.0` |
| 14.10 — Isola: la casa del giocatore | Terreno fuori dalle mura, legname e pietra da raccogliere, fondamenta e casa visitabile con deposito, missione QS-17 (ADR 0042) | `v0.0.62.0` |
| 14.11 — Isola: la Prova dell'Arco | Tre anelli lungo il ruscello da centrare in fila, missione QS-14 di Silvia (ADR 0043) | `v0.0.62.0` |
| 15.1 — Prologo: oggetti di trama e scelte | Quattordici oggetti di trama («Ricordi»), scelte con effetti nelle missioni secondarie, rito che ferma i banditi (ADR 0044) | `v0.0.63.0` |
| 15.2 — Prologo: segreti, indizi e valori | Sblocchi dei 26 segreti, quattro indizi nei mobili, valori personali degli abitanti, guardie che vedono i furti, fama nei saluti (ADR 0045) | `v0.0.63.0` |
| 15.3 — Prologo: mondo vivo | Voci «?» sui luoghi, luoghi esplorati, bottino che torna, casse delle mareggiate, pedaggio del ponte, risposte con requisito (ADR 0046) | `v0.0.63.0` |
| 15.4 — Prologo: il risveglio | Creazione del personaggio nella scena della spiaggia: età nello Specchio della Marea, indole dai rottami, nome a Gaspare (ADR 0047) | `v0.0.63.0` |
| 15.5 — Prologo: storia principale e partenza | Salti di tempo e meteo di scena, Disco di Tiro, stele del Cenote, Quipu Maestro e Porta Sommersa, doni della Sera delle Lanterne, partenza per la via del mare o delle profondità con scena raccontata (ADR 0048) | `v0.0.63.0` |
| 15.6 — Prologo: sistemi rimasti | Armature, Scatto e Disarmo, compagni Silvia e Dino, letto di casa, magia insegnata da Agnese e incantesimo Calma (ADR 0049) | `v0.0.63.0` |
| 15.7 — Prologo: voci, suoni e ronde | Voci di tutte le battute di missioni e confidenze (Piper locale), suoni di magia, arco e serrature, guardie di ronda che si fermano a parlare (ADR 0050) | `v0.0.63.0` |
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
