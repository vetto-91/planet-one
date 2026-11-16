# ADR-0057: Schede video integrate

- Stato: accettato
- Data: 2026-11-16

## Contesto
Planet One deve girare anche sulle schede video integrate nelle CPU Intel (Arc, Iris Xe) e AMD (Radeon).
Queste schede non hanno memoria propria: usano la RAM di sistema, con poca banda e pochi shader. Il preset
Alta, quello di partenza, prevede:
- ombre del sole a 4 cascate da 4096 px;
- SSAO, bloom e SMAA;
- ombre delle luci piccole: stufa, lume, luci interne e lanterne delle guardie.

Su queste schede è troppo pesante.

Vincoli:
- gameplay e meccaniche non cambiano;
- con le impostazioni di oggi, una scheda dedicata vede la stessa grafica;
- FPS, RAM e VRAM sulla RTX 3080 Ti non peggiorano rispetto all'ADR 0056.

Sulla macchina di prova (Core Ultra 9 285K) l'Arc integrata (8086:7d67) non è riconosciuta da Mesa 23:
serve Mesa 24.1 o più recente. Il caso peggiore si è quindi simulato con **llvmpipe**, il rendering
Vulkan sulla CPU (`WGPU_ADAPTER_NAME=llvmpipe`, tipo di scheda `Cpu`), senza toccare i driver. AMD non è
provabile qui.

## Decisione
- **Riconoscimento della scheda** (`graphics::pick_preset_for_adapter`, in `PreStartup`):
  - legge `RenderAdapterInfo`;
  - se la scheda è `IntegratedGpu` o `Cpu` (`wants_light_preset`) e la grafica non è mai stata scelta,
    usa il preset **Integrata**;
  - «mai stata scelta» vuol dire nessuna riga `gfx_*` in `settings.txt` e nessun `PLANET_ONE_GFX`;
  - la scelta vale solo in memoria: il file non viene scritto. Se poi il giocatore salva le impostazioni,
    il preset resta salvato come qualunque altra scelta fatta dal menu;
  - nel log: `[GRAFICA] scheda: <nome> (<tipo>) → preset …`;
  - una scheda dedicata resta su Alta, come prima.
- **Preset «Integrata»** (`GfxOptions::INTEGRATED`), anche nel menu Esc → Grafica e con
  `PLANET_ONE_GFX=integrata`:
  - ombre del sole **Minime**, un livello nuovo: 1 cascata da 1024 px fino a 40 m;
  - niente ombre di lampioni e luci piccole, niente luci interne;
  - niente SSAO, bloom e SMAA;
  - particelle al 50%, vegetazione al 40%, niente onde;
  - animazioni lontane ferme;
  - erba, fiori e cespugli fino a 30 m invece di 42;
  - texture dei materiali a metà risoluzione.
- **Opzioni nuove nel menu**, salvate in `settings.txt`. Il loro default è il comportamento di oggi, così
  Alta, Media e Bassa restano identiche:
  - «Ombre di stufa, lumi e lanterne» (`gfx_small_shadows`, default sì): stufa e lume della casetta,
    luci interne delle case, lanterne delle guardie;
  - «Erba, fiori e cespugli» (`gfx_grass_distance`: Normali, cioè 42 m, oppure Vicini, cioè 30 m);
  - «Texture ridotte» (`gfx_reduced_textures`, vale al prossimo avvio): `pbr::load_tex` prende le texture
    da `assets/textures/pbr/half/`. Le copie a metà risoluzione si generano con
    `scripts/make_half_textures.py`: 39 file, 3,4 MB contro 28 MB.
- Un file `settings.txt` vecchio, senza le chiavi nuove, mantiene la grafica di oggi. C'è un test.

### Misure
**RTX 3080 Ti, tutti i 53 copioni, preset Alta scelto dal rilevamento** (confronto con la corsa dell'ADR 0056):
| Misura | ADR 0056 | Dopo |
|---|---|---|
| Copioni passati | 53 | 53 |
| FPS medio, media dei copioni | 54,7 | 55,2 |
| 1% peggiore, media dei copioni | 32,0 | 31,3 |
| RAM massima, media dei copioni | 2798 MB | 2835 MB |
| VRAM totale massima, media dei copioni | 4858 MB | 4865 MB |
| Foto rispetto alla corsa dell'ADR 0056 (mediana) | — | 0,010 |

- Le differenze stanno nel rumore fra una corsa e l'altra: i singoli copioni oscillano in tutte e due le
  direzioni, da −15 a +10 FPS.
- Le foto fuori soglia sono le stesse cinque note dall'ADR 0056: tre della piazza in `giro-camera`, la
  spiaggia del risveglio e la fonte sotto la quercia.

**llvmpipe, copione `fps` a 1280x720** (rendering sulla CPU, 24 core):
| Misura | Alta (prima) | Integrata, scelta da sola |
|---|---|---|
| Rilevamento | — | `llvmpipe (Cpu) → preset Integrata` |
| Il pilota parte (120 fotogrammi di gioco) | no, entro 40 minuti | no, entro 20 minuti |
| RAM massima | 3001 MB | 2289 MB (−24%) |

- llvmpipe è troppo lento per misurare gli FPS: con nessuno dei due preset arriva ai 120 fotogrammi che il
  pilota aspetta prima del primo comando.
- Il rilevamento e la RAM sono provati, la fluidità no: va misurata su una scheda integrata vera.

## Conseguenze
- Su una scheda dedicata non cambia nulla: stesso preset, stesse foto, stesse prestazioni.
- Un giocatore con una scheda integrata al primo avvio parte dal preset leggero, e dal menu Esc può
  sempre alzarlo.
- Dopo un cambio delle texture PBR va rilanciato `scripts/make_half_textures.py`.
- Restano da misurare, sulla macchina vera:
  - l'Arc integrata con Mesa 24.1 o più recente;
  - una Radeon integrata AMD;
  - con le schede vere vanno rivisti i valori del preset (cascata, distanza dell'erba).
- Scartati:
  - **scala di rendering** (disegnare a 75% o 50% e ingrandire): bevy_egui 0.43 aggancia l'interfaccia
    alla prima camera, quindi spostare la scena su un'immagine intermedia avrebbe cambiato camera a egui.
    Avrebbe toccato anche le foto senza finestra del pilota e la camera della mappa. È la leva più forte
    per le integrate, ma va fatta insieme a un cambio di egui; `MainPassResolutionOverride` di Bevy serve
    solo agli upscaler come DLSS;
  - **mipmap e KTX2**: avrebbero cambiato la resa anche sulle schede dedicate;
  - **cambiare il preset di chi ha già scelto la grafica**: la scelta del giocatore vince sempre.
