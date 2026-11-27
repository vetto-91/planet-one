# Licenze dei modelli

Autori, licenze e link di tutti gli asset (modelli, texture, audio, voci, font) stanno in
[`CREDITS.md`](../../CREDITS.md), alla radice del progetto. I modelli di questa cartella sono di
Quaternius, licenza CC0 1.0.

Per rigenerare i personaggi: scaricare gli zip in `assets_src/zips/` (`UAL_Standard.zip`,
`BaseCharacters_Standard.zip`, `Outfits_Standard.zip`), poi:

```bash
python3 -I scripts/extract_character_packs.py assets_src
python3 -I scripts/build_characters.py assets_src assets/models/characters
```

Animali e alberi a foglia larga: `blender -b --disable-autoexec -P scripts/build_animals_trees.py -- assets_src assets`,
partendo dai pacchetti in `assets_src/animali_zip/` e `assets_src/alberi/`.
