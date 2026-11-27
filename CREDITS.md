# Crediti e fonti

Chi ha fornito cosa in Planet One: modelli 3D, texture, audio, voci, font, librerie e strumenti.
Il **codice** del gioco è sotto doppia licenza **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`).
Gli asset hanno la licenza indicata qui sotto.

> **Regola:** ogni asset nuovo (modello, texture, suono, voce, font) si aggiunge a questo file nello
> stesso commit che lo porta nel repo, con autore, licenza e link.

## Modelli 3D

Tutti di **Quaternius** (https://quaternius.com), livello gratuito, licenza
**[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)** (dominio pubblico). I pacchetti
originali stanno in `assets_src/` (non in git) e vengono rielaborati dagli script indicati.

| Pacchetto | Usato per | File nel gioco | Link | Script |
|---|---|---|---|---|
| Universal Base Characters (Standard) | corpi, capelli, occhi dei personaggi | `assets/models/characters/*.gltf` | https://quaternius.itch.io/universal-base-characters | `scripts/build_characters.py` |
| Modular Character Outfits – Fantasy (Standard) | abiti (ricolorati) | `assets/models/characters/textures/abito_*.png` | https://quaternius.itch.io/modular-character-outfits-fantasy | `scripts/build_characters.py` |
| Universal Animation Library (Standard) | animazioni | `assets/models/characters/animations.gltf` | https://quaternius.itch.io/universal-animation-library | `scripts/build_characters.py` |
| Farm Animals Animated | mucche e pecore | `assets/models/animals/` | https://quaternius.com | `scripts/build_animals_trees.py` |
| Textured Stylized Trees – May 2020 | alberi a foglia larga | `assets/models/trees/`, `assets/textures/alberi/` | https://www.patreon.com/quaternius | `scripts/build_animals_trees.py` |

Galline, gabbiani, uccelli, pesci e farfalle sono costruiti nel codice (`src/animals.rs`); edifici,
mura, moli, barche e mobili sono procedurali (`src/buildings.rs`, `src/interiors.rs`, `src/village.rs`).

## Texture

### Poly Haven (CC0 1.0)

Da **Poly Haven** (https://polyhaven.com), licenza **CC0 1.0**. Si riscaricano con
`scripts/fetch_pbr_textures.py` (solo da `dl.polyhaven.org`); le copie ridotte di `pbr/half/` le
produce `scripts/make_half_textures.py`.

| Nel gioco (`assets/textures/pbr/`) | Materiale Poly Haven |
|---|---|
| `grass_*` | [leafy_grass](https://polyhaven.com/a/leafy_grass) |
| `sand_*` | [coast_sand_01](https://polyhaven.com/a/coast_sand_01) |
| `cobble_*` | [cobblestone_floor_04](https://polyhaven.com/a/cobblestone_floor_04) |
| `plaster_*` | [white_plaster_02](https://polyhaven.com/a/white_plaster_02) |
| `planks_*` | [weathered_brown_planks](https://polyhaven.com/a/weathered_brown_planks) |
| `stone_*` | [castle_wall_slates](https://polyhaven.com/a/castle_wall_slates) |
| `tiles_*` | [ceramic_roof_01](https://polyhaven.com/a/ceramic_roof_01) |
| `slate_*` | [roof_slates_03](https://polyhaven.com/a/roof_slates_03) |
| `thatch_*` | [thatch_roof_angled](https://polyhaven.com/a/thatch_roof_angled) |
| `cloth_*` | [hessian_230](https://polyhaven.com/a/hessian_230) |
| `iron_*` | [rust_coarse_01](https://polyhaven.com/a/rust_coarse_01) |
| `fir_bark_*`, `fir_twig_*` | corteccia e rametti di [fir_tree_01](https://polyhaven.com/a/fir_tree_01) |

### Generate con Gemini dall'autore

Immagini create dall'autore del gioco con **Gemini** (Google); valgono i
[termini d'uso di Google per i contenuti generati](https://policies.google.com/terms/generative-ai).

| File | Cos'è |
|---|---|
| `assets/textures/sky.png` | panorama di nuvole del cielo (caricato da `src/scene.rs`, convertito in cubemap da `src/sky.rs`) |
| `assets/textures/capanna.png` | facciata della casetta del Viandante (sistemata da `scripts/make_capanna_texture.py`) |
| `assets/textures/porta.png`, `assets/textures/imposte.png` | porta e scuri della casetta |

### Fatte dal progetto

Generate dagli script del repo, con la stessa licenza del codice:
- `pietra.png`, `selciato.png`, `serranda.png`: `scripts/make_stone_texture.py`,
  `make_selciato_texture.py`, `make_serranda_texture.py`;
- `terrain_heightmap.png`: `scripts/gen_heightmap.py`;
- `icone/*.png`, le icone degli oggetti: `scripts/make_item_icons.py`;
- `cartelli/*.png`, i cartelli: `scripts/make_cartelli.py`. Scritti con il font **DejaVu Serif
  Bold** ([licenza DejaVu](https://dejavu-fonts.github.io/License.html), libera).

## Audio

| Cosa | File | Autore / fornitore | Licenza / termini |
|---|---|---|---|
| Musiche di sottofondo | `assets/audio/music/legacy_of_the_first_light.ogg`, `lanterns_in_the_grove.ogg` | create dall'autore con **Gemini** (Google) | [termini di Google per i contenuti generati](https://policies.google.com/terms/generative-ai) |
| Brusio del mercato «Vita tra i banchi» | `assets/audio/ambience/vita_tra_i_banchi.ogg` | generato con l'**IA di Google** (il file originale porta la filigrana SynthID) | come sopra |
| Effetti e ambienti: pioggia, vento, tuoni, mare, uccelli, gabbiani, passi, campane, incantesimi, serrature… | nessun file: sintetizzati all'avvio | il progetto (`src/weather.rs`, `src/ambience.rs`, `src/bells.rs`, `src/sfx.rs`) | licenza del codice |
| Voci dei personaggi | `assets/audio/voci/` (`viandante/`, `ladro/`, `prologo/`, `abitanti/`, `congedo/`) | generate **in locale** con [Piper](https://github.com/rhasspy/piper): voce maschile `it_IT-riccardo-x_low`, voce femminile e narratore `it_IT-paola-medium` | licenze dei modelli di voce, nelle loro [model card](https://huggingface.co/rhasspy/piper-voices/tree/main/it/it_IT) |

Le voci si rigenerano con `scripts/gen_tts_voci.py`, `gen_voci_prologo.py` e `gen_voci_abitanti.py`;
la pronuncia di alcune parole è corretta da `assets/audio/voci/pronuncia.txt`.

## Font dell'interfaccia

Il gioco usa i font predefiniti di [egui](https://github.com/emilk/egui) (crate `epaint_default_fonts`):

| Font | Autore | Licenza |
|---|---|---|
| Ubuntu Light | Canonical | [Ubuntu Font Licence 1.0](https://ubuntu.com/legal/font-licence) |
| Hack | Source Foundry | MIT + Bitstream Vera |
| Noto Emoji | Google | [SIL Open Font License 1.1](https://openfontlicense.org) |
| emoji-icon-font | Jake Jarvis (con egui) | MIT |

## Software

Librerie principali (`Cargo.toml`); l'elenco completo delle dipendenze si vede con `cargo tree`.

| Crate | Versione | Licenza | Progetto |
|---|---|---|---|
| bevy | 0.20.1 | MIT OR Apache-2.0 | https://bevy.org |
| avian3d | 0.8.0 | MIT OR Apache-2.0 | https://github.com/Jondolf/avian |
| bevy_egui | 0.43.1 | MIT | https://github.com/vladbat00/bevy_egui |
| ureq | 2.12 | MIT OR Apache-2.0 | https://github.com/algesten/ureq |
| serde, serde_json | 1.0 | MIT OR Apache-2.0 | https://serde.rs |
| cpal | 0.17 | Apache-2.0 | https://github.com/RustAudio/cpal |
| libc (solo Unix) | 0.2 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| windows-sys (solo Windows) | 0.61 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |

## Strumenti di IA (installati dal giocatore, non distribuiti col gioco)

Il gioco usa l'IA **solo in locale** (ADR 0005, 0006, 0022, 0063). Questi strumenti li scarica il giocatore
con `setup_ai.sh` (o `setup_ai.ps1`, inclusi nei pacchetti), accettandone le licenze, nella cartella dati del gioco:

| Strumento | Versione | Licenza | Fonte |
|---|---|---|---|
| Ollama (avviato e chiuso dal gioco, ADR 0063) | v0.40.18 | MIT | https://github.com/ollama/ollama/releases |
| Modello di testo `gemma4:e4b` (o `gemma4:e2b`) | — | [termini d'uso di Gemma](https://ai.google.dev/gemma/terms) | https://ollama.com/library/gemma4 |
| Piper (sintesi vocale) | 2023.11.14-2 | MIT | https://github.com/rhasspy/piper/releases |
| Voci italiane di Piper: riccardo x_low, paola medium, serena medium/high | — | vedi ogni model card | https://huggingface.co/rhasspy/piper-voices/tree/main/it/it_IT |
| whisper.cpp (riconoscimento vocale) | v1.9.5 | MIT | https://github.com/ggml-org/whisper.cpp |
| Modello Whisper `ggml-base.bin` | — | MIT (OpenAI Whisper) | https://huggingface.co/ggerganov/whisper.cpp |

## Storia e testi

- Trama, personaggi, missioni e dialoghi sono dell'autore. Le missioni seguono i suoi documenti di
  trama «Trama-Planet-One» (non nel repo), citati nelle intestazioni di
  `assets/quests/prologo/missioni.txt`, `secondarie.txt` ed `eventi.txt`.
- Alcune battute richiamano l'**Odissea** di Omero (dominio pubblico): Odisseo «dai molti modi»
  e la prova dell'arco attraverso i dodici anelli.
