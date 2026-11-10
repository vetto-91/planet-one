# ADR-0030: Rampe sotto le scale

- Data: 2026-11-10
- Stato: accettato
- Versione: v0.0.43.0

## Contesto
Il giocatore si incastrava nelle scale. Il controller (`MoveAndSlide` di Avian) non sale i gradini da solo, e gli spigoli fra scatole affiancate bloccano la capsula anche con alzate di 0,28 m.

## Decisione
- I gradini restano visibili ma non hanno più collider (`step_visual`). Sotto ci sta una rampa liscia (`Interior.ramps`: centro, rotazione, dimensioni) che passa all'altezza della pedata e arriva a filo del piano di sopra.
- Le chiocciole hanno una rampa per ogni cella, larga 0,5 m al massimo; i primi gradini dei pianerottoli restano scatole piene.
- Nei solai sopra le chiocciole i fori coprono le celle dove la testa (capsula da 1,7 m più solaio) arriverebbe nel solaio, con un margine di 0,25 m; le strisce sottili fra foro e muro vengono assorbite nel foro.
- La verifica è un test (`src/walk_test.rs`) che crea la fisica vera in un'app Bevy minima e fa camminare una capsula con `MoveAndSlide` lungo i percorsi degli abitanti e le chiocciole.

## Conseguenze
- Camminando su una rampa i piedi possono stare fino a mezzo gradino sopra o sotto la pedata disegnata.
- Ogni nuova scala deve avere la sua rampa; il test lo segnala se manca.
