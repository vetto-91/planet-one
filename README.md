# Planet One

RPG 3D open source, in terza persona, con grafica realistica in corso (personaggi veri da `v0.0.25.0`, mondo e materiali PBR da `v0.0.26.0`, animazioni dei personaggi da `v0.0.28.0`). Scritto in **Rust** con il motore **[Bevy](https://bevy.org)**. Obiettivo iniziale: una **vertical slice** giocabile (1 quest, 1 NPC, 1 nemico, inventario minimale).

> Stato: **Fase 5 — Vertical slice completata, con rifiniture** (`v0.0.34.0`).
> Quest "Denaro Rubato", bussola, obiettivi (TAB), combattimento e inventario.
> Roadmap e fasi: [docs/ROADMAP.md](docs/ROADMAP.md).

## Requisiti

- Rust stable (>= 1.97) — [`rustup`](https://rustup.rs)
- Linux: `libudev`, `alsa`, `vulkan` driver (di solito già presenti su desktop)
- Windows 10/11: driver video con Vulkan o DirectX 12 (nessun'altra dipendenza)
- macOS 11+: Metal (Intel o Apple Silicon, nessun'altra dipendenza)

## Installazione dai pacchetti di rilascio

Dalla pagina [Releases](https://github.com/vetto-91/planet-one/releases) scarica l'archivio della tua piattaforma,
estrailo (l'eseguibile e la cartella `assets/` devono restare insieme) e avvia `planet-one`.

| Piattaforma | Archivio | Primo avvio |
|---|---|---|
| Linux x86_64 | `planet-one-vX-linux-x86_64.tar.gz` | `./planet-one` |
| Windows x86_64 | `planet-one-vX-windows-x86_64.zip` | doppio clic su `planet-one.exe`; se SmartScreen avvisa: "Ulteriori informazioni" → "Esegui comunque" (eseguibile non firmato) |
| macOS (Intel + Apple Silicon) | `planet-one-vX-macos-universal.tar.gz` | eseguibile non firmato: tasto destro su `planet-one` → Apri, oppure `xattr -dr com.apple.quarantine <cartella>` e poi `./planet-one` |

Note: su Mac i tasti F3/F4/F11 richiedono il tasto Fn (o l'impostazione "Usa F1, F2 ecc. come tasti funzione standard").
Le impostazioni (volume) sono salvate in `%APPDATA%\planet-one\` (Windows), `~/Library/Application Support/planet-one/` (macOS),
`~/.config/planet-one/` (Linux). Windows e macOS sono costruiti da GitHub Actions (`.github/workflows/release.yml`).

## Clonare e avviare

```bash
git clone git@github.com:<utente>/planet-one.git
cd planet-one
cargo run --release
```

La prima compilazione scarica e compila Bevy: servono alcuni minuti.

## Compilazione ed esecuzione

```bash
cargo run            # esegue in dev
cargo run --release  # build ottimizzata
cargo fmt            # formatta
cargo clippy         # linter
```

## Shell dei trucchi (tasto `\`)

Giocando normalmente non ci sono trucchi. Con il tasto `\` si apre una shell (Esc o `\` la chiudono), e a ogni avvio si riparte con tutto spento. `aiuto` mostra i comandi:

| Comando | Effetto |
|---|---|
| `god`, `noclip`, `invisibile` | Nessun danno; volo senza collisioni (Spazio sale, Ctrl scende); invisibili ai nemici |
| `tp x y z`, `tp chiesa`, `tp porta nord`, `tp marta`, `tp casa-media 1` | Teletrasporto |
| `passepartout [off]`, `chiavi`, `dai mela 3` | Chiave che apre tutto, chi ha quali chiavi, oggetti |
| `f3`, `f4`, `debug on`, `coordinate` | Collider, piazzamenti, tasti F3 e F4, coordinate nell'HUD |
| `ora 21.5`, `meteo neve`, `meteo auto` | Ora e meteo |
| `ia [on\|off]`, `ia modelli`, `ia modello <nome>` | Dialoghi IA, sempre e solo locali |
| `tts [on\|off]`, `tts voci`, `tts voce <nome\|auto>` | Voce IA |
| `stt [on\|off]`, `stt modelli`, `stt modello <nome>` | Microfono nella chat, solo locale |

## Debug piazzamenti (F4)

Per indicare dove mettere alberi o qualunque oggetto: F4 apre il pannello. In 3D il punto è quello del **mirino** (clic sinistro conferma, trascina = direzione, **R** = ruota 15°);
**Ctrl tenuto** libera il mouse per usare il pannello. Nella mappa (M) si sceglie col clic sinistro; **X** elimina al mirino.

- **Piazza**: *singolo* (allineamento libero, a **griglia** o a **linea** con distanza uguale), *gruppo*,
  *rettangolo* (4 angoli qualsiasi), *cerchio* (raggio in anteprima dal centro) e *poligono* (N punti, poi **OK** o
  Invio). Per le aree si può scegliere la distribuzione *regolare* (passo uguale) o casuale; l'anteprima mostra i punti.
- **Gomma**: clic su un oggetto per eliminarlo; dall'elenco "Elimina" toglie un intero piazzamento.
- **Sposta**: clic su un oggetto e poi sul nuovo punto; resta parte del suo piazzamento.
- **Ruota**: clic e trascina da un oggetto; la freccia rossa indica il suo davanti. Gli oggetti non si compenetrano; "Ridistribuisci" rigenera un'area.

Ogni modifica riscrive `debug/placements.txt` (blocchi `[P-nnnn]`; il nord è −Z), riletto all'avvio. Alberi, fiori,
panchine e lampioni diventano **oggetti veri** (anche senza F4; gli alberi evitano spawn, casetta, pozione e ladro).
Nella mappa: frecce/WASD o tasto centrale = sposta la vista, C = su di te.

## Debug

Variabili d'ambiente per provare il gioco senza giocarlo: `PLANET_ONE_HOUR=21.5` (ora iniziale),
`PLANET_ONE_POS=x,y,z` (teletrasporto del player), `PLANET_ONE_DELAY_SCALE=0.1` (ritardi del Viandante), `PLANET_ONE_LOOK=yaw,pitch[,distanza[,1]]` (camera; 1 = prima persona).

## Debug input

Tutti gli eventi di tastiera/mouse vengono loggati con prefisso `[INPUT]`/`[EVENT]`:

```bash
~/.cargo/bin/cargo run 2>&1 | tee run.log
grep "\[INPUT\]" run.log    # solo input
grep "\[EVENT\]" run.log    # eventi finestra/cursore
```

## Controlli

| Tasto | Azione |
|---|---|
| W/A/S/D o frecce | Movimento |
| Shift | Corsa (1.7×) |
| Space | Salto |
| Mouse | Orbita camera (sensibilità dal menu) |
| Esc | Menu pausa (tasti, slider sensibilità, checkbox FPS, reset scena) |
| \\ | Shell dei trucchi (vedi sotto) |
| F3 | Wireframe dei collider: attivo solo dopo `debug on` nella shell |
| F11 | Schermo intero on/off (in fullscreen il cursore è bloccato e nascosto) |
| M | Mappa dall'alto (vista drone, nord in alto, bussola in alto a sinistra); rotella o +/- = zoom; frecce/WASD o tasto centrale = sposta, C = su di te; M/Esc = chiudi |
| F4 | Modalità debug piazzamenti, attiva solo dopo `debug on` nella shell (vedi sotto): strumenti Piazza / Gomma / Sposta, clic sinistro (mappa e 3D) |
| I | Inventario: categorie, ricerca, filtri, peso, assegnazione ai tasti 1-9 e 0 |
| T | "Aspetta": fa avanzare l'ora di gioco di N ore (T o Esc per chiudere) |
| J | Diario delle quest (tab Da fare / Completate, quest espandibili); J o Esc per chiudere |
| TAB (tenuto) | Obiettivi della quest (solo a quest attiva) |
| E | Parla con l'NPC vicino (entro 3 m); nel dialogo continua / conferma; **guardando un oggetto col mirino** (entro 3 m) lo usa: porta, finestre, stufa, interruttore della luce, serranda del negozio |
| Invio | Nel dialogo: continua / conferma (alternativa a E) |
| Su/Giù o W/S | Nel dialogo: cambia la scelta evidenziata |
| 1–9, 0 | Usa l'oggetto nello slot corrispondente della barra in basso (pozione: +25 HP) |
| Click sinistro | Attacco melee |

## Quest: "Denaro Rubato"

Parla con il **Viandante**, il negoziante nel chiosco della casetta vicino allo spawn: alla fine del dialogo scegli
"Sì, ti aiuto" per far partire la quest ("Ora non ho tempo" la rimanda).
Trova il **ladro** (cubo rosso, lontano, oltre le colline; il segnalino rosso sulla bussola
in alto indica la sua direzione), uccidilo, raccogli le **12 monete d'oro** che lascia,
torna dal Viandante → completata, con ricompensa di 5 monete.
Tieni **TAB** per gli obiettivi: spunta verde = completato, sfera azzurra = in corso.

Se uccidi il ladro prima di parlare col Viandante, la quest si attiva da sola con gli
obiettivi già completati fino a quel punto.
Se ti allontani dal Viandante durante il dialogo (oltre 8 m) il dialogo si chiude e lui saluta.
Il ladro ti apostrofa quando ti vede da vicino (~12 m).

### La casetta del Viandante

Il Viandante vende dal chiosco della casetta (3D, con fondamenta e scalini di pietra). Porta, finestre
(3), stufa, interruttore della luce e serranda del negozio (scritta CHIUSO) si usano con E **guardandoli
col mirino**; dentro si può entrare: letto, cucinino con stufa a legna (dal camino esce il fumo), tavolo,
baule, scaffale e lanterna; la luce delle lampade e della stufa resta dentro le pareti (ombre).

Se lasci aperte porta o finestre il Viandante, dopo un ritardo casuale (45 s – 3 h di gioco), le chiude; di notte, se
qualcosa viene aperto, si alza entro 1 h di gioco, chiude e torna a dormire. Le finestre hanno scuri spessi e vetro.

Giornata del Viandante: **6:00** si alza, accende la luce, chiude la porta se aperta, accende la stufa e
alza la serranda; la stufa è accesa 6–10 e 17–21 (se la accendi/spegni tu non la tocca più fino al mattino).
**Dopo le 21** spegne la stufa (se l'aveva accesa lui), abbassa la serranda, spegne la luce, chiude la porta
e va a dormire (da addormentato non parla: "Zzz…").

### Musica e audio

La colonna sonora (`assets/audio/music/legacy_of_the_first_light.ogg`, in loop) parte
all'avvio; dal tramonto (18:00) fino a poco prima dell'alba (5:54) suona `lanterns_in_the_grove.ogg`
con dissolvenza incrociata. Il volume delle musiche e quello delle voci dei personaggi si regolano dal menu Esc
("Volume musica", "Volume voci") e vengono salvati in `~/.config/planet-one/settings.txt`.
Le voci registrate (`assets/audio/voci/`) già presenti sono state generate con voci neurali Microsoft (edge-tts, online) in una
fase precedente: verificarne i termini d'uso prima di distribuire il gioco. Le **nuove** si generano solo in locale con
`scripts/gen_tts_voci.py` (Piper + ffmpeg, solo i clip mancanti; `--force` per rigenerarli tutti). Autore e licenza del brano da
indicare prima di rendere pubblico il repository.

### Dialoghi con IA (locale)

I dialoghi e le quest scritti hanno sempre la precedenza. Quando un dialogo finisce senza scelte di quest in sospeso e
la IA è pronta, compare l'opzione **"Parliamo d'altro…"**: si apre una chat in cui il personaggio risponde (in italiano) e
propone tre repliche (tasti 1–3 o clic); si può anche scrivere liberamente (Invio) e uscire con Esc. La IA non legge né
cambia quest, inventario o monete.

**Tutto gira solo sul tuo PC**: nessun servizio cloud, nessuna telemetria. Il gioco parla solo con Ollama su
`127.0.0.1:11434` (host non locali vengono rifiutati e il client non ha nemmeno il TLS). Dopo il download iniziale del
modello tutto funziona offline.

1. Installa [Ollama](https://ollama.com/download) (per ora non incluso nel gioco).
2. `./scripts/setup_ai.sh --pull` (Windows: `.\scripts\setup_ai.ps1 -Pull`) scarica il modello predefinito `gemma4:e4b`
   (5–10 GB, ~6–8 GB di VRAM; per PC più deboli `MODEL=gemma4:e2b`). Non impostare mai `OLLAMA_HOST=0.0.0.0`.
3. Nel menu Esc → "Dialoghi con IA": interruttore (spegnilo sui PC lenti: il gioco va come senza IA), stato del servizio
   e nome del modello. Le impostazioni sono in `settings.txt` (`ai_enabled`, `ai_model`, `ai_host`).

**Voce**: le risposte della IA sono lette ad alta voce con [Piper](https://github.com/rhasspy/piper), un programma locale senza rete.
`./scripts/setup_ai.sh --tts` (Windows: `.\\scripts\\setup_ai.ps1 -Tts`) installa Piper e la voce `it_IT-riccardo-x_low`; con `--all-it` scarica tutte le voci italiane
(`riccardo`, `paola`, `serena`) per provarle: la voce di ogni personaggio si sceglie con la riga `voce:` del suo file, oppure dal menu Esc.
Prima di scaricare lo script mostra il link alla model card (licenza) di ogni voce: il gioco non ridistribuisce voci. Casella "Voce IA" nel menu
per spegnerla; senza Piper il gioco mostra solo il testo. Per provare senza Piper: `PLANET_ONE_TTS_MOCK=1`.

Il carattere dei personaggi è in `assets/ai/<nome>.txt` (modificabile a mano). Per provare l'interfaccia senza modello:
`PLANET_ONE_AI_MOCK=1`. Il microfono si installa con `./scripts/setup_ai.sh --stt` (Windows: `.\scripts\setup_ai.ps1 -Stt`), che compila whisper.cpp e scarica un modello da 148 MB; serve `cmake`. Si accende dal menu Esc o con `stt on` nella shell, poi in chat si tiene premuto **F2** o «Parla». Anche la voce resta sul tuo PC e l'audio viene cancellato subito dopo la trascrizione.

### Il villaggio di Fontevivo

Un villaggio murato con due cancelli (**Porta Nord** e **Porta Sud**, aperti dalle 6 alle 21 e di notte chiusi con un varco
pedonale libero; si usano anche col mirino), una **strada a selciato** che li unisce, **stradine** verso ogni casa, una piazza con
il pozzo, la **chiesa** col campanile, case di vario tipo, locanda, forno, lampioni accesi di sera e **cartelli** con il nome fuori dai
cancelli e lungo le strade. Il nome compare in alto quando si entra e sulla mappa (M).

Tutto si modifica in F4: le **mura** sono i piazzamenti `muro` (spostando un vertice il muro si rigenera subito; il terreno spianato
attorno si aggiorna al riavvio); case, chiesa, pozzo, cartelli e lampioni sono piazzamenti dei tipi `casa-piccola`, `casa-media`,
`casa-grande`, `locanda`, `fabbro`, `forno`, `stalla`, `chiesa`, `pozzo`, `cartello` (la nota del cartello è il nome del file in
`assets/textures/cartelli/`). Nome, cancelli e strade sono in `assets/world/villaggio.txt`. `scripts/gen_villaggio.py` rigenera il layout
iniziale (edifici, lampioni, cartelli e stradine; `--preview` salva un'anteprima dall'alto), `scripts/make_selciato_texture.py` e
`scripts/make_cartelli.py` le texture (solo locale).

### Abitanti e guardie

Gli edifici hanno porte da aprire con E, piani collegati da scale, un **sottotetto** e mobili per ogni tipo di edificio; il campanile si sale fino alla cella delle campane. **Locanda e chiesa** sono sempre aperte, **forno, fabbro e arciere** negli orari di lavoro, **case e caserma** sono chiuse a chiave: ogni abitante ha la sua chiave. Di sera gli abitanti rientrano, salgono le scale e dormono nel loro letto; le guardie fuori turno dormono in caserma.

**Bauli, armadi, botti, casse e scaffali** si aprono con E: si possono prendere e posare oggetti, con un limite di peso legato alla grandezza del mobile. Anche il **faro** si visita: alloggio del guardiano, scala a chiocciola, lanterna e balconata; è chiuso a chiave.

Fontevivo è abitata: 6 abitanti (locandiere, fornaia, prete, anziana, ragazzo, contadino) e 10 guardie: 6 ai cancelli e 4 di ronda sulle mura. Ognuno ha un saluto scritto
e poi, a fine dialogo, la **chat con la IA locale** ("Parliamo d'altro…") con il proprio carattere e la propria voce (Piper). Di giorno stanno
al loro posto, la sera si ritrovano in piazza (camminando sulle strade), di notte spariscono in casa; le guardie lavorano su **tre turni** (6–14, 14–22, 22–6): ai cancelli ce n'è sempre almeno una (subito oltre l'anta aperta), il cambio
turno arriva un'ora prima, e chi finisce va a riposare alla **casa delle guardie** (fuori da Porta Nord, a est della strada). Quattro guardie fanno la **ronda** lungo tutte le mura. Il camminamento è continuo: si sale dalle scale ricavate nello spessore del muro (una per cancello, più una a est e una a ovest), si passa dentro le torri e sopra i cancelli. Di notte hanno la lanterna accesa. I dati sono in `assets/world/abitanti.txt` (posto, orari, aspetto) e `assets/ai/<nome>.txt` (persona, voce,
saluto); l'aspetto (`modello:`) è uno dei file in `assets/models/characters/`, composti da `scripts/build_characters.py` (vedi `assets/models/LICENSES.md`).

### Negozi, mercato, armi e campi

Si compra con le monete (all'inizio ne hai 30; i nemici ne lasciano qualcuna). Parlando con un venditore, a fine dialogo compare
**"Vorrei comprare qualcosa"**: pannello con articoli e prezzi, clic su "Compra", Esc per uscire. Venditori: il **Viandante** (pozioni, 12),
l'**armaiolo** Ferruccio (spada corta 45, spada lunga 90, ascia 70, scudo 55), l'**arciera** Silvia (arco 80, 10 frecce 15) e, al mercato in
piazza, Agnese (pozioni 10), Tonio (mele 2) e Nina (pesce 4).

Gli oggetti stanno nella barra in basso (tasti 1–9, 0): le pozioni, le mele e il pesce curano; **spade, ascia e arco si impugnano** premendo il
loro tasto (di nuovo per rimetterli via; il bordo dorato indica ciò che è equipaggiato), lo **scudo** riduce il danno subito del 60%. Clic sinistro =
attacco: spada corta 9 danni veloce, spada lunga 14 con più raggio, ascia 18 lenta che colpisce più nemici, arco 10 da lontano (una freccia a colpo,
verso il mirino). A mani nude 4. Per provare: `PLANET_ONE_GIVE=1` mette tutto in inventario.

Fuori dalle mura ci sono **campi** di grano e un orto (tipi `campo`, `orto` in F4: si disegnano come aree con distribuzione regolare), pagliai e
spaventapasseri. `scripts/gen_mercato.py` aggiunge mercato, botteghe e campi senza toccare il resto.

### La strada del sud, il faro e il porticciolo

Dalla Porta Sud una strada selciata scende per circa 70 m fino alla costa. A ovest, sulla scogliera, c'è il **faro** (a fasce bianche e rosse, con
il guardiano Ettore): si accende al tramonto e il fascio gira. A est c'è il **porticciolo** con il molo di legno (ci si cammina sopra), tre barche, la
casetta dei pescatori e il pescatore Gaspare. Cartelli "Faro" e "Porticciolo" lungo la strada. Lo script `scripts/gen_porto.py` genera tutto (strada,
faro, molo, barche, lampioni, cartelli) senza toccare il resto; i tipi F4 sono `faro`, `molo` (a livello del mare) e `barca`.

La **campana della chiesa** suona alle ore piene dal mattino alle 21 (7 rintocchi alle 7, 12 a mezzogiorno…): il suono è sintetizzato in locale e il
volume dipende dalla distanza dal campanile (si sente anche fuori dalle mura, più piano).

### Giorno e notte

Il giorno di gioco dura 24 minuti reali (1 ora = 1 minuto); sole e luna attraversano il cielo
e luce e cielo cambiano con l'ora. L'ora e la data (si parte dal 21 marzo, anno 0) sono nel chip in alto a sinistra; il sole sale più in alto d'estate e più in basso d'inverno,
con giorni lunghi o corti. T apre "Aspetta" (avanza anche i giorni); la velocità del tempo (×1–×8) si sceglie nel menu Esc.
Il **meteo** cambia da un giorno all'altro (sereno, nuvoloso, pioggia, temporale con lampi e tuoni, nebbia, neve) secondo le probabilità del mese; con la temperatura
(≈0 °C a gennaio, ≈20 °C a luglio) e il vento. Meteo e temperatura sono nel chip sotto la data; con pioggia o neve forti gli abitanti si riparano in casa (le guardie restano),
la neve si accumula a terra e si scioglie, la stufa del Viandante resta accesa sotto gli 8 °C. Suoni di pioggia, vento e tuono sono sintetizzati in locale. Per provarlo:
`PLANET_ONE_WEATHER=pioggia|temporale|nebbia|neve|nuvoloso|sereno`.
Le stagioni si vedono: erba verde in primavera, secca d'estate, ocra d'autunno con foglie che cadono, neve a terra, sui tetti e sugli alberi d'inverno; i fiori sbocciano da primavera. Per provarle: `PLANET_ONE_DATE=0,1,15` (anno,mese,giorno).
Per provare un'ora precisa: `PLANET_ONE_HOUR=21 cargo run`.

### Modificare i dialoghi

I testi di ogni quest sono in `assets/quests/<quest>/dialoghi.txt` (formato descritto
nell'intestazione del file). Dopo una modifica basta "Reset scena" dal menu Esc.
La logica della quest è in `src/quests/<quest>/mod.rs`.

## Struttura

```
assets/   modelli glTF, texture, audio, scene
  textures/terrain_heightmap.png   heightmap in scala di grigi (256×256)
scripts/  utility di sviluppo (es. gen_heightmap.py per rigenerare il terreno)
src/      codice del gioco (ECS Bevy)
  main.rs   stato app e cablaggio sistemi
  scene.rs  costruzione e reset della scena
  terrain.rs heightmap → mesh + collider, hot-reload
  sky.rs    cubemap procedurale (gradiente cielo)
  player.rs movimento, salto, corsa, facing, ground-check
  camera.rs orbita e follow della camera
  menu.rs   UI egui (menu Esc, hint, contatore FPS, diario)
  audio.rs   musica di sottofondo e impostazioni audio
  hut.rs     casetta-chiosco: fondamenta, porta, interni, stufa e routine del Viandante
  props.rs   appoggio a terra degli oggetti di scena
  world_props.rs alberi, fiori, panchine, lampioni da debug/placements.txt
  voice.rs   riproduzione delle voci dei personaggi
  daynight.rs orologio di gioco, sole e luna, luci e cielo
  figure.rs  personaggi: modelli glTF con scheletro, posa di riposo, oggetti agganciati alle mani
  quests/   una cartella per quest (logica) + parser dialoghi e diario
assets/quests/<quest>/dialoghi.txt   testi della quest, modificabili a mano
docs/     decisioni (ADR), convenzioni e battute per le voci (docs/voci/, generate da scripts/export_voci.py)
.github/  template issue e PR
```

## Asset e hot-reload

Il terreno è generato da `assets/textures/terrain_heightmap.png` (scala di grigi, 256×256):
ogni pixel diventa un’altezza nel mesh e nel collider Avian. Modificando la PNG mentre il
gioco gira, il terreno si ricostruisce automaticamente (`bevy/file_watcher`).

Per rigenerare la heightmap con colline diverse:

```bash
python3 scripts/gen_heightmap.py
```

## Roadmap

- [x] Fase 0 — Bootstrap
- [x] Fase 1 — Movimento (WASD + camera terza persona)
- [x] Fase 2 — Scena (terreno heightmap, cielo, luci)
- [x] Fase 3 — Entità (NPC, dialoghi, nemico)
- [x] Fase 4 — UI / Inventario / Combattimento
- [x] Fase 5 — Vertical slice RPG ("Denaro Rubato")

## Contribuire

Progetto sviluppato in modo AI-assisted con revisione umana. Apri una issue usando i template in `.github/ISSUE_TEMPLATE/`.

## Licenza

Doppia licenza: **MIT OR Apache-2.0** — vedi `LICENSE-MIT` e `LICENSE-APACHE`.
