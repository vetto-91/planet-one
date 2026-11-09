# ADR-0029: Luoghi da scoprire

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.41.0

## Contesto
Fuori dalle mura l'isola aveva boschi e sentieri, ma non una meta.

## Decisione
- Ogni luogo è un piazzamento come gli edifici: la forma sta in `places::build`, la posizione in `debug/placements.txt`. Si sposta con F4, i bauli funzionano come gli altri contenitori e il terreno si appiana con gli spiazzi esistenti.
- Non si scava il terreno, che è una griglia di quote: la grotta è un cumulo di roccia con corridoio e camera, appoggiato alla scogliera.
- I banditi non hanno un elenco salvato a parte: un sistema controlla ogni due secondi che per ogni percorso di campo ci sia un nemico o una ricomparsa. Così funzionano nuova partita, reset e salvataggi fatti prima che esistessero.
- I nomi sulla mappa stanno in `isola.txt` (`luogo: nome | x z`).
- Mugnaio e contadina sono abitanti normali, con casa, letto e chiave.

## Conseguenze
- Rilanciare `scripts/gen_luoghi.py` riporta i luoghi alle posizioni iniziali, perdendo gli spostamenti fatti con F4.
- La quota di uno spiazzo viene dal centro dell'impronta: su un pendio ripido l'ingresso può restare in alto, come è successo alla prima posizione della grotta.
