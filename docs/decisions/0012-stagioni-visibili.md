# ADR-0012: Stagioni visibili (Fase 2 del calendario)

- Data: 2026-11-08
- Stato: accettato
- Versione: da rilasciare (v0.0.22.0)

## Contesto
Dopo la data e il sole stagionale (ADR 0011) il mondo deve anche cambiare aspetto con le stagioni.

## Decisione
- `src/seasons.rs`: `look(giorno_dell'anno)` interpola 12 fotogrammi chiave ciclici (nessuno scatto) e produce tinta dell'erba, chiome autunnali e innevate,
  fiori, neve, foglie in caduta, tinta del sole e luce ambiente. `SeasonLook` si aggiorna ogni quarto di giorno di gioco; `apply_season` modifica solo
  i materiali condivisi (marker `SeasonGround`, `SeasonLeaf`, `SeasonFlower`, `SeasonSnowRoof`, `SeasonSnowGround`) quando il look cambia o compaiono oggetti nuovi.
- Neve a terra: overlay con gli stessi vertici del terreno (come sabbia e strade), alfa a chiazze, ridotta su sabbia e strade, assente in mare; la quantità è
  l'alfa del materiale. Neve sui tetti: `buildings::B` raccoglie un secondo mesh (falde sollevate di 6 cm), mostrato solo con neve > 0,3.
- Alberi: le tre chiome condivise passano da verde ad arancio/rosso/giallo e poi a bianco; i fiori hanno una soglia casuale e crescono dalla primavera, spariscono in autunno.
- Foglie che cadono: 48 cubetti attorno alla camera, attivi in proporzione a `leaf_fall` (autunno).
- Sole e ambiente seguono la stagione (inverno freddo e più scuro, estate calda).

## Conseguenze
- Con il terreno completo l'overlay della neve raddoppia i triangoli visibili: misurati ≈ 55–65 FPS, nessun calo evidente.
- Fiori e chiome sono ricreati a ogni modifica dei piazzamenti (F4): il look viene riapplicato da `Added<...>`.
- Meteo (pioggia, nebbia, neve che cade, temperatura) è la Fase 3.
