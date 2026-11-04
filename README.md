# Planet One

RPG 3D open source, cel-shaded, in terza persona. Scritto in **Rust** con il motore **[Bevy](https://bevy.org)**. Obiettivo iniziale: una **vertical slice** giocabile (1 quest, 1 NPC, 1 nemico, inventario minimale).

> Stato: **Fase 1.4 — Fix menu Esc + FPS overlay** (`v0.0.1.4`).
> Menu Esc (tasti, slider sensibilità, checkbox FPS, reset scena robusto),
> WASD + corsa (Shift) + salto (Space), camera orbit, contatore FPS.

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

## Controlli

| Tasto | Azione |
|---|---|
| W/A/S/D o frecce | Movimento |
| Shift | Corsa (1.7×) |
| Space | Salto |
| Mouse | Orbita camera (sensibilità dal menu) |
| Esc | Menu pausa (tasti, slider sensibilità, checkbox FPS, reset scena) |

## Struttura

```
assets/   modelli glTF, texture, audio, scene
src/      codice del gioco (ECS Bevy)
  main.rs   stato app e cablaggio sistemi
  scene.rs  costruzione e reset della scena
  player.rs movimento, salto, corsa, facing, ground-check
  camera.rs orbita e follow della camera
  menu.rs   UI egui (menu Esc, hint, contatore FPS)
docs/     decisioni (ADR) e convenzioni
.github/  template issue e PR
```

## Roadmap

- [x] Fase 0 — Bootstrap
- [x] Fase 1 — Movimento (WASD + camera terza persona)
- [ ] Fase 2 — Scena (terreno, cel-shading, luci)
- [ ] Fase 3 — Entità (NPC, dialoghi, nemico)
- [ ] Fase 4 — UI / Inventario
- [ ] Fase 5 — Vertical slice RPG

## Contribuire

Progetto sviluppato in modo AI-assisted con revisione umana. Apri una issue usando i template in `.github/ISSUE_TEMPLATE/`.

## Licenza

Doppia licenza: **MIT OR Apache-2.0** — vedi `LICENSE-MIT` e `LICENSE-APACHE`.
