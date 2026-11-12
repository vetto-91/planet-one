# ADR-0032: Audio posizionale

- Stato: accettato
- Data: 2026-11-12

## Contesto
Tutti gli effetti uscivano identici dalle due casse: una campana o un abitante a sinistra non si sentivano
a sinistra (revisione dei suoni del beta test). Il gioco calcola già da sé quanto un suono cala con la
distanza (campane, uccelli, gabbiani, pesci), con curve scelte a orecchio.

## Decisione
- Si usa l'audio spaziale di Bevy (rodio): `SpatialListener` sulla camera, `PlaybackSettings::with_spatial`
  e un `Transform` sui suoni che hanno un punto d'origine.
- **Posizionali**: rintocchi (dal campanile), voci di dialoghi e fumetti (figlie di chi parla, all'altezza
  della bocca: seguono il personaggio), uccelli, gabbiani, pesci (in un punto a caso attorno), tuoni (lontano,
  da una direzione a caso).
- **Non posizionali**: musica, suoni continui d'ambiente e del meteo (mare, vento, pioggia), passi del
  giocatore, voce dell'IA nella chat (si parla faccia a faccia).
- Rodio fa calare il volume con 1/d²; la scala `audio::SPATIAL_SCALE` (1/250) rende le distanze entro 250 m
  minori di 1, così quel calo non si somma ai nostri. La direzione non dipende dalla scala.
- Un suono davanti esce al 75% per orecchio (di lato: 100% e 50%, cioè 6 dB di differenza): `SPATIAL_GAIN`
  (1,15) lo compensa senza far saturare una voce tutta da un lato.
- In rodio 0.22 la formula è invertita (il canale dell'orecchio più lontano è il più forte): le orecchie
  dell'ascoltatore sono scambiate in `audio::listener()`. Verificato registrando le campane dalla piazza.

## Conseguenze
- Il copione `ascolto` registra le campane guardando a nord e a sud: la riga «Canali» di `scripts/ascolta.py`
  deve dare la sinistra più forte nel primo caso e la destra nel secondo (circa 3,7 dB).
- Se si aggiorna Bevy/rodio va ricontrollato lo scambio delle orecchie con lo stesso copione.
- I suoni posizionali hanno `SpatialAudioSink` invece di `AudioSink` (per cambiarne il volume dopo l'avvio).
