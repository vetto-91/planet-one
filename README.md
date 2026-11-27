# Planet One

RPG 3D open source in terza persona, con grafica realistica PBR. Scritto in **Rust** con il motore
**[Bevy](https://bevy.org)** 0.20, fisica [Avian](https://github.com/Jondolf/avian) e interfaccia
[egui](https://github.com/emilk/egui). Un naufrago si risveglia sulla spiaggia dell'Isola di Fontevivo
senza ricordare nulla: la storia del Prologo lo porta fino al Continente del Nord.

> **Stato**: ultimo rilascio **`v0.0.69.1`** (fase 15.13, l'IA locale avviata e chiusa dal gioco, ADR 0063).
> L'Isola di Fontevivo ha la storia completa del Prologo: 5 missioni principali, 14 secondarie e 11 eventi.
> Il Continente del Nord è esplorabile.
> Fasi e programma: [docs/ROADMAP.md](docs/ROADMAP.md). Storia dei rilasci: [CHANGELOG.md](CHANGELOG.md).
> Crediti di modelli, texture, audio e voci: [CREDITS.md](CREDITS.md).

## Installazione

Dalla pagina [Releases](https://github.com/vetto-91/planet-one/releases) scarica l'archivio della tua
piattaforma ed estrailo. L'eseguibile e la cartella `assets/` devono restare insieme.

| Piattaforma | Archivio | Primo avvio |
|---|---|---|
| Linux x86_64 | `planet-one-vX-linux-x86_64.tar.gz` | `./planet-one` |
| Windows x86_64 | `planet-one-vX-windows-x86_64.zip` | doppio clic su `planet-one.exe`. Se SmartScreen avvisa: «Ulteriori informazioni» → «Esegui comunque» (eseguibile non firmato) |
| macOS (Intel e Apple Silicon) | `planet-one-vX-macos-universal.tar.gz` | eseguibile non firmato: tasto destro su `planet-one` → Apri, oppure `xattr -dr com.apple.quarantine <cartella>` e poi `./planet-one` |

Requisiti:
- **Linux**: `libudev`, `alsa` e un driver Vulkan, di solito già presenti.
- **Windows 10/11**: un driver video con Vulkan o DirectX 12.
- **macOS 11+**: Metal.

Sulle schede video integrate il gioco sceglie da solo il preset grafico «Integrata».

Il gioco parte a schermo intero; **F11** passa alla finestra. Le impostazioni stanno in `settings.txt`:
- Linux: `~/.config/planet-one/`;
- Windows: `%APPDATA%\planet-one\`;
- macOS: `~/Library/Application Support/planet-one/`.

Su Mac i tasti F richiedono Fn, oppure l'opzione «Usa F1, F2 ecc. come tasti funzione standard».

## Dialoghi con IA (solo locale)

I dialoghi e le missioni scritti hanno sempre la precedenza. Quando un dialogo finisce senza scelte di
missione in sospeso e la IA è pronta, compare **«Parliamo d'altro…»**: si apre una chat in cui il
personaggio risponde in italiano, con il suo carattere, e propone tre repliche (tasti 1–3 o clic). Si può
anche scrivere liberamente (Invio) e uscire con Esc. La IA non legge né cambia missioni, inventario o monete.

**Tutto gira sul tuo PC**: nessun servizio cloud, nessuna telemetria. Dopo i download iniziali funziona
anche offline (ADR 0005).

### Installare la IA

Dalla cartella del gioco (o da `scripts/` se lavori sul codice):

```bash
./setup_ai.sh --ollama        # Ollama del gioco (~1,7 GB) e modello gemma4:e4b (~10 GB)
./setup_ai.sh --tts           # voce: Piper e la voce italiana it_IT-riccardo-x_low
./setup_ai.sh --stt           # microfono: whisper.cpp (serve cmake) e un modello da 148 MB
```

- **Windows**: `.\setup_ai.ps1 -Ollama`, `-Tts`, `-Stt`.
- **Schede AMD**: aggiungi `--amd` (`-Amd`).
- **PC più deboli**: `MODEL=gemma4:e2b ./setup_ai.sh --ollama`, poi scegli il modello nel menu Esc.
- **Senza scaricare nulla**: `--dry-run` mostra cosa verrebbe scaricato.

Ogni download viene dalle fonti ufficiali, va confermato e finisce nella cartella dati del gioco:
- Linux: `~/.local/share/planet-one/ai/`;
- Windows: `%LOCALAPPDATA%\planet-one\ai\`;
- macOS: `~/Library/Application Support/planet-one/ai/`.

Niente viene installato nel sistema. Prima di scaricare una voce lo script mostra il link alla sua licenza.

### Come gira

**Ollama vive solo mentre giochi** (ADR 0063):
- Con la IA accesa, il gioco avvia il suo Ollama all'apertura e lo chiude all'uscita. Se spegni la IA dal
  menu, si chiude subito.
- Ascolta solo su `127.0.0.1`, su una porta diversa a ogni avvio, mai in rete.
- Se il gioco va in crash, un piccolo processo di guardia (su Windows il sistema stesso) chiude Ollama e
  il modello caricato. Così memoria e scheda video si liberano.

Se l'Ollama del gioco non c'è, il gioco usa un Ollama già avviato sul PC all'indirizzo `ai_host`
(`127.0.0.1:11434`). Il client rifiuta gli indirizzi non locali e non ha nemmeno il TLS.

**Voce e microfono**:
- Le risposte sono lette con [Piper](https://github.com/rhasspy/piper), con la stessa voce che il
  personaggio ha nei dialoghi scritti.
- In chat, tenendo premuto **F2** o «Parla», parli al personaggio. La trascrizione la fa
  [whisper.cpp](https://github.com/ggml-org/whisper.cpp) sul tuo PC e l'audio viene cancellato subito dopo.

**Menu Esc → «Dialoghi con IA»**: interruttore (spegnilo sui PC lenti, il gioco va come senza IA), stato,
modello, voce e microfono.

Richiede circa 6–8 GB di memoria video per `gemma4:e4b`. Con la IA in generazione la scheda video è
condivisa col gioco.

## Controlli

| Tasto | Azione |
|---|---|
| W/A/S/D o frecce | Movimento |
| Mouse | Orbita della camera (sensibilità nel menu) |
| Maiusc | Corsa |
| Spazio | Salto |
| X | Scatto in avanti (talento dell'Atletica, costa vigore) |
| Ctrl (in acqua profonda) | Apnea: ci si immerge (talento del Nuoto) |
| V / tasto destro tenuto | Prima persona fissa / prima persona finché lo tieni |
| E | Parla con chi hai vicino; guardando un oggetto col mirino lo usa (porte, finestre, stufe, luci, mobili, cancelli) |
| Invio, Su/Giù | Nel dialogo: continua, cambia scelta |
| Clic sinistro | Attacco con l'arma impugnata; con l'arco tira verso il mirino |
| 1–9, 0 | Usa o impugna l'oggetto dello slot (pozioni, cibo, armi, scudo, arco) |
| Z / Q | Sceglie l'incantesimo / lo lancia |
| F / G | Alle spalle di un abitante: borseggio / gli infili cinque monete. Di fronte, G: il regalo che gradisce |
| R | Spacca pietra e taglia legname; al Poggio del Naufrago costruisce la casa |
| Spazio (scasso) | Ferma l'ago sulla tacca d'oro, un perno alla volta |
| I | Inventario: categorie, ricerca, filtri, peso, tasti 1–9 e 0 |
| C | Personaggio: statistiche, abilità, talenti |
| J | Diario: missioni da fare e completate, «Volti e segreti» |
| Tab (tenuto) | Obiettivi della missione seguita |
| M | Mappa: rotella o +/- = zoom, frecce/WASD = sposta, C = su di te, clic su un luogo noto = viaggio veloce |
| T | Aspetta: fa passare ore o giorni |
| H | Nasconde / mostra l'interfaccia |
| F2 (tenuto, in chat) | Parla al personaggio col microfono |
| F5 / F9 | Salvataggio rapido / ricarica |
| F11 | Schermo intero sì/no (a schermo intero il cursore è catturato) |
| F12 | Segnala bug: schermo, stato, log e nota in una cartella |
| Esc | Menu: salvataggi, impostazioni, grafica, audio, IA |
| \\ | Shell dei trucchi (vedi «Per sviluppatori») |

Con `debug on` nella shell si attivano F3 (collider), F4 (piazzamenti) e F6 (camera libera).

## Il gioco

### Il risveglio e il personaggio

Una partita nuova comincia sulla spiaggia a est del molo, tra i rottami, senza nome:
- guarda nella pozza d'acqua ferma tra le rocce per scoprire la tua età;
- raccogli tra i rottami la cosa che ti chiama: è la tua indole. Vale quella nel mirino; Esc o «Lascialo
  lì» la rimettono giù;
- di' il tuo nome a Gaspare, al molo.

Il personaggio cresce **facendo**: correre, nuotare, combattere, parlare ed esplorare allenano sedici
**abilità**, con un grado da 1 a 100.
- Gradi e missioni fanno salire il **livello**, che dà un punto statistica, un Nodo d'Oro e un po' di vita.
- Ai gradi 15, 30, 50, 75 e 100 un Nodo d'Oro attiva un rango del **talento** dell'abilità.
- Sopra la barra degli oggetti ci sono **magia**, **vita** e **vigore**. Il vigore cala correndo, saltando
  e colpendo e torna da solo.
- Si portano **30 kg**; i quattro zaini del fabbro arrivano a 60.

**Incantesimi**: da subito hai la Sfera di luce e il Dardo di Luce Viva. Con la pratica arrivano il Soffio
di cura, il Passo leggero e lo Scudo etereo.

### La storia del Prologo

La storia principale dell'isola è in cinque missioni (diario: `J`):

1. **Il Nome sulla Sabbia**: i rottami, Gaspare al molo, la salita a Fontevivo, il Viandante.
2. **Denaro Rubato**: il ladro ha preso dodici monete al Viandante. Con un'arma in pugno il ladro combatte;
   senz'armi gli si può parlare (ti ascolta se hai letto la tavoletta al suo campo) o stringere un patto e
   dividere l'oro.
3. **Le Voci delle Pietre Antiche**: Gino al mulino, le rovine, il santuario, la torre antica di notte.
4. **Sotto l'Arco dei Sei Gradoni**: Don Anselmo, poi la grotta a est fino al laghetto e in fondo al cunicolo.
5. **Oltre il Velo del Nord**: la Sera delle Lanterne in piazza (dopo le 20) e la partenza prima dell'alba.

Gli abitanti propongono **14 missioni secondarie** quando ci parli; si fanno in qualsiasi ordine. Girando
per l'isola capitano **11 eventi** legati a luogo, ora e meteo (una nave fantasma, una bottiglia sulla
spiaggia, il serpente di luce…). Un luogo compare sulla mappa quando ci arrivi; da allora un clic sul suo
nome ti ci riporta.

Alcune missioni hanno compagni, come Silvia e la Guardia Dino: ti seguono, combattono e viaggiano con te.

### Fontevivo e i suoi abitanti

Un paese murato con due cancelli. La Porta Nord e la Porta Sud sono aperte dalle 6 alle 21; di notte le
apre chi passa e si richiudono da sole.

Dentro ci sono la piazza col pozzo e il mercato, la chiesa col campanile che suona le ore, la locanda, il
forno, il fabbro, l'arciera e le case. Locanda e chiesa sono sempre aperte, le botteghe negli orari di
lavoro, le case sono chiuse a chiave. Fuori ci sono il faro di Ettore, il porticciolo di Gaspare, la casetta
del Viandante, campi, orti, mulino e fattoria.

Ci vivono **15 abitanti e 10 guardie**, oltre al Viandante:
- ognuno ha casa, lavoro, orari e un letto. Di sera si ritrovano in piazza e di notte dormono;
- le guardie fanno tre turni ai cancelli e la ronda sulle mura, con la lanterna di notte;
- con pioggia o neve forti gli abitanti si riparano in casa;
- chi ti parla si volta verso di te, e un passante si ferma se lo guardi;
- dopo il dialogo scritto, ognuno ha la sua chat con la IA.

**Negozi**: si paga con le monete. A fine dialogo con un venditore compare «Vorrei comprare qualcosa».
- Il **Viandante** vende pozioni.
- **Ferruccio** vende armi, scudo, zaini e grimaldelli.
- **Silvia** vende archi e frecce.
- Al mercato ci sono **Agnese**, **Tonio** e **Nina**.

**Armi**: spade, ascia e arco si impugnano col loro tasto; lo scudo riduce il danno. Con il talento della
Lama un nemico ferito gravemente getta l'arma e si arrende.

**Altre cose da fare**:
- **La tua casa**: a sud-ovest del paese, fuori dalle mura, c'è il Poggio del Naufrago. Con `R` raccogli
  pietra e legname (per il legname serve un'ascia), poi posi le fondamenta e alzi la casa. Dentro, i mobili
  sono il tuo deposito.
- **La Prova dell'Arco**: oltre il ponte a est del paese, tre anelli da centrare in fila con tre frecce.
  Silvia ti presta l'arco se non ne hai uno.
- **Scasso e borseggio**: i bauli chiusi si aprono col grimaldello. Alle spalle di un abitante, `F` gli
  sfila qualche moneta; se se ne accorge paghi una multa.
- **Karma e segreti**: quello che fai pesa sulla Bilancia di Ma'at e cambia i prezzi. Ogni abitante si fida
  di te a modo suo: parlagli, portagli il regalo che gradisce, aiutalo. A un amico confida il suo segreto.
- **Mobili**: bauli, armadi, botti, casse e scaffali si aprono con E, con una capienza secondo la
  grandezza. Un mobile vuoto lo dice già nel prompt, «(Vuoto)».

### L'isola

Fuori dalle mura ci sono:
- alture con la vista sul paese, scogliere, spiagge, il ruscello col laghetto, sentieri;
- boschi di abeti e latifoglie, radure, fiori ed erba alta;
- posti da scoprire: il campo del ladro, la torre in rovina (si sale fino in cima), le rovine del tempio, la
  grotta nella scogliera, l'accampamento dei banditi, il santuario;
- animali: mucche e pecore al pascolo, galline in paese, gabbiani al porto, uccelli, pesci, farfalle;
- in acqua: con l'apnea si raggiunge la cassa sommersa al largo del relitto.

**Giorno e notte**: un giorno di gioco dura 24 minuti reali e si parte il 21 marzo dell'anno 0. Sole e luna
seguono le stagioni.

**Meteo**: cambia ogni giorno secondo il mese (sereno, nuvoloso, pioggia, temporale, nebbia, neve), con
temperatura e vento.

**Stagioni**: si vedono su erba, chiome, fiori, foglie che cadono e neve su tetti e alberi.

### Il Continente del Nord

Finita la storia del Prologo, Gaspare ti porta in barca alla **Piana delle Brume**. In alternativa c'è la
via sotto il mare, oltre la Porta Sommersa. La traversata è letta da una voce narrante.

Dall'Approdo delle Brume il sentiero attraversa il bosco di querce e betulle, costeggia il Grande Lago e
arriva a **Fontebruma**. Ci vivono 22 abitanti, fra il paese, il lago, l'eremo, le famiglie del bosco e la
collina.

Inventario, missioni, data e ora viaggiano con te; ogni regione resta com'era. Le missioni del continente
arrivano nei prossimi passi.

### Musica, suoni e voci

- **Musiche**: due, create dall'autore con Gemini, una di giorno e una di notte, con dissolvenza.
- **Brusio del mercato**: generato con l'IA di Google.
- **Altri suoni**: tutti sintetizzati dal gioco. Onde, ruscello, vento, uccelli, grilli, pioggia, tuoni,
  campane, incantesimi e passi diversi su erba, terra, sabbia, pietra, legno, acqua e neve.
- **Voci dei personaggi**: generate in locale con Piper.
- **Volumi**: musica, ambiente e voci si regolano nel menu Esc.

Dettagli e licenze in [CREDITS.md](CREDITS.md).

### Salvataggi

Il menu principale ha Continua, Nuova partita, Carica partita e Impostazioni. Il gioco salva da solo ogni
5 minuti e all'uscita; **F5** salva al volo e **F9** ricarica.

Ogni salvataggio è un file di testo modificabile in `saves/<personaggio>/`, nella cartella dati:
- Linux: `~/.local/share/planet-one/`;
- Windows: `%LOCALAPPDATA%\planet-one\`;
- macOS: `~/Library/Application Support/planet-one/`.

## Per sviluppatori

### Compilare

Serve Rust stable 1.97 o più recente ([`rustup`](https://rustup.rs)). Su Linux servono anche gli header
di `libudev` e `alsa`.

```bash
git clone git@github.com:vetto-91/planet-one.git
cd planet-one
cargo run --release  # la prima compilazione scarica e compila Bevy: alcuni minuti
cargo run            # dev (codice a opt-level 1, dipendenze a 3)
cargo test           # test logici e geometrici, con la prova di camminata in fisica vera
cargo fmt            # obbligatorio prima di ogni commit
cargo clippy         # deve essere pulito prima di ogni commit
```

Regole di lavoro, branch, commit e l'elenco commentato di tutti i moduli: [AGENTS.md](AGENTS.md). Le scelte di
progetto sono negli ADR di [docs/decisions/](docs/decisions/README.md).

### Struttura

```
src/        codice del gioco (ECS Bevy): un modulo per sistema (scena, terreno, giocatore, camera, abitanti,
            missioni, meteo, audio, IA locale in src/ai/…); l'elenco commentato è in AGENTS.md
assets/     modelli glTF, texture, audio, voci, mondo (assets/world/*.txt) e testi delle missioni (assets/quests/)
scripts/    generatori degli asset e strumenti (beta_check.sh, setup_ai.sh, package_linux.sh, gen_*.py, make_*.py)
prove/      copioni del pilota automatico (scripts/beta_check.sh)
docs/       ROADMAP, versioni, decisioni (ADR), piano beta, battute per le voci
debug/      piazzamenti dell'isola e del continente (modalità F4)
.github/    template di issue e PR, rilascio di Windows e macOS
```

### Modificare il mondo e i testi

I testi sono file di testo modificabili a mano, ognuno col suo formato descritto nell'intestazione.

**Storia e personaggi**:
- **Missioni**: `assets/quests/prologo/missioni.txt` (principali), `secondarie.txt` ed `eventi.txt`.
  Shell: `missioni` mostra lo stato, `missione mp03 2` salta a una tappa.
- **Dialoghi**: `assets/quests/<quest>/dialoghi.txt`. Dopo una modifica basta «Reset scena» dal menu Esc.
- **Abitanti**: posto, orari, aspetto e letto in `assets/world/abitanti.txt`; carattere e voce per la IA in
  `assets/ai/<nome>.txt`.
- **Fumetti e segreti**: battute fuori dalle missioni in `assets/world/battute.txt`; segreti e regali in
  `assets/world/segreti.txt`.

**Luoghi**:
- **Isola**: forma, boschi, animali e luoghi in `assets/world/isola.txt`.
- **Paese**: nome, cancelli e strade in `assets/world/villaggio.txt`.
- **Edifici e oggetti**: case, mura, lampioni e oggetti sono piazzamenti di `debug/placements.txt`
  (modalità F4). Gli script `gen_villaggio.py`, `gen_mercato.py` e `gen_porto.py` generano il layout
  iniziale.
- **Continente**: `assets/world/nord/` e `debug/placements-nord.txt`, generati da `scripts/gen_nord.py`.
- **Terreno**: è la heightmap `assets/textures/terrain_heightmap.png` (`scripts/gen_heightmap.py`). Si
  ricostruisce da sola se la modifichi a gioco aperto.

**Voci**: dopo aver cambiato le battute si rigenerano con `scripts/gen_voci_prologo.py`,
`gen_tts_voci.py` e `gen_voci_abitanti.py`.

### Shell dei trucchi (tasto `\`)

Giocando normalmente non ci sono trucchi. Il tasto `\` apre una shell; Esc o `\` la chiudono. A ogni avvio
si riparte con tutto spento. `aiuto` mostra i comandi:

| Comando | Effetto |
|---|---|
| `god`, `noclip`, `invisibile` | Nessun danno; volo senza collisioni (Spazio sale, Ctrl scende); invisibile ai nemici |
| `tp x y z`, `tp chiesa`, `tp porta nord`, `tp marta`, `tp casa-media 1` | Teletrasporto |
| `passepartout [off]`, `chiavi`, `dai mela 3` | Chiave che apre tutto, chi ha quali chiavi, oggetti |
| `f3`, `f4`, `debug on`, `coordinate` | Collider, piazzamenti, tasti di debug, coordinate nell'HUD |
| `ora 21.5`, `meteo neve`, `meteo auto` | Ora e meteo |
| `missioni`, `missione <id> <tappa>`, `veloce [luogo]`, `regione <isola\|nord>` | Storia, viaggio veloce e regioni |
| `abilita`, `livello <n>`, `talento <abilità>`, `stat forza 15` | Personaggio |
| `ia [on\|off]`, `ia modelli`, `ia modello <nome>` | Dialoghi IA, sempre e solo locali |
| `tts [on\|off]`, `tts voci`, `tts voce <nome\|auto>` | Voce IA |
| `stt [on\|off]`, `stt modelli`, `stt modello <nome>` | Microfono nella chat |
| `vai`, `dove`, `guarda`, `foto`, `dump`, `copione <file>` | Comandi del pilota automatico |

### Debug piazzamenti (F4)

F4, attivo dopo `debug on`, apre il pannello per mettere alberi, case o qualunque oggetto. In 3D il punto è
quello del mirino; nella mappa (M) si sceglie col clic.

Comandi:
- clic sinistro conferma, trascinare dà la direzione;
- **R** ruota di 15°, **X** elimina;
- **Tab** o la rotella passano fra gli oggetti vicini;
- **Ctrl tenuto** libera il mouse per usare il pannello;
- **Ctrl+Z** e **Ctrl+Y** annullano e ripetono, senza limite di passi.

Strumenti:
- **Piazza**: singolo, gruppo, rettangolo, cerchio o poligono, con distribuzione regolare o casuale.
- **Gomma**, **Sposta** e **Ruota**.

**F6** accende la camera libera. Nell'elenco si cerca per id, tipo o nota.

Ogni modifica riscrive `debug/placements.txt` (blocchi `[P-nnnn]`; il nord è −Z), che viene riletto
all'avvio.

### Variabili d'ambiente

Con una qualunque di `POS`, `HOUR`, `LOOK`, `SHELL`, `SCRIPT` si entra subito in partita, a finestra.

| Variabile | Effetto |
|---|---|
| `PLANET_ONE_HOUR=21.5`, `PLANET_ONE_DATE=0,12,21` | Ora e data iniziali (`anno,mese,giorno`) |
| `PLANET_ONE_WEATHER=neve` | Meteo forzato (`sereno`, `nuvoloso`, `pioggia`, `temporale`, `nebbia`, `neve`) |
| `PLANET_ONE_POS=x,y,z`, `PLANET_ONE_LOOK=yaw,pitch[,dist[,1]]` | Posizione del giocatore e camera (1 = prima persona) |
| `PLANET_ONE_SHELL="god on;ora 22"` | Comandi della shell all'avvio |
| `PLANET_ONE_SCRIPT=prove/giro.txt` | Pilota automatico (vedi sotto) |
| `PLANET_ONE_SPEED=4` | Tempo di gioco accelerato |
| `PLANET_ONE_SIZE=1280x720`, `PLANET_ONE_HEADLESS=1` | Finestra fissa; senza finestra (la camera disegna in un'immagine) |
| `PLANET_ONE_GFX=basso` | Grafica delle prove (`basso`, `medio`, `alto`) senza toccare le impostazioni |
| `PLANET_ONE_DATA_DIR=/tmp/prova`, `PLANET_ONE_OUT=cartella` | Dati di una partita di prova; dove il pilota scrive foto e rapporto |
| `PLANET_ONE_GIVE=1` | Tutte le armi, frecce e 200 monete |
| `PLANET_ONE_SESSION=Nome`, `PLANET_ONE_MENU=1`, `PLANET_ONE_WINDOWED=1` | Partita senza menu; menu comunque; a finestra |
| `PLANET_ONE_AI_MOCK`, `PLANET_ONE_TTS_MOCK`, `PLANET_ONE_PIPER`, `PLANET_ONE_STT_MIC` | Prove dell'IA senza modello, senza Piper, con un altro Piper, con un WAV al posto del microfono |
| `PLANET_ONE_OLLAMA=/percorso/ollama` | Ollama da avviare al posto di quello nella cartella dati |
| `PLANET_ONE_ANIM=sprint`, `PLANET_ONE_DELAY_SCALE=0.1`, `PLANET_ONE_SCAN_WALLS=1` | Animazione forzata del giocatore; ritardi del Viandante; diagnostica delle mura (`[SCAN]`) |

I log di riferimento sono `[INPUT]`, `[EVENT]`, `[CURSOR]`, `[QUEST]` e `[AI]`:

```bash
cargo run 2>&1 | tee run.log
grep "\[QUEST\]" run.log
```

### Pilota automatico e segnalazioni

Il gioco si può giocare da un **copione** di testo, senza tastiera né mouse: serve a riprodurre un bug
sempre allo stesso modo. Si scrive un comando per riga (`#` = commento); ogni riga che non è del pilota va
alla shell.

```text
god on
attendi npc:viandante:modo = Ready 90
vai viandante
guarda viandante
premi e
avanza
scegli 1
verifica quest = SeekThief
foto dopo
esci
```

| Comando | Effetto |
|---|---|
| `vai <x> <z>`, `vai <nome>`, `corri …` | Cammina davvero (fisica vera); se resta bloccato lo scrive |
| `guarda <yaw> <pitch>`, `guarda <nome> [distanza]`, `mira <oggetto>` | Mette il mirino su un punto, un abitante, un edificio o un oggetto |
| `premi <tasto> [s]` | Tiene premuto un tasto (`w`, `spazio`, `shift`, `e`, `invio`, `f5`, `click`, `+`, `-`…) |
| `avanza`, `scegli <n o testo>` | Fa scorrere il dialogo; sceglie una risposta |
| `aspetta <s>`, `attendi … [s]`, `mai … [s]` | Attende; la condizione diventa vera entro `s` secondi / non lo diventa mai |
| `verifica <campo> <op> <valore>` | Controllo (`=`, `!=`, `<`, `>`, `contiene`) su `vita`, `x/y/z`, `quest`, `monete`, `dialogo`, `ora`, `fps`, `npc:<nome>:<campo>`, `prop:<id>:<campo>`, `/puntatore/json`… |
| `foto [nome]`, `dump [nome]`, `giro-foto <nome> <n>` | Screenshot; stato del gioco in JSON; foto tutt'attorno |
| `camera <yaw> <pitch> [dist]`, `ruota <gradi>`, `prima-persona [off]` | Punto di vista |
| `compra`, `prendi`, `chiudi`, `di <frase>` | Negozi, mobili, frase nella chat con la IA |
| `registra <nome>`, `ferma` | Registra l'audio (Linux); `scripts/ascolta.py` lo descrive |
| `esci` | Rapporto e codice d'uscita 1 se qualcosa è fallito |

```bash
scripts/beta_check.sh                      # tutti i copioni di prove/, rapporto in prove/out/RAPPORTO.md
scripts/beta_check.sh prove/giro.txt       # uno solo
scripts/beta_check.sh --aggiorna           # le foto di questa corsa diventano i riferimenti
scripts/beta_check.sh --con-finestra       # con la finestra vera (foto con HUD e pannelli)
scripts/beta_check.sh --con-casse          # l'audio esce dalle casse
scripts/beta_check.sh --risoluzioni 4k     # 720p, 1080p, 1440p, 4k, LxA o «tutte»
```

Le prove non disturbano chi usa il PC: girano senza finestra e con l'audio in una scheda virtuale di
PulseAudio. Un copione può chiedere variabili proprie con righe `# env: CHIAVE=valore`.

Le foto si confrontano con quelle di `prove/riferimento/`: una differenza RMSE oltre `SOGLIA=0.12` è un
fallimento. Per ogni bug riprodotto si scrive un copione nuovo.

**F12, «Segnala bug»**: salva in `~/.local/share/planet-one/segnalazioni/<data-ora>/` lo schermo, lo stato
del gioco, le ultime righe del log e una `nota.txt` da compilare. Basta mandare quella cartella.

### Rilasci

Le versioni hanno il formato `vMAJOR.MINOR.FEATURE.BUGFIX` ([docs/VERSIONING.md](docs/VERSIONING.md)). I
tag si mettono solo su `main`, dopo la verifica manuale della build.

- **Linux**: il pacchetto si costruisce in locale con `scripts/package_linux.sh vX.Y.Z.W`.
- **Windows e macOS**: li costruisce GitHub Actions a richiesta (`.github/workflows/release.yml`, ADR 0004).

Ogni pacchetto contiene il gioco, `assets/`, README, CREDITS, le licenze e lo script `setup_ai` della sua
piattaforma.

## Contribuire

Progetto sviluppato in modo AI-assisted con revisione umana. Apri una issue con i template di
`.github/ISSUE_TEMPLATE/`.

## Licenza

Il **codice** è sotto doppia licenza **MIT OR Apache-2.0**: vedi `LICENSE-MIT` e `LICENSE-APACHE`.

Gli **asset** hanno ciascuno la propria licenza:
- CC0 per i modelli di Quaternius e le texture di Poly Haven;
- i termini di Google per i contenuti generati con Gemini;
- le licenze delle voci di Piper.

L'elenco completo, con autori e link, è in [CREDITS.md](CREDITS.md).
