# Changelog — Planet One

Formato versione: `vMAJOR.MINOR.FEATURE.BUGFIX` (vedi [docs/VERSIONING.md](docs/VERSIONING.md)).

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
