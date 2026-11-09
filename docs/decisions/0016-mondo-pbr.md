# ADR-0016: Mondo e materiali PBR

- Data: 2026-11-09
- Stato: accettato (Fase 2 della grafica realistica)
- Versione: v0.0.26.0

## Contesto
Dopo i personaggi veri (ADR 0015) il mondo era ancora cel-shaded: terreno con texture illustrate, edifici a colori di vertice senza UV, alberi a coni, nessun tonemapping.

## Decisione
- Rendering: `Hdr`, `Tonemapping::TonyMcMapface`, `Bloom::NATURAL`, SSAO (`ScreenSpaceAmbientOcclusion`, con `Msaa::Off` e `Smaa`), ombre a 4 cascate da 180 m con mappe da 4096, luce d'ambiente generata dal cielo (`GeneratedEnvironmentMapLight`, intensità da `update_sky`).
  Il sole sale a 9000 lux; di notte cielo e ambiente quasi si spengono, restano luna e lampioni. Bevy crea `EnvironmentMapLight` una sola volta: l'intensità si aggiorna lì, non su `GeneratedEnvironmentMapLight`.
- Texture CC0 Poly Haven 1k in `assets/textures/pbr/` (colore, normal GL, ARM), scaricate una sola volta da `scripts/fetch_pbr_textures.py`. Il gioco non accede alla rete. Feature `jpeg` di Bevy attiva.
- `buildings::B`: ogni faccia ha UV in metri e un materiale ricavato dal colore della tavolozza (`classify`): i colori esistenti restano la fonte, e diventano una tinta. `finish_parts` restituisce una mesh per materiale (`pbr::Mat`), usata da edifici e mura.
- Terreno: materiale PBR con tangenti analitiche e variazione di colore a larga scala che nasconde la ripetizione; `seasons` aggiunge una correzione verso il verde al prato.
- Abeti (`trees.rs`): i modelli scansionati hanno milioni di triangoli, quindi tronco + carte con i rametti (circa 300 triangoli, una mesh condivisa, 3 varianti) e copia bianca sui rami per la neve.

## Conseguenze
- SSAO e ombre costano: 50–75 FPS in scena (RTX 3080 Ti); sui PC deboli potrebbe servire un'opzione per spegnerle.
- Tela (`hessian_230`) e ferro (`rust_coarse_01`) completano la tavolozza: i colori non riconosciuti diventano legno se marroni, intonaco tinto altrimenti. La casetta del Viandante mantiene la sua foto.
- Ciò che pesa è spegnibile dal sotto-menu "Impostazioni grafiche" (`graphics.rs`, opzioni `gfx_*` in `settings.txt`).
