# ADR-0054: La traversata e l'arrivo all'Approdo delle Brume

- Stato: accettato
- Data: 2026-11-15

## Contesto
Quarto passo del piano per completare isola, Prologo e traversata (dopo ADR 0051–0053). La partenza del
Prologo era una scena raccontata con un solo punto d'arrivo, la via sotto il mare oltre la Porta Sommersa
si vedeva ma non si prendeva, il ritorno si faceva solo in barca e i compagni restavano sull'isola.

Trama: i documenti 09 e 10 (e la scelta dell'utente) vogliono le due vie all'**Approdo delle Brume**:
la barca attracca al molo; chi passa sotto il mare sbuca da un'antica **fonte di pietra a gradoni tra le
radici di una quercia gigante**, a una trentina di metri dal porticciolo (doc 10, §1.2). Tiro-Scheria e il
Cenote dell'Alba del documento 06 restano per gli atti successivi.

## Decisione
- **Due punti d'arrivo** (`src/crossing.rs`): la scena del viaggio ricorda la via (`Cutscene::deep`), e
  l'arrivo la legge una volta (`crossing::arrival`): via del mare → il molo (`Region::arrival`); via sotto il
  mare → la fonte sul continente, o la nicchia davanti alla Porta Sommersa sull'isola.
- **La fonte a gradoni** sul continente (26; 444, 28 m dal porto): quattro gradoni quadrati di pietra che
  scendono in un'acqua ferma, e una quercia gigante dietro, con le radici che abbracciano i gradoni e una
  chioma larga (`crossing::build_spring`, costruita quando si è sul continente).
- **La via sotto il mare si prende nei due sensi**:
  - sull'isola, i gradini oltre la Porta Sommersa ora si scendono davvero (pavimento vero, niente più
    gradino che ferma il passo); in fondo, nell'acqua, `E` comincia la traversata verso il continente;
  - sul continente, nell'acqua della fonte, `E` riporta sull'isola, con le sue righe raccontate
    (`crossing::return_lines`), alla nicchia davanti alla porta;
  - si può prendere solo a Prologo compiuto (`continente`) e con la Porta Sommersa aperta. Prima l'acqua
    è solo da guardare.
- **I compagni partono con te**: chi ti segue viaggia con te. Sul continente, se l'abitante non c'è, compare
  al tuo fianco con la sua scheda dell'isola (`villagers::traveler_def`, `villagers::spawn_villager`):
  stesso aspetto, stessa voce, stessa vita. Congedato, ti aspetta all'approdo (`TRAVELER_POST`); tornando
  sull'isola ritrovi l'abitante di sempre.
- **Prova**: `prove/zzzzzzzzzzzzzzzzzzzzzzz-traversata.txt` (Silvia con te, i gradini, la via sotto il mare
  verso la fonte, Silvia sul continente, il ritorno alla nicchia).

## Conseguenze
- La traversata resta raccontata: il viaggio per mare e la via sotto il mare non si percorrono metro per
  metro (scelta presa con l'utente all'ADR 0048).
- Il compagno congedato sul continente non si salva come abitante: dopo un caricamento o un viaggio torna a
  casa sua, sull'isola.
- Sul continente il compagno non ha casa né turni: aspetta all'approdo a ogni ora.
