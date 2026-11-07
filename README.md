# Planet One

RPG 3D open source, cel-shaded, in terza persona. Scritto in **Rust** con il motore **[Bevy](https://bevy.org)**. Obiettivo iniziale: una **vertical slice** giocabile (1 quest, 1 NPC, 1 nemico, inventario minimale).

> Stato: **Fase 5 — Vertical slice completata, con rifiniture** (`v0.0.10.0`).
> Quest "Denaro Rubato", bussola, obiettivi (TAB), combattimento e inventario.
> Roadmap e fasi: [docs/ROADMAP.md](docs/ROADMAP.md).

## Requisiti

- Rust stable (>= 1.97) — [`rustup`](https://rustup.rs)
- Linux: `libudev`, `alsa`, `vulkan` driver (di solito già presenti su desktop)

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
| F3 | Wireframe dei collider (debug fisico) |
| F11 | Schermo intero on/off (in fullscreen il cursore è bloccato e nascosto) |
| M | Mappa dall'alto (vista drone, nord in alto, bussola in alto a sinistra); rotella o +/- = zoom; frecce/WASD o tasto centrale = sposta, C = su di te; M/Esc = chiudi |
| F4 | Modalità debug piazzamenti (vedi sotto): strumenti Piazza / Gomma / Sposta, clic sinistro (mappa e 3D) |
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
Le voci (`assets/audio/voci/`) sono generate con `scripts/gen_tts_voci.py` (edge-tts, voci neurali
Microsoft, richiede rete): verificarne i termini d'uso prima di distribuire il gioco. Autore e licenza del brano da indicare prima di rendere
pubblico il repository.

### Giorno e notte

Il giorno di gioco dura 24 minuti reali (1 ora = 1 minuto); sole e luna attraversano il cielo
e luce e cielo cambiano con l'ora. L'ora è nel chip in alto a sinistra; T apre "Aspetta".
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
  figure.rs  personaggi a parallelepipedo con skin a sviluppo (player/ladro/viandante)
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
