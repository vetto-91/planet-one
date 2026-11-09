# ADR-0026: Forma dell'isola e acque

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.37.0

## Contesto
Fuori dal paese il terreno era quasi piatto, senza acque interne né sentieri.

## Decisione
- `assets/world/isola.txt` descrive alture, scogliere, ruscello, laghetto e ponti, ed è modificabile a mano; i sentieri sono strade di tipo `sentiero` in `villaggio.txt`.
- Le alture sono gaussiane sommate al terreno e si annullano vicino alle mura: il paese non cambia quota.
- Le scogliere sono settori di costa in cui il terreno si alza e scende di colpo; un recinto invisibile sul ciglio impedisce di cadere e restare bloccati ai piedi della parete.
- Il ruscello parte dal terreno definitivo: la superficie scende sempre, il laghetto è piatto e sta sotto il punto più basso del suo bordo. L'alveo non scende sotto la soglia della riva, altrimenti diventerebbe mare.
- Mare e ruscello usano una mappa di normali generata nel codice, con mipmap, che scorre; schiuma lungo la riva. Tutto spegnibile da «Onde e schiuma».
- La roccia copre i pendii ripidi, in base alla pendenza.

## Conseguenze
- Il terreno si ricalcola al riavvio dopo una modifica a `isola.txt`.
- L'acqua dolce si attraversa a piedi; non c'è nuoto.
