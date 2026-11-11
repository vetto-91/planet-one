# Piano di beta test, debug e correzioni — Planet One v0.0.55.0

Obiettivo: arrivare a una versione **stabile** prima di riprendere lo sviluppo. Il lavoro va in quattro fasi:
prima gli strumenti per giocare e controllare il gioco in automatico, poi i bug bloccanti, poi un giro
sistematico di ogni sistema, infine documenti e pulizia.

Questo piano nasce da una prima ricognizione fatta il 10/11/2026 su `main` = `v0.0.55.0` (commit `e5c765c`).
- **Analisi statica**: clippy esteso (`pedantic`, `unwrap_used`, `expect_used`, `float_cmp`, `match_same_arms`).
- **Documenti**: confronto fra documenti e codice.
- **Asset**: elenco degli asset mancanti e non usati.
- **Partite di prova**:
  - 4 avvii con meteo e ore diverse, con lettura dei log;
  - un giro dell'isola pilotato con la shell del gioco (10 luoghi, screenshot);
  - una prova di dialogo;
  - un ciclo salva (F5) / carica (F9).

Il metodo di guida usato (xdotool che digita nella shell `\`, più screenshot della finestra) funziona,
ma è lento e cieco: non sa dove sono gli NPC né cosa succede. Da qui la Fase 0.

---

## Riepilogo di quanto trovato

### Bug di gioco (con prova)

| # | Gravità | Problema | Prova |
|---|---|---|---|
| B1 | **Alta** | **Morte non gestita per le frecce dei banditi.** La vita scende sotto zero (−5) senza morte né respawn. Da lì il giocatore è di fatto immortale, perché il danno da contatto si ferma se `hp <= 0`. Solo il contatto (`combat.rs:292`) e la caduta (`player.rs:456`) gestiscono la morte, e in due modi diversi: la caduta non resetta la camera. | Log `[NEMICI] freccia: vita -5`, nessun respawn (`combat.rs:559-567`) |
| B2 | **Alta** | **Caduta sotto il mondo.** Dopo `tp 58 12 -58` (torre in rovina) il giocatore era a Y −40, sotto il terreno, in caduta senza fine. Manca una rete di sicurezza. | Screenshot del giro, HUD «Y −40.0» |
| B3 | Media | **Prestazioni**: 23–55 FPS in `--release` su RTX 3080 Ti a finestra. | HUD FPS in tutti gli screenshot |
| B4 | Media | **Avviso Bevy B0004 a ogni avvio** (e di nuovo dopo F9): un'entità con `InheritedVisibility` ha un genitore senza. Possibili difetti di visibilità di qualche oggetto. | Log `warning[B0004] Entity ~2324` |
| B5 | Media | **Le frecce ignorano `invisibile`** (il contatto lo rispetta): incoerenza dei trucchi. | `combat.rs:559` controlla solo `god` |
| B6 | Media | **Gli scuri della casetta del Viandante sono chiusi a mezzogiorno** (prova del 10 mag, 11:45–12:22). Da verificare contro la routine del negoziante (`Keeper`). | Screenshot prova di dialogo |
| B7 | Bassa | `PLANET_ONE_DATE` con formato sbagliato (es. `2026-05-10`) viene **ignorata in silenzio**: ho perso così le prove d'inverno e di neve. Serve un avviso. | `daynight.rs:106` |
| B8 | Bassa | Le partite di prova (`PLANET_ONE_SESSION`) scrivono nei **salvataggi veri** dell'utente (`~/.local/share/planet-one/saves/Beta`). | Log `[SAVE] salvato …/Beta/rapido.txt` |
| B9 | Da verificare | `tp porta nord` porta a (0,1; −2,3), 5 m dentro le mura invece che sul cancello (z −7,6). | Log shell |

### Problemi noti lasciati aperti nei rilasci precedenti

- Il gesto `Interact` degli abitanti alla finestra **non è mai stato visto a schermo** (avviene dentro casa).
- Le gallerie della grotta: le pareti che sbucano dalla collina dove il terreno è basso e il laghetto non sono controllati a schermo.
- La grotta non è davvero sottoterra: la heightmap non ha cavità.
- Le scale delle mura sono corrette solo spostandone l'inizio. Non sono state viste a schermo né provate con la fisica vera.
- Il difetto di Bevy 0.20.1 in `extract_lights` è **aggirato**, non risolto: `graphics::keep_shadow_lights_extracted` segna come cambiate ogni fotogramma tutte le luci con ombre.
- Le luci interne si accendono solo dentro l'impronta dell'edificio (`BuildingFootprint`): nelle gallerie della grotta valgono solo i falò.

### Codice

- Clippy normale pulito. Il clippy esteso dà circa 1.600 avvisi, quasi tutti rumore (cast `usize→f32`, indicizzazione). Quelli che contano:
  - 20 `unwrap()` / 9 `expect()` fuori dai test: generazione mesh in `nature.rs`, `buildings.rs:841`, `menu.rs:484`. Sicuri oggi, ma vanno resi espliciti.
  - 7 confronti stretti fra `f32` (`animation.rs:236/250`, `world_props.rs:472/588`, `village_life.rs:354`).
- **Logica duplicata**: morte e respawn in due punti (B1); `hearth_schedule` e `shutter_schedule` quasi identici.
- **Sistemi al limite dei 16 parametri di Bevy** (`villager_routine`): ogni modifica obbliga a spezzarli. Va rifattorizzato con `SystemParam` raggruppati.
- **File molto lunghi**: `hut.rs` 2.410 righe, `buildings.rs` 2.317, `debug_place.rs` 1.854, `save.rs` e `village.rs` 1.587. Da dividere quando li si tocca.

### Documenti

| # | Dove | Problema |
|---|---|---|
| D1 | `CLAUDE.md` § Verifica | Dice «Il gioco non ha ancora test automatici»: ce ne sono 155, compresa la prova di camminata con fisica vera. |
| D2 | `README.md` § Controlli | «Shift: Corsa (1.7×)», ma nel codice è 1,55 (`SPRINT_FACTOR`). |
| D3 | `README.md` § Quest | «ladro (cubo rosso…)»: ora è un personaggio glTF (`figure::spawn_character(…, "ladro")`). |
| D4 | `README.md` § Debug | Elenca 4 variabili su 17. Mancano `DATE`, `WEATHER`, `GIVE`, `SHELL`, `SESSION`, `MENU`, `WINDOWED`, `ANIM`, `SCAN_WALLS`, `AI_MOCK`, `TTS_MOCK`, `PIPER`, `STT_MIC`. |
| D5 | `README.md` § Requisiti | «Rust ≥ 1.97», ma `Cargo.toml` non ha `rust-version`. |
| D6 | `docs/decisions/README.md` | Registra ADR 0001 e 0002, ma i file non esistono. Mancano ADR per mura e torri (v0.0.50), scuri e focolari, gallerie della grotta, gesti degli abitanti e il difetto di Bevy aggirato. |
| D7 | `src/devtools.rs` | `PLANET_ONE_DATE=anno,mese,giorno` va bene nel commento, ma l'errore non si vede (B7). |

### Asset

- Riferimenti nel codice: nessun file mancante. I nomi «mancanti» trovati dalla ricerca sono percorsi relativi o generati, tutti presenti.
- Probabilmente non usati:
  - `assets/textures/abitante-*.png` (7 file, circa 2,5 MB: vecchie texture degli abitanti a blocchi);
  - la cartella vuota `assets/scenes/`.
- Da controllare a vista: tutte le texture PBR caricate con nome composto (`pbr::load`), le voci (`assets/audio/voci/`, 102 file) rispetto alle battute in `dialoghi.txt` (`scripts/export_voci.py`), le licenze in `assets/models/LICENSES.md` rispetto ai modelli presenti.

---

## Fase 0 — Strumenti per giocare e controllare il gioco senza mani (prima di tutto)

**Stato al 10/11/2026: fatta** (`src/autopilot.rs`, `src/bugreport.rs`, `scripts/beta_check.sh`, `prove/*.txt`, `prove/riferimento/`, ADR 0031).
- Copione: `vai`, `corri`, `guarda`, `premi`, `avanza`, `scegli`, `foto`, `dump`, `verifica`, `attendi`, `mai`, `esci`.
- Variabili: `PLANET_ONE_SPEED`, `PLANET_ONE_DATA_DIR`, `PLANET_ONE_SIZE`, `PLANET_ONE_OUT`.
- Shell: `vai`, `corri`, `dove`, `guarda`, `foto`, `dump`, `copione`, `quest <fase>`, `uccidi`, `vicino`, `fps`, `entita`.
- F12 «Segnala bug».
- `beta_check.sh`: rapporto completo e confronto delle foto con i riferimenti. Fra due corse uguali la differenza misurata è 0,005–0,08 (il mulino, per le pale che girano); la soglia è 0,12.
- Copioni:
  - passano: `quest-avvio`, `quest-completa` (dal ladro alla consegna), `giro`, `f12`;
  - **falliscono di proposito, riproducendo i bug: `b1-frecce` e `b2-torre`**.

Scoperte fatte costruendo il pilota:
- `tp <abitante>` e `dove` usavano la posizione **locale** degli NPC figli di un altro oggetto: `tp viandante` portava in un punto sbagliato. Corretto (posizione nel mondo).
- **Viandante spesso occupato**: alle 11 va e viene dalla stufa; mentre cammina (`Busy`) non si può parlare con lui e non compare nessun messaggio. Il giocatore preme E e non succede nulla. Da decidere: un fumetto «Un attimo!» o la sosta al banco quando arriva qualcuno.
- L'insegna della casetta è in inglese («TASTE OF THE FOREST (GUSTO DEL BOSCO)») e altrove la casetta è chiamata «Ristoro del bosco»: nome da uniformare.
- B8 risolto da `PLANET_ONE_DATA_DIR` (usata da `beta_check.sh`).

Oggi un'IA (o tu) gioca solo con tastiera, mouse e screenshot. Propongo un **pilota automatico** dentro il gioco.
Così ogni bug di gameplay o di grafica si riproduce con un file di testo, ogni volta uguale.

### 0.1 Copione di prova: `PLANET_ONE_SCRIPT=prove/nome.txt`
Un file di testo con un comando per riga, eseguito in ordine con i tempi di gioco. Comandi proposti:
- tutti i comandi della shell (`tp`, `ora`, `meteo`, `dai`, `god`…);
- `aspetta <s>`: attesa in tempo di gioco;
- `vai <x> <z>` / `vai <nome>`: cammina davvero (fisica vera) verso un punto, un abitante o un luogo. Si ferma se è bloccato e lo scrive (trova muri, scale rotte, porte bloccate);
- `guarda <yaw> <pitch>` / `guarda <nome>`: orienta la camera;
- `premi <tasto> [per <s>]`: input simulato (corsa, salto, attacco), senza xdotool;
- `usa`: interagisce con l'oggetto o l'NPC al mirino; `scegli <n>`: risposta nel dialogo;
- `foto <nome>`: screenshot fatto da Bevy (`Screenshot::primary_window()`) in `prove/out/`, senza finestra attiva né xdotool;
- `stato`: fotografia in **JSON** di tutto lo stato utile. Contiene posizione, vita, inventario, quest, ora e meteo, animazione corrente del giocatore, NPC vicini con fase, animazione e percorso, nemici con vita, porte/scuri/focolari vicini, FPS;
- `verifica <condizione>`: ad esempio `verifica vita > 0`, `verifica quest = Accepted`, `verifica scuri casa-media 1 = aperti`. Un fallimento finisce nel log con `[VERIFICA] FALLITA` ed esce con codice d'errore;
- `esci`.

### 0.2 Corsa veloce e isolamento
- `PLANET_ONE_SPEED=4`: tempo di gioco accelerato per le routine (giornata intera in pochi minuti).
- `PLANET_ONE_DATA_DIR=<cartella>`: salvataggi e impostazioni in una cartella di prova (risolve B8).
- `PLANET_ONE_SIZE=1280x720`: finestra fissa e ripetibile per gli screenshot.

### 0.3 Nuovi comandi della shell (utili anche a te giocando)
`stato` in JSON, `vicino [raggio]` (cosa c'è attorno), `vai`, `usa`, `scegli`, `quest <fase>` (salta a una fase), `uccidi` (nemico al mirino), `fps`, `entita` (conteggi per tipo, per trovare perdite), `foto`.

### 0.4 Tasto «Segnala bug» (per te): F12
Salva in `~/.local/share/planet-one/bug/<data-ora>/` lo screenshot, lo `stato` JSON, le ultime 300 righe di log e una nota scritta in un campo egui.
Mi mandi la cartella e riproduco il bug esattamente. Da usare insieme alla **scheda di segnalazione** in fondo.

### 0.5 Copioni e controllo automatico
- `prove/*.txt`: un copione per sistema (vedi Fase 2).
- `scripts/beta_check.sh`: esegue tutti i copioni e raccoglie `WARN`/`ERROR`/`panic` e le verifiche fallite. Confronta gli screenshot con quelli di riferimento (`compare -metric AE` di ImageMagick, già installato) e produce `prove/out/RAPPORTO.md`.
- Test logici senza grafica (`cargo test`, `MinimalPlugins` come `walk_test`): quest completa, morte e respawn da ogni fonte, salva/carica di ogni stato, routine di un abitante per 24 ore.

**Verifica della Fase 0**: un copione «giro dell'isola» e uno «quest Denaro Rubato» girano da soli, danno uno stato JSON coerente e gli screenshot, e falliscono di proposito se si rompe una condizione.

---

## Fase 1 — Bug bloccanti

**Stato all'11/11/2026: fatta**, tutti i copioni verdi.
- B1 morte unica (`combat::player_death`), B5 frecce e `invisibile`, B4 avviso B0004 (era la stufa della casetta), B7 avviso sulla data: corretti.
- B2: la causa era un `tp` sotto la superficie del terreno, non un buco; aggiunta la rete di sicurezza (`player::needs_rescue`).
- B6: il Viandante ora apre gli scuri al mattino e torna al banco se ti avvicini.
- B9: non è un bug, `tp porta` porta di proposito 6 m dentro il cancello.
- **Trovato in più**: l'imbocco delle gallerie della grotta (v0.0.55) era impraticabile perché il terreno attraversava la discesa. Corretto con `terrain::TerrainHole`.

B3, misura (FPS a finestra 1280×720, RTX 3080 Ti, entità ≈ 19.900 ovunque):

| Luogo | Giorno | Notte |
|---|---|---|
| Spawn | 35 | 37 |
| Piazza | 42 | 31 |
| Faro | 38 | 49 |
| Mulino | 30 | 35 |
| Rovine | 45 | 50 |
| Grotta | 40 | 32 |

Gli FPS non dipendono molto dal luogo e le entità sono sempre le stesse: il costo sembra globale, non locale. Le correzioni si decidono insieme.


1. **B1 Morte unica**: un solo sistema `player_death` che scatta su `hp <= 0` qualunque sia la causa (contatto, frecce, caduta, futuro annegamento). Fa vita piena, posizione di rinascita, reset della camera, messaggio a schermo «Sei svenuto…» e un piccolo tempo di invulnerabilità; facoltativo perdere qualche moneta. Test: ogni fonte di danno porta a respawn; la vita non è mai negativa.
2. **B2 Rete di sicurezza**: se il giocatore (o un NPC, un animale, un oggetto) scende 3 m sotto il terreno nel punto in cui si trova, va riportato sopra, con log `[RETE]` della posizione. Poi va trovata la causa alla torre in rovina: teletrasporto dentro un collider, oppure un buco o uno spiazzo mal raccordato nel terreno. Test: `tp` in 50 punti a caso con `verifica y > terreno - 0,5`.
3. **B5**: le frecce rispettano `invisibile`; i banditi non vedono chi è invisibile.
4. **B4**: trovare l'entità del B0004 con un sistema di diagnosi che, all'avviso, stampa nome e componenti del figlio e del genitore. Poi dare `Visibility` al genitore o spostare il figlio.
5. **B3 Prestazioni**:
   - misurare con i contatori diagnostici di Bevy (`FrameTimeDiagnosticsPlugin`, conteggio di entità, mesh, luci);
   - provare i preset grafici, spegnendo una cosa alla volta (ombre dei lampioni, SSAO, luci interne, vegetazione);
   - dare a `stato` gli FPS medi di ogni luogo, così da avere una mappa delle zone pesanti;
   - correggere le zone più pesanti.

   Non è ottimizzazione prematura: sotto 30 FPS si gioca male.

---

## Fase 2 — Giro sistematico di ogni sistema (beta test)

**Stato all'11/11/2026: primo giro** con i copioni `movimento`, `combattimento`, `salvataggi`, `meteo` e `vita-paese` (24 ore a velocità 6, un `dump` di tutti gli abitanti ogni mezz'ora).

| # | Sistema | Esito | Stato |
|---|---|---|---|
| F2-1 | Combattimento | **Bug**: i banditi uccisi nei primi 40 s di gioco ricomparivano subito, mentre il log diceva «tornerà una notte a caso» | corretto (popolamento iniziale una volta per scena, `BanditsSeeded`) |
| F2-2 | Shell | `dai freccia` non funzionava: il nome è «Frecce» | corretto (singolare e plurale) |
| F2-3 | Movimento | Nel mare il personaggio sta dritto in posa di riposo, con i piedi sopra l'acqua: sembra camminare sull'acqua. Manca una posa o un'animazione di nuoto | corretto: fermi si sta a galla dritti, in movimento il corpo si distende sull'acqua (`player::facing`, copione `nuoto`) |
| F2-4 | Abitanti | Le guardie delle mura (Gino, Ivo, Livio, Mario) e il guardiano del faro (Ettore) lavorano 24 ore su 24 senza mai riposare; le guardie dei cancelli hanno invece tre turni. Incoerenza di regole | corretto: le ronde hanno i turni (Gino 1, Ivo 2, Livio e Mario 3), a fine turno entrano in una torre e riposano (Gino e Ivo a letto alla caserma); ce n'è sempre almeno una sulle mura (test `the_walls_are_always_patrolled`, 24 ore verificate col pilota). Ettore lavora di notte (17→7) e dorme di giorno al faro |
| F2-5 | Abitanti | Nessuno resta bloccato nelle 24 ore. Il campionamento ogni mezz'ora è grosso: un blocco breve può sfuggire | ok |
| F2-6 | Salvataggi | F5/F9 riportano posizione, ora, inventario e quest | ok |
| F2-7 | Caduta e morte | 22,7 m: danno 173, morte, rinascita allo spawn con la vita piena | ok |
| F2-8 | Meteo | Le foto non sono conclusive: la camera era sotto il tetto del pozzo. Nella neve, a terra non c'è neve dopo 6 s: è giusto, si accumula col tempo | rifatto: nebbia, nuvole, pioggia (gocce visibili), temporale e neve si vedono; la neve copre il terreno dopo 90 s. L'inquadratura resta stretta accanto a una casa |
| F2-9 | Abitanti | Livio e Mario senza letto | corretto: due brande nel sottotetto della caserma |
| F2-10 | Interni | **Bug**: nei sottotetti la rampa finiva contro il muretto, quindi in soffitta non si saliva (né il giocatore né un abitante). Trovato dalla prova di camminata con le nuove brande | corretto: la rampa finisce 0,6 m prima (dove c'è spazio) e c'è il percorso fino al sottotetto |
| F2-11 | Negozio e mobili | Comprare, monete insufficienti (pulsante spento, messaggio), forziere della grotta (54 monete e 2 pozioni), «prendi tutto» | ok (copione `negozi-mobili`) |
| F2-12 | Interfaccia | Menu Esc, diario, mappa, inventario, personaggio, «Aspetta», obiettivi (TAB) si aprono e chiudono | ok (copione `interfaccia`) |
| F2-13 | Interfaccia | L'inventario dice «Oggetti: 34» contando anche le 30 monete; le targhette dei nomi degli abitanti si sovrappongono quando sono vicini | corretto nel codice: monete contate a parte («Oggetti: 4   Monete: 30»), targhette sovrapposte spostate in alto (test `overlapping_plates_are_stacked`); **non ancora visto a schermo** |
| F2-14 | IA locale | **Bug**: la prima risposta, col gioco che occupa la scheda video, superava i 40 s e la chat diceva «la IA non risponde»; il messaggio d'errore nominava sempre il Viandante, con chiunque si parlasse | corretto: attesa fino a 150 s, nome giusto. Con Marta e `gemma4:e4b` la chat risponde (copione `ia-locale`) |
| F2-15 | Grafica | Le foto degli edifici del copione `edifici` vengono dall'alto: `guarda <edificio>` mira alla porta, che è sotto i piedi. Va aggiunta la distanza della camera | `guarda <edificio> <distanza>` mira a metà facciata e allontana la camera; **non ancora visto a schermo** |
| F2-16 | Suoni | Non si possono controllare dai log né dalle foto | strumenti pronti: `registra`/`ferma` nel pilota (WAV da `parec`) e `scripts/ascolta.py` (volume, picchi, saturazione, silenzi, eventi, regolarità dei passi, bande, spettrogramma, parlato con whisper.cpp locale). Provato su una voce del Viandante; l'ascolto del gioco è da fare |



Per ogni voce: un copione in `prove/`, cosa ci si aspetta, come si verifica (stato JSON, screenshot o test).

| Sistema | Cosa provare | Atteso |
|---|---|---|
| **Movimento** | Camminata, corsa, salto, pendii, acqua (nuoto), scale delle case, delle mura e delle torri, rampe della grotta | Nessun blocco, niente compenetrazioni, nuoto solo in acqua vera, danno da caduta coerente |
| **Camera** | Terza e prima persona, collisione coi muri, dentro le case e le gallerie, guardando in basso | Mai dentro un muro; in prima persona si vede il corpo e non la testa |
| **Combattimento** | Spada, ascia, arco, scudo contro ladro e banditi; respawn dei banditi; frecce dei nemici | Danni giusti, scudo che riduce, morte unica (B1), animazioni d'attacco |
| **Quest «Denaro Rubato»** | Tutti i rami: accetta, rimanda, uccidi prima, raccogli 12 monete, consegna; salva/carica a ogni fase | Fasi e diario coerenti; ricompensa +5 |
| **Dialoghi e IA** | Viandante, abitanti, guardie; IA spenta e accesa (Ollama locale), `AI_MOCK`/`TTS_MOCK`; allontanarsi durante il dialogo | Chiusura oltre 8 m, voci giuste, mai rete esterna |
| **Inventario, negozi, contenitori** | Comprare e vendere, peso e zaini, trascinamento, doppio clic, forzieri della grotta, case | Monete e peso coerenti, contenuti che restano dopo salva/carica |
| **Salvataggi** | Automatico, F5/F9, menu, carica da elenco, dopo un cambio di giorno | Nessuna perdita: posizione, ora, quest, porte, scuri, focolari, contenitori, abitanti a metà percorso |
| **Tempo, meteo, stagioni** | `aspetta` 24 h in 4 stagioni (`PLANET_ONE_DATE` corretto) e 6 meteo | Alba e tramonto giusti, neve a terra e sui tetti, pioggia sotto i tetti, suoni |
| **Vita del paese** | 24 h accelerate: abitanti (lavoro, sera, notte), guardie (turni, ronda sulle mura), porte, scuri col gesto, focolari e fumo, torce, lampioni, campane | Nessuno bloccato, nessuno che attraversa i muri, scuri e porte coerenti con chi li usa |
| **Luoghi e grotta** | Ogni luogo a piedi dal paese; dentro la grotta fino a ogni galleria; forzieri | Nessuna caduta, nessuna parete che sbuca dal terreno, luce sufficiente |
| **Mura e torri** | Ronda completa sul camminamento, ogni scala, ogni torre | Nessun cuneo o buco, passaggi liberi (`PLANET_ONE_SCAN_WALLS` pulito) |
| **Grafica** | Ogni tipo di edificio da vicino, alberi, animali, personaggi (LOD e animazioni lontane), notte con lampioni | Niente z-fighting, texture stirate, mesh rovesciate o ombre sbagliate |
| **Animazioni** | Ogni clip su ogni personaggio, arma riposta in corsa, gesto alla finestra, chi dorme, chi parla | Nessuna posa rotta né arma nella testa |
| **Suoni** | Passi su ogni superficie, ambiente per zona, musica giorno e notte, campane, meteo | Volume giusto, nessun suono mancante o ripetuto in modo innaturale |
| **Interfaccia** | Menu, impostazioni (salvate e ricaricate), mappa, diario, bussola, HUD, shell, F4 | Testi giusti, nessun pannello sovrapposto, tasti come nel README |
| **Piattaforme** | Build Windows e macOS dalla CI | Si avviano e trovano gli asset |

---

**Limite scoperto l'11/11**: il pilota ha bisogno che la sessione di `dev` sia quella attiva. Se è attiva l'altra sessione del PC (un altro utente su un altro terminale virtuale), il gioco va in panic alla creazione della finestra (Vulkan «Invalid surface»). Con lo schermo solo bloccato, o il monitor spento, funziona. Il rendering software (Xvfb + llvmpipe) è troppo lento: in 5 minuti non finisce l'avvio. Rimedio possibile: una modalità senza finestra, con la camera che disegna in un'immagine sulla GPU.

## Fase 3 — Documenti e pulizia

- Correggere D1–D7. Aggiungere in `README` le nuove variabili, i comandi della shell e il copione di prova.
- Scrivere gli ADR mancanti (0001, 0002 e le decisioni recenti). Un ADR per il pilota automatico.
- Togliere gli asset non usati (`abitante-*.png`, `assets/scenes/`), dopo una ricerca finale.
- Rifattorizzare: morte unica (fatta in Fase 1), orari di scuri e focolari in una funzione comune, `SystemParam` raggruppati per `villager_routine`, divisione dei file sopra 2.000 righe quando li si tocca.

## Fase 4 — Rilascio di stabilità

Si chiude quando:
- tutti i copioni di `prove/` passano;
- `cargo test` è verde;
- `beta_check.sh` non ha `ERROR` né `panic`;
- gli FPS sono sopra la soglia concordata;
- ci sono 2 ore di gioco tue senza bug bloccanti.

Poi si fa la versione «beta stabile» (proposta `v0.1.0.0`, inizio di una nuova fase), con la solita procedura di rilascio.

---

## Come puoi aiutarmi

Quello che non posso provare da solo:
- **Sensazione dei controlli**: mouse vero, sensibilità, schermo intero, cursore catturato, F11.
- **Audio**: qualità e volume dei suoni sintetizzati e delle voci; io vedo solo i log.
- **Partite lunghe** giocate «da giocatore», senza trucchi: lì escono i bug che i copioni non pensano.
- **Windows e macOS**: build della CI.
- **IA locale con Ollama, Piper e whisper.cpp** sul tuo hardware, con microfono vero.

**Scheda di segnalazione** (oppure il tasto F12 della Fase 0.4):
- **Cosa**: una frase.
- **Dove**: coordinate F3 o `coordinate on`, più uno screenshot.
- **Quando**: ora di gioco, meteo, cosa stavi facendo.
- **Atteso / successo**.
- **Si ripete?** Sì / no / a volte.

## Ordine proposto

Fase 0 (0.1, 0.2, 0.4 prima di tutto) → Fase 1 (B1, B2, B5, B4, B3) → Fase 2, un sistema alla volta, con un rilascio piccolo per gruppo → Fase 3 → Fase 4.
