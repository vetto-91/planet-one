# Changelog — Planet One

Formato versione: `vMAJOR.MINOR.FEATURE.BUGFIX` (vedi [docs/VERSIONING.md](docs/VERSIONING.md)).

## v0.0.2.1 — Fase 2.1: character controller kinematico + fix collisioni terreno (2026-11-04)

- Player migrato da `RigidBody::Dynamic` a character controller cinematico
  Avian (`RigidBody::Kinematic` + `CustomPositionIntegration` + `MoveAndSlide`
  in `FixedUpdate`): elimina sprofondamenti e cadute nel vuoto sulle pendenze
  dell'heightfield a celle grandi (100 m / 256 = ~0.39 m).
- campaign Gravità manuale (20 m/s²) in `movement`; salto solo se `Grounded`.
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
