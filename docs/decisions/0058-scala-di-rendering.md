# ADR-0058: Scala di rendering

- Stato: accettato
- Data: 2026-11-16

## Contesto
Con l'ADR 0057 il preset «Integrata» porta l'Arc integrata (Intel Graphics ARL, Mesa 25.0.7) a 25 FPS a
1080p e 36 a 720p, con la finestra. Spegnere anche le ombre del sole dà solo 3 FPS in più: il costo che
resta è il numero di pixel disegnati.

La leva è la **scala di rendering**: disegnare la scena più piccola e ingrandirla sulla finestra. L'ADR 0057
l'aveva scartata per tre motivi:
- bevy_egui 0.43 aggancia il contesto principale alla prima camera, cioè a quella della scena: se la scena
  va su un'immagine, anche l'interfaccia ci andrebbe, sfocata;
- la mappa (M) usa la stessa camera;
- le foto del pilota e la modalità senza finestra (`PLANET_ONE_HEADLESS`) disegnano già su un'immagine.

`MainPassResolutionOverride` di Bevy non serve: riduce solo il passaggio principale e lascia il resto agli
upscaler come DLSS.

Vincoli:
- gameplay invariato;
- al 100% la catena di disegno è esattamente quella di oggi: stesse foto, FPS, RAM e VRAM sulla RTX;
- interfaccia, mappa, foto del pilota e modalità senza finestra restano a piena risoluzione.

## Decisione
- **Opzione «Risoluzione della scena»** nel menu Esc → Grafica: 100%, 75% o 50% (`gfx_render_scale` in
  `settings.txt`).
  - Alta, Media e Bassa restano al 100%. Un file `settings.txt` senza la chiave nuova resta al 100%.
  - **Integrata passa al 75%**: sull'Arc dà il 25% di FPS in più a 1080p (misure sotto).
  - Per le prove: `PLANET_ONE_RENDER_SCALE=100|75|50`, da usare insieme a `PLANET_ONE_GFX`. Nella shell
    (`\`) c'è `scala <100|75|50>`, che vale solo per la partita in corso.
- **Al 100% non cambia nulla**: la camera 3D disegna sulla finestra con egui sopra, come prima. Il sistema
  nuovo (`render_scale::apply_render_scale`, in `First`) non fa niente finché la scala resta al 100%.
- **Sotto il 100%** (`src/render_scale.rs`):
  - la camera 3D disegna su un'immagine `Rgba8UnormSrgb` della misura ridotta;
  - una camera 2D (ordine 1, sulla finestra, senza MSAA, tonemapping e dithering) mostra l'immagine a tutto
    schermo con un filtro lineare;
  - il contesto egui passa sulla camera 2D. L'interfaccia resta a piena risoluzione.
  - Lo spostamento di egui toglie dalla camera 3D tutti i componenti del contesto e li mette sulla camera
    2D. È lo stesso `egui::Context`, quindi lo stato resta: finestre aperte, scala dell'interfaccia,
    tema.
  - Le texture che egui ha già caricato (caratteri, icone) cambiano chiave in `EguiManagedTextures`. egui
    manda i caratteri una volta sola: senza questo passaggio il testo sparirebbe.
  - Nel mondo di disegno, la camera che perde egui conserva un collegamento a una vista che non esiste più.
    bevy_egui lo toglie solo alle camere spente, e senza toglierlo va in panic. Lo toglie
    `drop_stale_egui_views`.
  - Tornando al 100% si fa il percorso inverso: la camera 2D sparisce e la camera 3D torna sulla finestra.
- **Coordinate dello schermo**: il fattore di scala dell'immagine è quello della finestra moltiplicato per
  la scala di rendering, quindi la sua misura logica è quella della finestra. `world_to_viewport` e
  `viewport_to_world` danno le stesse coordinate di prima, e targhette, fumetti, segnalini della mappa e
  mirino del debug restano al loro posto senza toccarli.
- **Ombre**: con la camera su un'immagine, Bevy non sa da dove scegliere il dettaglio delle ombre. La camera
  3D riceve `ShadowLodOrigin`, come già senza finestra.
- **Mappa (M)**: l'immagine prende la misura piena della finestra e la mappa è disegnata a piena
  risoluzione. Alla chiusura torna alla scala scelta.
- **Finestra**: se cambia misura (F11, ridimensionamento) si crea un'immagine nuova. Se è ridotta a
  icona, la catena resta com'è.
- **Senza finestra** la scala non si usa: la camera disegna già su un'immagine alla misura esatta, per le
  foto del pilota. Con la finestra il pilota fotografa la finestra, cioè quello che vede il giocatore, alla
  misura piena.
- **Query sulla camera**: con due camere, le query che cercavano «l'unica camera» falliscono. Ora sono
  filtrate su `Camera3d`:
  - `enter_map` ed `exit_map` in `map.rs`: senza filtro la mappa non si apriva bene;
  - le camere di `menu.rs` e `hud.rs`: targhette e barre dei nemici.

### Misure
**Arc integrata (Intel Graphics ARL, Mesa 25.0.7), preset Integrata, finestra vera** (`--con-finestra`,
`VK_ICD_FILENAMES=…/intel_icd.x86_64.json`, ogni corsa con la sua cartella dati). FPS medio / 1% peggiore:

| Copione | 100%, 1080p | **75%, 1080p** (1440x810) | 50%, 1080p (960x540) | 100%, 720p | **75%, 720p** (960x540) | 50%, 720p (640x360) |
|---|---|---|---|---|---|---|
| `fps` | 24 / 17 | 30 / 22 | 40 / 27 | 36 / 24 | 41 / 32 | 48 / 38 |
| `giro` | 25 / 20 | 32 / 26 | 41 / 33 | 36 / 30 | 41 / 25 | 49 / 40 |
| `meteo` | 26 / 20 | 32 / 24 | 40 / 26 | 36 / 25 | 41 / 28 | 48 / 34 |
| Media | 25 | **31 (+25%)** | 40 (+61%) | 36 | **41 (+14%)** | 48 (+34%) |
| RAM massima | 930–1081 MB | 976–1093 MB | 977–1089 MB | 980–1047 MB | 973–1066 MB | 957–1042 MB |

- Le corse al 75% sono quelle senza variabili: rilevamento `Intel(R) Graphics (ARL) (IntegratedGpu) →
  preset Integrata`, scena a 1440x810 o a 960x540.
- In tutte le 18 corse: nessun fotogramma lento (> 100 ms), nessun fallimento, nessun avviso del gioco.
  L'unico errore nei log è un `audio stream error: Buffer underrun` di rodio.
- Al 100% i numeri sono quelli dell'ADR 0057 (25 e 36): la catena diretta non è peggiorata.
- La RAM non cambia: l'immagine intermedia pesa pochi MB.
- A 1080p il 75% vale più che a 720p, perché a 720p pesa di più il costo fisso del fotogramma (CPU e copia
  sulla RTX).
- Il 50% rende la scena visibilmente sfocata, soprattutto a 720p (640x360). Il 75% è il compromesso per il
  preset; il 50% resta una scelta del giocatore.

**RTX 3080 Ti, tutti i copioni** (`scripts/beta_check.sh`, senza finestra, dove la scala non si usa):

| Misura | ADR 0057 | Dopo |
|---|---|---|
| Copioni passati | 53 | 53 |
| FPS medio, media dei copioni | 55,2 | 55,4 |
| 1% peggiore, media dei copioni | 31,3 | 34,8 |
| Foto fuori soglia (0,12) | le cinque note | le stesse cinque |
| Differenza mediana delle foto dai riferimenti | — | 0,027 (170 foto) |
| RAM massima del processo, media dei copioni | — | 1941 MB |
| VRAM massima del processo, media dei copioni | — | 2252 MB (al massimo 4283) |

- Senza finestra `render_scale::plugin` non registra nessun sistema: la catena è letteralmente quella di
  prima. FPS e foto lo confermano. Le foto fuori soglia sono le cinque note dall'ADR 0056:
  - tre della piazza in `giro-camera`;
  - la spiaggia del risveglio;
  - la fonte sotto la quercia.
- La memoria qui è misurata **per processo**: RSS di `planet-one` e la sua riga in `nvidia-smi`. Durante la
  corsa sulla RTX c'era un modello di Ollama caricato da un'altra parte, con 4,5 GB di VRAM. Questi numeri
  non si confrontano con i totali dell'ADR 0057, che contavano tutta la scheda. Nessun codice nuovo gira
  senza finestra, quindi non hanno motivo di cambiare.
- Avvisi nei log: 19 in tutto, gli stessi tipi di sempre: giocatore riportato sopra il terreno, fotogrammi
  lenti durante i viaggi fra regioni, una rinomina della cartella dei salvataggi. Nessun ERROR, nessun panic.

**RTX 3080 Ti con la finestra, prove di funzionamento** (non misure):
- al 50% con la finestra, i pannelli (menu, diario, inventario, personaggio, aspetta) sono nitidi sopra
  la scena ingrandita;
- la mappa al 50% è uguale a quella al 100%: RMSE 0,003 sull'intera foto;
- la targhetta «Viandante» e il suggerimento «[E] Parla» stanno nello stesso punto al 50% e al 100%;
- cambio di scala in partita (100 → 50 → mappa → 75 → 100 → 50): nessun panic né avviso, inventario con
  caratteri e icone intatti dopo il giro completo di egui.

## Conseguenze
- Sulle schede dedicate e con le impostazioni salvate non cambia nulla.
- Chi parte su una scheda integrata senza aver mai scelto la grafica parte dal 75%.
- Chi ha già salvato il preset Integrata resta al 100%, perché la scelta del giocatore vince sempre. Può
  scegliere il 75% dal menu.
- Il passaggio di egui dipende dai componenti interni di bevy_egui 0.43: `EguiParts` in
  `render_scale.rs`, chiavi di `EguiManagedTextures` e `EguiCameraView`. A ogni aggiornamento di bevy_egui
  va riprovato il cambio di scala in partita.
- Una query nuova sulla camera di gioco va filtrata con `With<Camera3d>`. Una query su tutte le camere
  vede anche la camera finale.
- Scartati:
  - **egui sempre su una camera sua**, anche al 100%: avrebbe cambiato la catena di disegno delle schede
    dedicate (una camera e un passaggio in più);
  - **un upscaler** (FSR, DLSS): non c'è in Bevy per Vulkan su Intel. Il filtro lineare è semplice e basta
    per il 75%;
  - **una scala automatica** in base agli FPS: cambierebbe la nitidezza da sola durante il gioco.
- Restano da misurare, come per l'ADR 0057: una Radeon integrata, l'Arc con lo schermo collegato
  all'integrata e Windows con il driver Intel ufficiale.
