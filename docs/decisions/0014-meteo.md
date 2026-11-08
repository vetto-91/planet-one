# ADR-0014: Meteo (Fase 3 del calendario)

- Data: 2026-11-08
- Stato: accettato
- Versione: da rilasciare (v0.0.24.0)

## Contesto
Dopo il calendario (ADR 0011) e le stagioni visibili (ADR 0012) mancava il tempo atmosferico.

## Decisione
- `src/weather.rs`: `day_weather(data)` sceglie il tipo (sereno, nuvoloso, pioggia, temporale, nebbia, neve) con un hash della data e i pesi del mese, più una finestra oraria in cui
  il fenomeno è attivo (anche a cavallo della mezzanotte). La pioggia con temperatura media sotto 1,5 °C diventa neve e viceversa; niente temporali in inverno né neve d'estate (test).
- Temperatura: media stagionale (≈0 °C a gennaio, ≈20 °C a luglio) ± 4 °C nel giorno + offset del meteo.
- `WeatherState` (nuvole, pioggia, neve, nebbia, vento) si avvicina ai valori obiettivo con costante di tempo di ~8 s (per la velocità del tempo). `PLANET_ONE_WEATHER` forza il meteo.
- Effetti: `DistanceFog` sulla camera (colore secondo l'ora, densità secondo il meteo); sole, ambiente e cielo attenuati dalle nuvole in `daynight::update_sky`; lampi (bagliore + tuono ritardato) col temporale;
  particelle di pioggia (900) e neve (600) in coordinate di mondo con il vento, riciclate a terra o fuori raggio (come le foglie); al coperto (raggio verso l'alto, escluso il giocatore) calano e il suono si abbassa.
- Neve a terra (`SnowCover`): cresce nevicando e si scioglie sopra zero; si somma alla neve stagionale (`seasons::apply_season`).
- Audio: pioggia, vento (anelli seamless) e tuono sintetizzati in memoria (rumore filtrato), come le campane; nessun file, nessuna rete.
- Mondo: abitanti (non guardie) si riparano in casa con precipitazioni forti (isteresi); lampioni accesi con cielo molto scuro; la stufa del Viandante resta accesa sotto 8 °C.

## Conseguenze
- Pioggia e neve sono particelle intorno alla camera: costano poco (≈1500 trasformazioni/frame), nessun calo visibile.
- Il colore della nebbia non coincide al 100% con quello dello skybox all'orizzonte.
- Possibili sviluppi: pozzanghere/terreno bagnato, tetti con acqua, effetti sugli animali, previsioni.
