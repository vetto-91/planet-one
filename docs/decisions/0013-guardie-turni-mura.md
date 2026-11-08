# ADR-0013: Guardie con turni, casa delle guardie e mura percorribili

- Data: 2026-11-08
- Stato: accettato
- Versione: da rilasciare (v0.0.23.0)

## Contesto
Le guardie stavano ferme ai cancelli senza mai riposare e il posto era fisso (il portone aperto le copriva). Si voleva sempre almeno una guardia per cancello, un luogo dove
riposano e guardie sulle mura con accesso.

## Decisione
- `abitanti.txt`: `cancello: Nord|Sud` + `turno: 1|2|3` (6–14, 14–22, 22–6) per le guardie dei cancelli; `muro: nord|sud|est|ovest` per le guardie di ronda.
  I posti si calcolano dalla geometria (`villagers::gate_post`: subito oltre la punta dell'anta aperta, lati diversi per turno) e seguono F4.
- Fase `Rest`: fuori turno la guardia cammina lungo le strade fino alla porta della casa delle guardie e sparisce; quando comincia il turno compare e raggiunge il posto.
  Il cambio arriva **un'ora prima** (la casa sta fuori da Porta Nord e Porta Sud dista ~45 min a piedi): per un'ora due guardie condividono il cancello su posti diversi.
  Test: ogni ora della giornata ha almeno un turno in servizio.
- `caserma` (edificio procedurale, `scripts/gen_guardie.py`): fuori da Porta Nord a est della strada perché dentro le mura non c'era posto libero; stradina fino alla strada principale.
- Camminamenti (`village.rs`): le torrette restano piene, quindi i camminamenti sono tratti tra due ostacoli. Si rendono percorribili il fianco più lungo e libero di ogni
  cancello (scala "della porta") più il tratto migliore a est e a ovest (lungo e in piano). Piano a ≈2,4 m sopra il terreno medio (mai meno di 1,3 m sopra il punto più alto),
  parapetto = muro rialzato, scala con alzata 0,3 m e pedata 0,38 m; i collider hanno il marker `Ground`. Tratti ostruiti da edifici o pali non sono scelti (`walk_clearance`).

## Conseguenze
- Dove il terreno sale il muro del tratto diventa più alto (a est fino a ~5 m).
- Le guardie di ronda sono fisse (nessun turno); i camminamenti non attraversano le torrette.
