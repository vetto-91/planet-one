# ADR-0043: La Prova dell'Arco

- Stato: accettato
- Data: 2026-11-14

## Contesto
La missione QS-14 della trama (`Trama-Planet-One/09`), ispirata alla gara dell'arco dell'Odissea, chiede di
centrare tre anelli di ferro appesi lungo il ruscello, dal ponte di legno. Mancavano i bersagli.

## Decisione
- **Tre anelli** (`src/archery.rs`) appesi a tre pali oltre il ponte (81, −8), a 9, 14 e 19 metri verso valle,
  sfalsati ai due lati dell'acqua (−2,6 m, +2,6 m, al centro) così che una freccia ne attraversi uno solo.
- **Regola**: una freccia che passa dentro il ferro (55 cm dal centro) fa valere l'anello. Servono il vicino,
  il medio e il lontano, in quest'ordine, con tre frecce consecutive: una freccia a vuoto vicino agli anelli,
  o dentro l'anello sbagliato, fa ricominciare dal primo. Superata la prova non la si perde più.
- La freccia si riconosce dal tratto percorso in un frame che attraversa il piano dell'anello (`through`).
  Ogni anello centrato allena il Tiro con l'Arco; la prova superata di più.
- **Missione QS-14** «La Prova dell'Arco» (Silvia), con `quando: conta arco 1`. Le secondarie sono 17.
- Stato nel salvataggio (`prova_arco` in `[casa]`). Shell: `scocca <1|2|3>` tira una freccia mirata
  all'anello, compensando la caduta (serve alle prove automatiche). Pilota: `/anelli`, `/prova_arco`.
- `combat::loose` scocca una freccia del giocatore: la usano l'arco e il comando della shell.

## Conseguenze
- Niente vento laterale sulle frecce, niente suono dell'anello centrato.
- La seconda parte della missione della trama (l'imbocco della grotta, il bracciale del maestro di Silvia) e
  i premi «Arco Maestro» e faretra non ci sono: Silvia dà frecce e monete.
- Gli anelli non reagiscono (non oscillano, non cambiano colore): l'esito si legge negli avvisi.
- Si può tirare da qualunque punto, non solo dal ponte.
