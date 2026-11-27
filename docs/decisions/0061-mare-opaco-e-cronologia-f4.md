# ADR-0061: Mare opaco con la distanza e cronologia dell'editor F4

- Stato: accettato
- Data: 2026-11-17

## Contesto
Dalla beta v0.0.63 sono arrivate quattro segnalazioni su mare, suoni ed editor dei piazzamenti.

- **n. 22, il mare al tramonto.** Sole e luna sono sfere a 400 m dalla camera (`daynight.rs`). Il mare è un
  piano a −1 m largo 4 km, con un materiale trasparente ad alfa fisso 0,86.
  - La parte del disco fra il livello del mare e l'altezza degli occhi sta *davanti* al mare lontano, quindi
    si vede sotto l'orizzonte.
  - La parte sotto il livello del mare si vede in trasparenza.
  - Lontano si vede il fondale, e oltre la griglia dell'isola si vede il vuoto sotto il mare: c'è una fascia
    più chiara sull'orizzonte.
  - L'utente non vuole niente di visibile sotto l'orizzonte del mare, e nessun fondale oltre una decina di
    metri da sé.
- **n. 39, «Annulla ultimo» in F4.** Il pulsante faceva `items.pop()`: toglieva l'ultimo piazzamento
  dell'elenco invece di annullare l'ultima azione. Toglieva anche la casa del giocatore, che vive solo in
  memoria in fondo all'elenco e che `house::sync_placement` rimette. Così sono spariti cinque piazzamenti in
  un commit passato.
- **n. 40, scegliere un oggetto in F4.** Il mirino colpiva solo il terreno e prendeva il punto più vicino
  entro 2 m. Fra molti oggetti, o con oggetti alti, non si sceglieva quello voluto, e la camera restava
  legata al personaggio.
- **n. 26 e n. 35.** I passi sull'erba del fossato fuori dalle mura suonavano d'acqua: il terreno lì è a
  −1,4 m, più basso del mare, e `step_surface` decideva il mare dalla sola quota. Il brusio del mercato era
  sintetico, e l'utente ha una registrazione sua da usare.

## Decisione

### Mare e dischi
- **Primo shader del progetto**: `assets/shaders/distance_fade.wesl`. È un'estensione del materiale standard
  (`terrain::FadeMaterial = ExtendedMaterial<StandardMaterial, DistanceFade>`, registrata da
  `terrain::plugin`). Bevy 0.20 compila gli shader dei materiali in WESL. L'estensione ricalca il
  `pbr.wesl` di Bevy per il solo passaggio in avanti; un solo parametro `vec4` sceglie il modo.
- **Mare**: l'alfa cresce con la distanza orizzontale dalla camera, da quella del materiale (0,86) fino a 1.
  - Parte a `SEA_CLEAR_M` = 4 m ed è opaco da `SEA_OPAQUE_M` = 14 m.
  - La camera sta 4–7 m dietro al personaggio, quindi il fondale si vede fino a una decina di metri da lui.
  - Onde (`water::animate_water`) e mappa (`map::sea_on_map`) lavorano sul `base` del materiale nuovo.
- **Sole e luna**: lo stesso shader in modo «orizzonte».
  - Il frammento sparisce sotto l'orizzonte del mare visto dalla camera: la depressione è l'altezza sul mare
    divisa per 1000 m, il piano lontano della camera.
  - I dischi diventano trasparenti, cioè non scrivono la profondità e non entrano nel prepass di SSAO, così
    lo `discard` vale per tutto il disegno.
  - Al tramonto il disco scende dietro la linea del mare.
- **Niente normali dal prepass per il mare.**
  - Bevy fa leggere le normali del prepass (`LOAD_PREPASS_NORMALS`) a ogni mesh, anche trasparente,
    quando la camera ha il prepass delle normali (con SSAO). Le toglie solo ai materiali con trasmissione.
  - Il mare prendeva così le normali del fondale che ha sotto. Oltre la griglia dell'isola, dove sotto non
    c'è nulla, prendeva il valore vuoto, cioè una normale verso il basso.
  - La fascia chiara sull'orizzonte era questa: un'altra luce, non il fondale in trasparenza.
  - `DistanceFade::specialize` toglie la definizione: il mare usa le sue normali delle onde, che ora si
    vedono davvero.
- **Onde vere sul mare**: con le normali giuste la piastrella delle onde da 16 m faceva macchie di sole
  larghe e sfocate. Ora è da 8 m (`SEA_WAVE_TILE_M`) e, oltre i 30–120 m, la normale sfuma verso l'alto
  per non far vedere la ripetizione. La foto di riferimento `movimento/mare.png` è aggiornata.
- Niente nuove impostazioni grafiche: lo shader costa due `smoothstep` per frammento del mare.

### Passi
- `TerrainField` tiene la maschera del mare aperto (`in_sea`, lo stesso riempimento dal bordo che disegna il
  mare). `step_surface` dà acqua o molo solo sul mare vero; una conca dell'entroterra resta erba.
- Lo stato del pilota ha `giocatore.passi` (ultima superficie) per i copioni.

### Brusio del mercato
- «Vita tra i banchi» (mp3 fuori da git; generata con l'IA di Google, come dicono i suoi metadati C2PA con
  filigrana SynthID, e non registrata dall'autore come scritto in origine: vedi `CREDITS.md`) è convertita con ffmpeg in
  `assets/audio/ambience/vita_tra_i_banchi.ogg` (Vorbis q5).
  - L'originale sfuma a zero negli ultimi 14 s. Il ciclo usa 0–160 s con gli ultimi 6 s sfumati sopra i
    primi 6, così il giro non ha buchi.
  - Si carica dall'`AssetServer` al posto di `synth_market()`, con lo stesso livello per distanza, ora e
    pioggia. Il guadagno scende da 0,6 a 0,35, perché il file è più forte del mormorio sintetico.

### Editor F4
- **Cronologia senza limite** (`DebugPlace::undo`/`redo`).
  - `changed()` mette nella pila lo stato del file di *prima* della modifica, come fotografia dei piazzamenti
    senza quelli in memoria (`runtime`: la casa del giocatore). Una modifica nuova svuota la pila del ripeti.
  - Annulla e ripeti rimettono la fotografia e lasciano i piazzamenti in memoria dove sono. Gli id non tornano
    indietro.
  - «Svuota» non toglie più la casa del giocatore.
  - Comandi: Ctrl+Z, Ctrl+Y e Ctrl+Maiusc+Z, più i pulsanti «↶ Annulla (n)» e «↷ Ripeti (n)».
  - I tasti si leggono in `PreUpdate` e si tolgono al gioco: Z da solo cambierebbe l'incantesimo.
- **Scelta col mirino**.
  - Il raggio colpisce anche i modelli veri (`MeshRayCast` di Bevy), saltando terreno, mare, giocatore e
    cielo. Il punto colpito sceglie i candidati: le cose piccole entro 2 m, gli edifici fino a 9 m dal centro
    (si mira un muro) ma dopo le cose piccole.
  - **Tab** o la **rotella** passano al candidato successivo; l'etichetta rossa dice quale è, e quanti sono.
  - Gomma, Sposta, Ruota, X e il clic destro in mappa usano il candidato scelto.
- **Elenco**: ricerca per id, tipo o nota; «Vai e seleziona» porta la camera libera davanti all'oggetto e
  passa a Sposta.
- **Camera libera** (F6 o casella nel pannello, solo con F4 acceso).
  - Vola staccata dal giocatore: WASD, E/Spazio su, Q/C giù, Maiusc ×4.
  - I suoi tasti si tolgono al gioco, quindi il personaggio resta fermo.
  - Il mouse la gira senza cambiare la visuale del giocatore. Spegnendo F4 si spegne anche lei.

## Conseguenze
- Il mare lontano è opaco e ha la stessa luce dappertutto: niente più fascia chiara all'orizzonte né
  fondale lontano. Vicino alla riva il fondale si vede come prima. Le increspature della mappa delle onde
  ora si vedono (prima il mare mostrava le pieghe del fondale).
- Sole e luna non si vedono più sotto l'orizzonte. Dalle alture la linea del mare scende di poco, e il taglio
  la segue.
- Il primo shader del progetto va compilato su ogni scheda: provato su RTX 3080 Ti (Vulkan) e Intel Arc
  integrata (Mesa).
- Un errore nel file `.wesl` non ferma il gioco, ma il mare e i dischi non si disegnano: lo si vede nel log
  (`ERROR` di `bevy_render`) e nelle foto dei copioni.
- Annullare in F4 riscrive `debug/placements.txt` com'era. La cronologia vive solo finché il gioco è aperto e
  si azzera al cambio di regione.
- Il raggio sui modelli guarda le mesh che restano sulla CPU. Quelle solo-GPU dell'ADR 0056 (terreno, grandi
  mesh unite) non si colpiscono, e per loro vale il punto del terreno.

## Misure
Copioni `fps` e `b-mare-tramonto` a 1080p senza finestra, due corse alternate per `main` (v0.0.66.0) e per
questo ramo, con gli asset del progetto. Durante le corse giravano altri 4–6 processi del gioco di altre
sessioni, quindi i valori dell'1% peggiore non sono affidabili.

| Scheda | Copione | main (FPS medi) | ramo (FPS medi) |
|---|---|---|---|
| RTX 3080 Ti | `fps` | 34, 26 | 38, 38 |
| RTX 3080 Ti | `b-mare-tramonto` | 15, 34 | 43, 46 |
| Arc integrata (Mesa 25, Integrata) | `fps` | 53, 73 | 76, 59 |
| Arc integrata (Mesa 25, Integrata) | `b-mare-tramonto` | 66, 42 | 41, 88 |

Nessun calo misurabile e nessun errore dello shader su nessuna delle due schede. Nelle foto del tramonto,
su `main` il disco del sole si vede attraverso il mare e la fascia chiara arriva all'orizzonte. Sul ramo il
sole è tagliato alla linea del mare, la fascia non c'è e le onde sono visibili.
