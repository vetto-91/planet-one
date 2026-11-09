# Licenze dei modelli

I file in `characters/` sono composti da `scripts/build_characters.py` a partire da questi pacchetti di **Quaternius** (https://quaternius.com), livello gratuito, licenza **CC0 1.0** (dominio pubblico):

- Universal Base Characters — https://quaternius.itch.io/universal-base-characters
- Modular Character Outfits - Fantasy — https://quaternius.itch.io/modular-character-outfits-fantasy
- Universal Animation Library — https://quaternius.itch.io/universal-animation-library

Per rigenerarli: scaricare gli zip in `assets_src/zips/` (`UAL_Standard.zip`, `BaseCharacters_Standard.zip`, `Outfits_Standard.zip`), poi:

```bash
python3 -I scripts/extract_character_packs.py assets_src
python3 -I scripts/build_characters.py assets_src assets/models/characters
```
