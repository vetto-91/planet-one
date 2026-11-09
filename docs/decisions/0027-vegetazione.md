# ADR-0027: Vegetazione dell'isola

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.38.0

## Contesto
Sull'isola c'erano solo 60 abeti piazzati a mano, tutti a nord-est.

## Decisione
- Le zone di bosco stanno in `assets/world/isola.txt`. La distribuzione usa rumore e hash a seme fisso: è la stessa a ogni avvio ed evita paese, strade, sentieri, edifici, campi, acqua e pendii ripidi.
- Gli alberi non sono entità singole: per ogni riquadro di 30 m le copie si fondono in poche mesh, così il disegno è per riquadro.
- Tre livelli per distanza: alberi pieni con ombre entro 55 m, versione a carte larghe senza ombre oltre, erba, fiori e cespugli solo entro 42 m. Tronchi e rocce si vedono sempre.
- Le latifoglie perdono le foglie d'inverno; i fiori seguono la stagione.
- «Densità vegetazione» in Impostazioni grafiche ricostruisce i riquadri.

## Conseguenze
- La vegetazione dipende da F4 solo al cambio dei piazzamenti, che la ricostruisce.
- Le latifoglie generate con le carte degli abeti sono l'anello debole: da vicino sembrano più palme che querce. Un pacchetto CC0 può sostituirle cambiando solo i prototipi.
