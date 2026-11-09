# ADR-0018: Mura percorribili e torri attraversabili

- Data: 2026-11-09
- Stato: accettato (sostituisce i camminamenti a tratti di ADR 0013)
- Versione: da rilasciare (v0.0.30.0)

## Contesto
I camminamenti erano quattro tratti separati dalle torrette piene e le guardie di ronda stavano ferme.

## Decisione
- Coordinate lungo il perimetro (`Perimeter`): il lato interno del muro resta sulla linea del poligono, lo spessore cresce solo verso l'esterno (3,4 m), così il villaggio non perde spazio.
- Quote del piano (`walk_heights`): terreno lisciato + 3,6 m, fisse sopra i cancelli, pendenza massima 0,09 per metro; a tratti da 2,2 m i dislivelli restano sotto l'alzata delle scale.
- Torri cave generate da `TowerPlan`: base piena fino al piano, stanza con un varco di 3 m verso ogni tratto di muro. Il muro entra di poco sotto la torre per non lasciare buchi con le torri ruotate sulla bisettrice.
- Cancelli: una torre per lato e un ponte tra le due, a 3,6 m sopra il terreno del vano; i battenti non cambiano.
- Scale nella corsia interna, dentro lo spessore del muro: una dopo ogni cancello, più i tratti più lunghi a est e a ovest. Tutte portano all'intero anello.
- Le guardie `muro:` percorrono il percorso chiuso `VillageInfo.patrol`, distanziate di un quarto di giro.
- Niente alberi o campi nella fascia di muro e torri (`in_wall_band`).

## Conseguenze
- Le mura sono più alte (circa 4,7 m col parapetto), a est il piano arriva a circa 6 m sul livello del mare.
- Il cancello è più profondo: si passa in un tunnel lungo 4,4 m.
