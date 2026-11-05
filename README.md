# Planet One

RPG 3D open source, cel-shaded, in terza persona. Scritto in **Rust** con il motore **[Bevy](https://bevy.org)**. Obiettivo iniziale: una **vertical slice** giocabile (1 quest, 1 NPC, 1 nemico, inventario minimale).

> Stato: **Fase 5 — Vertical slice completata** (`v0.0.5.0`).
> Quest "Denaro Rubato", bussola, obiettivi (TAB), combattimento e inventario.
> Roadmap e fasi: [docs/ROADMAP.md](docs/ROADMAP.md).

## Requisiti

- Rust stable (>= 1.97) — [`rustup`](https://rustup.rs)
- Linux: `libudev`, `alsa`, `vulkan` driver (di solito già presenti su desktop)

## Compilazione ed esecuzione

```bash
cargo run            # esegue in dev
cargo run --release  # build ottimizzata
cargo fmt            # formatta
cargo clippy         # linter
```

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
| TAB (tenuto) | Obiettivi della quest (solo a quest attiva) |
| E | Parla con l'NPC vicino; nel dialogo continua / conferma |
| Invio | Nel dialogo: continua / conferma (alternativa a E) |
| Su/Giù o W/S | Nel dialogo: cambia la scelta evidenziata |
| 1–9, 0 | Usa l'oggetto nello slot corrispondente della barra in basso (pozione: +25 HP) |
| Click sinistro | Attacco melee |

## Quest: "Denaro Rubato"

Parla con il **Viandante** (NPC blu) allo spawn: alla fine del dialogo scegli
"Sì, ti aiuto" per far partire la quest ("Ora non ho tempo" la rimanda).
Trova il **ladro** (cubo rosso, lontano, oltre le colline; il segnalino rosso sulla bussola
in alto indica la sua direzione), uccidilo, raccogli le **12 monete d'oro** che lascia,
torna dal Viandante → completata, con ricompensa di 5 monete.
Tieni **TAB** per gli obiettivi: spunta verde = completato, sfera azzurra = in corso.

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
  menu.rs   UI egui (menu Esc, hint, contatore FPS)
docs/     decisioni (ADR) e convenzioni
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
