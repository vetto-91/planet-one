# Changelog — Planet One

Formato versione: `vMAJOR.MINOR.FEATURE.BUGFIX` (vedi [docs/VERSIONING.md](docs/VERSIONING.md)).

## Non rilasciato

- **Calendario (Fase 1)**: data di gioco dal 21 marzo anno 0 (mesi reali), stagioni, sole stagionale con alba e tramonto che cambiano nell'anno,
  velocità del tempo ×1/×2/×4/×8 nel menu Esc, "Aspetta" che avanza anche i giorni; cancelli, lampioni, faro, lanterne e Viandante seguono alba e tramonto
  (ADR 0011).

- **Correzioni**: gli abitanti sul molo (Gaspare) ora camminano sul piano del molo invece di sprofondare nel fondale; tutti i personaggi umani (giocatore,
  Viandante, abitanti, guardie, ladro) hanno le mani (cubetti color pelle ai lati del corpo).

## v0.0.20.0 — Porto, faro e campane (2026-11-08)

- **Strada del sud** dal cancello al mare, **faro** (luce girevole di notte, guardiano Ettore) e **porticciolo** con molo calpestabile, tre barche, casetta dei pescatori e il
  pescatore Gaspare; cartelli e lampioni lungo la strada (ADR 0010).
- **Campana** della chiesa: suono sintetizzato in locale, rintocchi alle ore piene dalle 7 alle 21 con volume secondo la distanza.
- **Correzioni**: di notte gli abitanti nascosti non mostrano più la targhetta; le bancarelle del mercato, la bottega dell'arciere, i pagliai e gli spaventapasseri
  ora compaiono (in v0.0.19.0 mancavano per una lista di tipi incompleta, ora c'è un test).

## v0.0.19.0 — Mercato, negozi, armi e campi (2026-11-08)

- **Negozi e mercato**: pannello di acquisto con le monete (30 iniziali, i nemici ne lasciano); venditori: Viandante (pozioni), armaiolo Ferruccio, arciera Silvia e tre bancarelle in
  piazza (pozioni, mele, pesce). Nuovi oggetti: spada corta, spada lunga, ascia, scudo, arco, frecce, mela, pesce (ADR 0009).
- **Armi e scudo**: si impugnano dalla barra, con attacco e statistiche proprie; l'arco spara frecce verso il mirino; lo scudo riduce i danni. `PLANET_ONE_GIVE=1` per provare.
- **Campi** di grano e orto fuori dalle mura, pagliai, spaventapasseri; nuovi tipi F4 (`arcere`, `bancarella-*`, `campo`, `orto`, `pagliaio`, `spaventapasseri`).
- Cancelli: solo due ante piene; i bottoni stanno sui piloni (dentro e fuori) e si usano col mirino; le guardie stanno accanto ai cancelli.

## v0.0.18.0 — Abitanti e guardie, correzioni al villaggio (2026-11-08)

- **Abitanti e guardie** di Fontevivo (6 + 4) con saluto scritto e chat con la IA locale (carattere e voce propri), routine giornaliera (lavoro, piazza la sera,
  in casa di notte; le guardie ai cancelli con lanterna notturna) (ADR 0008).
- Cartelli: il palo sta dietro la tabella. Cancelli: ante piene (si chiudono davvero, con collisione) e porticina centrale; ogni anta si apre col mirino.
- La pozione è ora sul tavolo dentro la casetta del Viandante. Mura di pietra più chiare.
- La quest del Viandante avanza solo con il Viandante (non con gli altri abitanti).

## v0.0.17.0 — Villaggio di Fontevivo (2026-11-08)

- **Villaggio di Fontevivo**: mura con torrette (dai piazzamenti `muro` di F4), due cancelli (Porta Nord e Sud) con battenti che si aprono di giorno e si
  chiudono di notte (varco pedonale sempre libero), strada a selciato e stradine, piazza con pozzo, chiesa con campanile, case di vario tipo,
  locanda, forno, lampioni e cartelli con il nome; nome sulla mappa e banner d'ingresso; terreno spianato dentro le mura (ADR 0007).
- F4: nuovi tipi (`casa-*`, `locanda`, `fabbro`, `forno`, `stalla`, `chiesa`, `pozzo`, `cartello`); gli alberi non vengono creati dentro le mura.
- Il mare ora si disegna sopra sabbia e strade (anche dalla mappa). Script: `gen_villaggio.py`, `make_selciato_texture.py`, `make_cartelli.py`.

## v0.0.16.0 — Voce IA locale con Piper, fase 2 (2026-11-08)

- Voce IA (fase 2): le risposte dei dialoghi IA sono lette ad alta voce con Piper **in locale** (programma esterno, senza rete); voce per personaggio,
  casella "Voce IA" e scelta della voce nel menu, `PLANET_ONE_TTS_MOCK=1` per provare (ADR 0006).
- `scripts/setup_ai.sh|ps1 --tts` installa Piper e le voci italiane con consenso esplicito; `scripts/gen_tts_voci.py` ora usa Piper locale (niente più edge-tts online).

## v0.0.15.0 — Dialoghi con IA locale, fase 1 (2026-11-08)

- Dialoghi con IA **solo locale** (Ollama, `gemma4:e4b` configurabile), fase 1 testuale: opzione "Parliamo d'altro…" a fine dialogo, chat con
  tre repliche suggerite e testo libero, interruttore e stato nel menu Esc; i dialoghi e le quest scritti hanno la precedenza (ADR 0005).
- CI di release solo manuale, con una casella per Windows e una per macOS (meno minuti a pagamento).
- `scripts/setup_ai.sh` / `setup_ai.ps1`; persona del Viandante in `assets/ai/viandante.txt`; `PLANET_ONE_AI_MOCK=1` per provare senza modello.

## v0.0.14.0 — Windows e macOS (2026-11-08)

- Porting Windows e macOS: archivi costruiti da GitHub Actions (Windows x86_64 `.zip`, macOS universale `.tar.gz`), impostazioni nella cartella di
  configurazione di ogni piattaforma, cursore `Confined` su Windows, niente console in release su Windows (ADR 0004).
- Piazzamenti (alberi, lampioni, fiori) incorporati nel gioco: compaiono anche nei pacchetti distribuiti.

## v0.0.13.0 — Interazioni col mirino e oggetti a terra (2026-11-07)

- Interruttore luce, pulsante serranda e maniglie interne delle finestre usabili col mirino da dentro (e non da fuori): ogni oggetto a parete ha il suo lato d'uso.
- Si parla con il Viandante (e gli altri personaggi) solo guardandolo col mirino.
- Oggetti piazzati (alberi, lampioni, fiori…) appoggiati subito al terreno vero all'avvio (prima restavano a y = 0,5 finché non si ricostruivano).
- Pannello F4 a larghezza fissa (niente tremolio sopra "Ruota"); maniglie delle finestre invisibili.

## v0.0.12.0 — Casetta da dentro, finestre a vetro scorrevole, spiaggia (2026-11-07)

- Casetta: porta, interruttore, stufa, serranda e finestre di nuovo usabili da dentro (gli occhi del giocatore erano dentro il collider del tetto);
  finestre a due stadi da dentro (alza il vetro, poi apri gli scuri); apertura del banco e vani delle finestre senza collider pieni, quindi il
  Viandante si vede (etichetta) e gli si parla dal finestrone e dai vetri; serranda abbassata e scuri chiusi bloccano la vista.
- Lampioni: la luce non attraversa più le pareti (ombre per i 3 più vicini).
- Isola: il mare è solo attorno all'isola (niente acqua nelle depressioni interne), barriera di riva che segue la costa, spiaggia con la texture
  sabbia che sfuma nel prato.
- F3: il collider del terreno non viene più disegnato per intero (60 → 7 FPS): griglia sparsa e paletti rossi sulla riva attorno al giocatore.

## v0.0.11.0 — Isola, debug con mirino, oggetti ruotabili (2026-11-07)

- Isola: terreno esteso a 300 m con costa irregolare, mare attorno e muro invisibile sulla riva; il quadrato della heightmap resta al centro.
- Debug F4: mirino al centro per piazzare (Ctrl tenuto libera il mouse per il pannello), rotazione con freccia rossa (trascina, R, strumento Ruota),
  niente compenetrazioni (anche tra aree), "Ridistribuisci", pannello sopra le linee e a scorrimento.
- Oggetti piazzati affondano nel terreno (niente basi sospese sui rilievi).
- Niente interazioni, dialoghi, etichette o barre vita di NPC attraverso i muri.

## v0.0.10.0 — Strumenti debug, oggetti veri, musica notturna (2026-11-07)

- Viandante: di notte, se la luce è accesa si alza quasi subito, chiude prima porta/finestre aperte, spegne la luce e torna a letto.
- Casetta: tetto più spesso (sottofaccia e bordo di gronda), niente luce tra tetto e muro.
- Mappa: spostamento con frecce/WASD e trascinamento, C ricentra sul giocatore.
- Debug F4: cursore libero in 3ª/1ª persona (tasto destro = guarda), poligono con OK, rettangolo a 4 angoli,
  distribuzione regolare, allineamento a griglia/linea, anteprima del cerchio, strumenti Gomma e Sposta.
- Musica notturna (`Lanterns_In_The_Grove`) dal tramonto all'alba con dissolvenza incrociata.
- Oggetti veri (alberi, fiori, panchina, lampione con luce notturna) creati da `debug/placements.txt` (`world_props.rs`).

## v0.0.9.0 — Mappa, debug piazzamenti, casetta rifinita (2026-11-07)

- Mappa in gioco (M): vista dall'alto della scena reale (camera ortografica, nord in alto), bussola in alto a sinistra,
  segnalino del giocatore, zoom con rotella e +/- (`MapView`).
- Modalità debug piazzamenti (F4): segnaposto singoli, in gruppo o ad area, aggiunti/eliminati dalla mappa o in 3D,
  salvati in `debug/placements.txt` e riletti all'avvio.
- Casetta: pulsante della serranda solo all'interno (parete +X, vicino allo spigolo), scuri spessi e più larghi, vetro
  alle 3 finestre, giunti muro/fondamenta chiusi (niente luce dal retro), luci con bias ridotto.
- Il Viandante chiude porta e finestre lasciate aperte dopo un ritardo casuale (45 s – 3 h di gioco); sera e mattina
  controllano anche le finestre; di notte si alza entro 1 h di gioco se qualcosa viene aperto.
- `scripts/sync_drive.sh --copy` (nessuna cancellazione su Drive).

## v0.0.8.0 — Interazione col mirino, casetta rifinita, routine del Viandante v2 (2026-11-07)

- Prima persona: occhi a ~1,57 m da terra. Chip delle coordinate con anche la quota Y dei piedi.
- Targhette di Viandante, Ladro e nemici solo entro 10 m dal player; i fumetti (anche "Zzz…") stanno sotto il nome.
- Interazione: gli oggetti (porta, finestre, stufa, interruttore, serranda) si usano solo **guardandoli col mirino**
  (entro 3 m, non dietro un muro) e hanno la priorità sull'NPC; il Viandante si parla solo entro 3 m.
- Casetta più bassa (affondata di 30 cm). Luci interne con ombre (la luce non attraversa i muri).
  Nuovi oggetti: interruttore della luce, serranda del negozio con scritta CHIUSO, 3 finestre apribili con imposte.
- Routine del Viandante v2: stufa 6–10 e 17–21 (non toccata se l'ha cambiata il giocatore), mattina (luce, porta,
  stufa, serranda), sera (stufa, serranda, luce, porta, letto).
- Voci: clip troppo basse normalizzate (`scripts/normalize_voci.py`), guadagno ×1,4 e musica abbassata mentre parla un personaggio.
- Variabili d'ambiente di debug (`PLANET_ONE_POS`, `PLANET_ONE_LOOK`).

## v0.0.7.0 — Colonna sonora, casetta esplorabile, voci (2026-11-07)

- Colonna sonora in loop (`audio/music/legacy_of_the_first_light.ogg`) con slider "Volume musica" nel
  menu Esc; il volume è salvato in `~/.config/planet-one/settings.txt`.
- Casetta-chiosco (mesh con le 4 viste della foto, tetto a padiglione, apertura del banco trasparente)
  al posto del Viandante, che ora fa da negoziante dentro il chiosco; l'insegna dice "GUSTO DEL BOSCO".
- NPC e casetta appoggiati al terreno (`SnapToGround`): l'NPC non è più mezzo sepolto.
- `docs/voci/viandante.txt` e `docs/voci/ladro.txt` con tutte le battute, generati da `scripts/export_voci.py`.
- Voci dei personaggi nel gioco (`assets/audio/voci/`, mappa in `voci.txt`): partono con ogni riga di dialogo,
  il congedo del Viandante e la battuta del ladro; la riga con direzione/distanza del ladro è composta da due
  clip. Slider "Volume voci" nel menu Esc.
- Casetta esplorabile 4x6 m: fondamenta e scale di pietra (texture generata `pietra.png`), porta apribile (E),
  interni (letto, cucinino con stufa, tavolo con sedie, baule, scaffale, lanterna), stufa accendibile con camino e fumo.
- Camera in terza persona che non attraversa pareti e tetto della casetta.
- Routine del Viandante: dopo le 21 accende la stufa, chiude la porta e va a dormire; alle 6 si sveglia,
  spegne la stufa, apre la porta e torna al banco. Da addormentato risponde "Zzz…".
- Versione del gioco in basso a destra nel menu Esc (`src/version.rs`).

## v0.0.6.0 — Rifiniture: diario, giorno/notte, skin, UI condivisa (2026-11-06)

- Diario delle quest (J) con tab "Da fare" / "Completate" e quest espandibili; pannello TAB
  alimentato dalla stessa voce.
- Quest in cartelle: logica in `src/quests/<quest>/`, testi modificabili a mano in
  `assets/quests/<quest>/dialoghi.txt` (ricaricati dal Reset scena, fallback incorporato).
- Dialoghi: E/Invio per avanzare, scelta Sì/No, il Viandante saluta se ci si allontana;
  il ladro apostrofa il giocatore quando lo vede (fumetti sopra la testa).
- Quest attivata da sola se il ladro muore prima di parlare col Viandante.
- Texture erba low-poly sul terreno e cielo a nuvole (panoramica convertita in cubemap a runtime).
- Ciclo giorno/notte (1 h = 60 s) con sole e luna, luce e cielo che cambiano; tasto T "Aspetta".
- Skin per player, ladro e viandante (mesh a UV per faccia); targhette "Viandante"/"Ladro".
- Tema UI condiviso (`ui_style`): menu, diario e HUD coerenti; chip più leggibili, coordinate
  su una riga, banner quest più in basso.
- Fix: A/D invertiti; luna meno luminosa; la quest si completa solo se il dialogo finale è letto.
- Repository: `.gitignore` completo, `scripts/sync_drive.sh`, documentazione allineata a `main`.

## v0.0.5.0 — Fase 5: Vertical slice RPG "Denaro Rubato" (2026-11-05)

- Quest completa in memoria: NPC Viandante allo spawn -> dialogo -> accetta
  "Denaro Rubato" (overlay titolo 3 s). Ladro rosso (QuestThief, 30 HP) in
  nord-ovest: ucciderlo (morte definitiva, no respawn) -> spawn 12 monete
  gialle (QuestCoin) -> raccolta automatica entro 2 m -> torna dal Viandante
  -> dialogo finale -> "Missione completata".
- Bussola sempre visibile in alto al centro: coordinate x,z + freccia rossa
  verso il target corrente (ladro / moneta piu' vicina / NPC).
- TAB tenuto: pannello obiettivi con checklist ([✓]/[ ] e conteggio monete).
- Overlay titolo quest in alto (banner oro).
- Reset scena resetta anche QuestState (full reset).
- Nessun salvataggio su file (come da piano).
- Test: quest accettata via log [QUEST], bussola+TAB verificati via screenshot, 0 panic.

## v0.0.4.9 — Avvio fullscreen + pulsante "Esci dal gioco" (2026-11-05)

- Avvio a schermo intero di default (WindowMode::Fullscreen(Current, Current));
  F11 alterna fullscreen/finestra. Cursore catturato subito in Playing.
- Menu Esc: nuovo pulsante "Esci dal gioco" → AppExit::Success (chiusura pulita,
  nessun kill forzato: pronto per salvataggi futuri).
- Test: avvio fullscreen 4480x1440 verificato via screenshot; 0 panic.

## v0.0.4.8 — Cursore in fullscreen + menu + pitch 180° (2026-11-05)

- apply_cursor_for_mode: cattura cursore (Locked+nascosto) SOLO in fullscreen
  E Playing; in Paused (menu Esc) o a finestra il cursore torna visibile e libero.
- warp_cursor_to_center: in fullscreen+Playing riporta il cursore al centro ogni
  frame → su X11 (Confined) la rotazione yaw resta infinita (360°), i delta
  continuano ad arrivare anche al bordo.
- Pitch verticale in 1a persona: ±1.54 rad (~±88°, quasi 180°): al massimo in
  alto (cielo) o in basso (terreno) la visuale si ferma, niente ribaltamento pg.
- Log [CURSOR] per stato cattura; [INPUT]/[EVENT] invariati.
- Test: fullscreen cattura ON, menu Esc cursore libero, yaw 360, 0 panic.

## v0.0.4.7 — Cursor capture in fullscreen + log completo input (2026-11-05)

- F11 fullscreen: cursore nascosto + `CursorGrabMode::Locked` (tutti gli input
  restano nel gioco). A finestra: cursore visibile e libero (esce e interagisce
  col sistema). Su X11 Locked ricade su Confined (max disponibile).
- Nuovo sistema `log_input_events`: logga TUTTI gli eventi con prefisso
  [INPUT]/[EVENT] — key press/release, mouse button, motion, wheel, cursor moved.
  Utile per debug via grep: `cargo run 2>&1 | grep "\[INPUT\]"`.
- Test: log 16 eventi catturati (tasti W/Space/F11, click, movimento mouse),
  fullscreen 4480x1440 con cursore bloccato, ritorno 1280x720 libero — 0 panic.

## v0.0.4.6 — F11 fullscreen + naso allineato al crosshair (2026-11-05)

- F11: toggle fullscreen nativo (`WindowMode::Fullscreen(Current, Current)`)
  gestito dal SO (cattura input mouse/tastiera). Aggiunto alla lista comandi
  del menu Esc. Testato: 1280x720 -> fullscreen -> 1280x720.
- PlayerVisual (naso) ora segue il crosshair:
  - 3a persona fermo: ruota verso la direzione della camera (yaw).
  - 3a in movimento: ruota verso la direzione di movimento (come prima).
  - 1a persona: rotazione = yaw+pitch della camera (corpo segue lo sguardo).
- Test: F11 on/off verificato via dimensioni finestra, rotazione naso
  sinistra/droite misurata via pixel gialli — 0 panic.

## v0.0.4.5 — Pitch coerente 3a/1a persona (2026-11-05)

- Alla transizione 3a->1a (V o RMB) il pitch viene azzerato: la mira parte
  orizzontale, niente salto verticale (prima: guardavi giu' in 3a e vedevi
  il cielo in 1a).
- In 1a: `rotation_x(-pitch)`, stesso segno dy in entrambe le viste:
  mouse giu' = guardi giu', mouse su = guardi su, coerente con la crosshair 3a.
- Hiatus del cursore: lasciato al SO (Ctrl+Alt+Invio full-screen cattura input).
- Test: transizione 3a(giu')->1a(orizzonte), poi 1a su/giu' — 0 panic.

## v0.0.4.4 — Fix prima persona e proiezione barra nemico (2026-11-05)

- Fix rotazione prima persona: `Quat::from_rotation_y(yaw) * from_rotation_x(pitch)`
  invece di `EulerRot::ZYX` (che metteva lo yaw sull'asse Z = visuale ruotata 90°).
- Mouse: letto via `CursorMoved` (delta) invece di `MouseMotion` raw: su X11 i
  device events non arrivano sempre, CursorMoved sì. In 1a persona dy invertito
  (mouse su = guarda su); in 3a persona comportamento storico invariato.
- Player nascosto in prima persona (sia corpo che mesh), riapparso in 3a.
- Fix proiezione barra nemico: `world_to_viewport` in Bevy 0.20 gia' restituisce
  pixel y-down — rimossa la doppia inversione Y che la spostava sotto il nemico.
- Test: 3a persona (orizzonte si sposta col mouse), 1a persona (mouse su = cielo,
  destra/sinistra = yaw corretto, player invisibile) — 0 panic.

## v0.0.4.2 — Fase 4.3: prima persona (RMB/V), barra enemy, barra player in basso (2026-11-05)

- Camera in prima persona VERA: la camera va dentro il player (stessa posizione
  e rotazione yaw/pitch), il pg non si vede. Crosshair centrato e sincronizzato.
- RMB tenuto → prima persona temporanea (rilascio = torna in terza persona).
- Tasto V → toggle lock prima persona (RMB non interferisce col lock attivo).
- Input RMB letto via MessageReader<MouseButtonInput> raw in PreUpdate-equivalente,
  per non essere filtrato da egui/ButtonInput.
- Barra vita player spostata in basso a sinistra, lunghezza dimezzata (110 px).
- Barra nemico sopra la testa: nome "enemy" + barra rossa proporzionale,
  proiettata world→viewport con Camera::world_to_viewport.
- Test: V lock prima persona verificato (0 pixel player = camera dentro il pg);
  RMB/release; movimento in prima persona — 0 panic.

## v0.0.4.0 / v0.0.4.1 — Fase 4: HUD, inventario, combattimento base (2026-11-05)

### v0.0.4.0 — HUD + inventario
- Barra vita in alto a sinistra (egui, colori dinamici verde/giallo/rosso).
- Crosshair centrale. Slot inventario in basso con icone colorate + quantita'
  e tooltip nome. Pickup raccoglibili (Pozione rossa, Moneta gialla) con
  raccolta automatica entro 2 m. Inventario in resource (sopravvive al reset
  scena). PlayerHealth { current, max } sul player.

### v0.0.4.1 — Combattimento base
- Attacco melee su click sinistro: cooldown 0.6 s, range 2.5 m, danno 4,
  target = nemico piu' vicino nel cono frontale (dot > 0.3).
- Nemico con EnemyHealth (20 HP):barra vita sopra il mesh (da fare in UI
  world), morte -> despawn + respawn dopo 3 s sul primo waypoint.
- Danno da contatto player<->nemico entro 1.2 m, ogni 0.8 s, iframe 0.3 s.
- Morte player (vita <= 0): respawn allo spawn con vita piena, camera resettata.
- Nessun attacco durante dialogo aperto.
- Test: click攻击 С Chamb; 0 panic.

## v0.0.3.0 — Fase 3: Entità (NPC, dialogo, nemico patrol) (2026-11-05)

- Nuovo modulo `entities.rs`: NPC (Viandante, blu) con dialogo lineare,
  inimigo (rosso) in patrol tra waypoint con lo stesso MoveAndSlide del player.
- Interazione: prompt "[E] Parla" quando nel range (5 m), tasto E apre il dialogo.
- Dialogo: pannello in basso stile RPG (egui), Enter/Space avanza, Esc chiude.
  Esc durante dialogo chiude il dialogo SENZA aprire il menu pausa.
- Movimento libero durante il dialogo (come richiesto).
- Reset scena include NPC e nemico (SceneNode).
- Test: dialogo aperto/avanzato/chiuso (4 righe), movimento durante dialogo,
  patrol nemico verificato via log — 0 panic.

## v0.0.2.5 — Fase 2.3: salto robusto su pendenze (multi-ray + coyote + hit-history + buffer) (2026-11-05)

- Ground-check multi-ray: 5 raycast (centro + 4 offset XZ) entro 1.5 m,
  normale minima normal.y >= 0.5: rileva il terreno anche su pendenze ripide.
- Coyote time alzato a 0.25 s + cronologia hit fisici di MoveAndSlide
  (last_ground_hit, 0.2 s): il salto resta valido anche se i ray non vedono
  il terreno durante la corsa in salita.
- Jump buffer 0.15 s: se premi Space poco prima di toccare terra, il salto
  parte appena diventi grounded.
- Snap al terreno solo entro 0.95 m (non in caduta libera).
- JUMP_SPEED 6.0 -> 7.5.
- Anti double-jump: buffer + coyote + hit consumati al salto.
- Test: salti in camminata e corsa ripetuti — 0 panic, salto sempre attivo.

## v0.0.2.4 — Fase 2.2: salto affidabile con coyote time (2026-11-05)

- Nuovo `GroundedState { grounded_now, coyote_left }` sul player.
- Coyote time 0.12 s: il salto resta valido 0.12 s dopo l'ultimo contatto
  reale, risolvendo i salti mancati in camminata/corsa sulle pendenze.
- Raycast ground-check esteso a 1.25 m (semi-altezza 0.85 + tolleranza 0.40)
  e filtro sulla normale (solo superfici con normal.y >= 0.6 contano come
  suolo: i muri non danno grounded).
- Anti double-jump: dopo un salto valido il coyote viene consumato.
- Tutto il movimento del player (ground-check, input, gravity, MoveAndSlide,
  facing) spostato in FixedUpdate: pipeline unica, delta stabile.
- JUMP_SPEED 6.0 → 7.0 per un salto visibilmente più alto.
- Camera resta in Update dopo il FixedUpdate.
- Test: salto da fermo, in camminata e in corsa (3 ripetizioni) — 0 panic.

## v0.0.2.3 — Fase 2.1: allineamento definitivo mesh/collider (2026-11-05)

- Fix definitivo: matrice collider costruita come heights[x][z] (riga=X,
  colonna=Z), trasposta rispetto alla prima versione: ora il wireframe F3
  copre esattamente tutta la mesh verde su tutto il terreno 100 m.
- Sonda numerica rimossa dopo la verifica (angoli + centro heightmap).
- Verifica: wireframe copre tutta la mesh, camminata 10 s stabile sul
  pendio, salto, menu Esc — 0 panic.

## v0.0.2.1 — Fase 2.1: character controller kinematico + fix collisioni terreno (2026-11-04)

- Player migrato da `RigidBody::Dynamic` a character controller cinematico
  Avian (`RigidBody::Kinematic` + `CustomPositionIntegration` + `MoveAndSlide`
  in `FixedUpdate`): elimina sprofondamenti e cadute nel vuoto sulle pendenze
  dell'heightfield a celle grandi (100 m / 256 = ~0.39 m).
- Gravità manuale (20 m/s²) in `movement`; salto solo se `Grounded`.
- Terreno allargato a 100 m (da 40 m); heightmap mappata su -1..+1 (grigio
  medio = 0) per avere colline sopra e sotto il livello base.
- Collider heightfield Avian corretto: matrice `heights_zx` (righe=Z, colonne=X)
  come richiesto da Parry, allineata alla mesh.
- Cielo procedurale in `Rgba8Unorm` (lineare) invece di Srgb: sRGB causava un
  "double gamma" che rendeva il cielo grigio; `Tonemapping::None` sulla camera.
- Nebbia di distanza rimossa (era troppo densa a 80 m sul terreno piccolo);
  riattivabile in futuro con slider.
- Luci: `GlobalAmbientLight` 80 cd/m², `DirectionalLight` 4000 lux con
  `shadow_depth_bias 0.05` / `shadow_normal_bias 3.0` per evitare shadow acne.
- Debug: F3 toggle wireframe collider Avian (disattivato di default).
- Diagnostica: log `terrain diagnostics` (image size, height range) a ogni rebuild.
- Test automatici: camminata 6 s su colline (player sempre a y≈0.85, mai nel
  vuoto), salto, menu Esc, 0 panic.

## v0.0.2.0 — Fase 2: Scena (terreno, cielo, nebbia, hot-reload) (2026-11-04)

- Terreno procedurale da heightmap PNG (`assets/textures/terrain_heightmap.png`,
  256×256 in scala di grigi): mesh generato a runtime con un piano suddiviso e
  vertici sollevati per pixel; grigio medio = altezza zero.
- Collider Avian `heightfield` generato dalla stessa heightmap: il player
  cammina realmente sulle colline.
- Hot-reload dell’asset: `bevy/file_watcher` + `AssetEvent<Image>::Modified`
  rigenerano mesh e collider senza riavviare il gioco.
- Cielo procedurale: cubemap generata a runtime (`sky.rs`) con gradiente
  orizzonte/zenit calcolato per pixel, senza asset esterni né HDRI da scaricare.
- Nebbia di distanza (`DistanceFog` + `FogFalloff::from_visibility`) e luce
  ambientale globale (`GlobalAmbientLight`) più calda e neutra.
- Rimossa la vecchia luce ambientale per-entità (spawn sbagliato) e il piano
  piatto segnaposto; il terreno ora è un’entità dedicata (`TerrainMesh`).
- Script `scripts/gen_heightmap.py` per rigenerare il terreno senza asset
  esterni (CC0, nessuna dipendenza aggiuntiva).
- Moduli nuovi: `terrain.rs` (heightmap → mesh/collider) e `sky.rs`
  (cubemap procedurale).
- Test: build pulita, runtime stabile, hot-reload verificato con
  heightmap modificata a runtime (mesh rigenerato senza panic).

## v0.0.1.4 — Fase 1.4: fix menu Esc e FPS overlay egui (2026-11-04)

- Fix regressione "menu Esc scomparso": il toggle era rimasto monodirezionale
  (solo chiusura) dopo la riscrittura della 1.3. Ripristinato il toggle
  bidirezionale `Playing ↔ Paused`.
- FPS overlay riscritto come Area egui in alto a destra (dentro
  `EguiPrimaryContextPass`): il vecchio `Text2d` era in coordinate world
  e risultava fuori schermo con la camera orbit.
- Rimosso il modulo `hud.rs` (Text2d + posizionamento viewport): il contatore
  ora vive in `menu.rs` insieme al resto della UI, con checkbox "Mostra FPS".
- `ShowFps` default `true`, persistente tra pause e reset.
- Test automatici: toggle Esc aperto/chiuso/riaperto, 2 reset consecutivi dal
  menu via click simulato — 0 panic, 0 ambiguità camera.

## v0.0.1.3 — Fase 1.3: fix reset e contatore FPS (2026-11-04)

- Fix del crash "Reset scena": introdotto lo stato transitorio `Resetting`;
  il reset avviene in `OnEnter(Resetting)` dove tutti i sistemi di gameplay
  (`run_if(in_state(Playing))`) sono spenti, eliminando la corsa critica con
  il ground-check che scriveva su entità despawnate.
- Fix della camera duplicata al reset: separato `spawn_camera` (solo startup)
  da `spawn_world` (startup + reset). La camera persiste e viene solo
  riposizionata; sparisce il warning "Camera order ambiguities".
- HUD FPS in alto a destra, attivabile/disattivabile dal menu Esc tramite
  checkbox "Mostra FPS". Usa `FrameTimeDiagnosticsPlugin` e `Text2d`.
- Rimossa risorsa `ResetSceneRequest` (ora obsoleta); codice documentato.

## v0.0.1.2 — Fase 1.2: Menu, salto e corsa (2026-11-04)

- Menu Esc (bevy_egui 0.43 + egui 0.36): lista tasti, slider sensibilità mouse,
  pulsanti "Riprendi" e "Reset scena" (solo da menu, per evitare pressioni accidentali).
- Stato applicativo `AppState { Playing, Paused }`: gameplay sospeso in pausa.
- Salto con Space: ground-check via raycast Avian (solo collider marcati `Ground`),
  impulso y=6.0 solo da terra, anti double-jump.
- Corsa con Shift: moltiplicatore 1.7× su WASD.
- Sensibilità mouse default 0.003 (da 0.005), regolabile a runtime dal menu.
- Reset scena da zero: despawn ricorsivo delle entità `SceneNode` (terreno, luci,
  player) e ricostruzione. La camera persiste (il contesto egui è agganciato ad essa
  e bevy_egui non lo ricrea) e viene riposizionata.
- Architettura: nuovi moduli `scene.rs` (spawn/reset) e `menu.rs` (UI).

## v0.0.1.0 — Fase 1: Movimento (2026-11-04)

- Player: capsule dinamica (Avian) con rotazione bloccata, gravità e collisione col terreno.
- Movimento WASD/frecce relativo all'orientamento della camera (piano XZ), 6 m/s.
- Mesh visivo separato (corpo + "naso") con rotazione smooth verso la direzione di movimento.
- Camera terza persona: orbita col mouse (yaw/pitch clampato) + follow con smoothing.
- Codice organizzato in moduli: `main.rs`, `player.rs`, `camera.rs`.
- Verificato: il player si assesta a y=0.850 (capsula r=0.35, semi-lunghezza 0.5).

## v0.0.0.2 — Fase 0 (2026-11-04)

- fix: inserito `CameraRenderGraph::new(Core3d)` esplicitamente prima di `Camera3d`
  per evitare il warning spurio dell'hook di spawn in Bevy 0.20.
- Nota successiva: il warning "render graph not configured" all'avvio è in realtà
  rumore upstream di Bevy 0.20 (si riferisce a entità Observer interne, non alla nostra
  camera). Il fix sopra è superfluo ma innocuo; la fonte è documentata nel codice.

## v0.0.0.1 — Fase 0: Bootstrap (2026-11-04)

- Progetto scaffoldato: Rust + Bevy 0.20.1.
- Finestra 1280×720 con titolo "Planet One".
- Scena base: cubo placeholder (personaggio), piano terra, luce ambientale + direzionale con ombre.
- Camera terza persona statica (il follow arriva in Fase 1).
- Licenza doppia MIT OR Apache-2.0, README, template issue/PR, convenzioni versioning.
