# ADR-0035: I bivi di «Denaro Rubato»

- Stato: accettato
- Data: 2026-11-13

## Contesto
La trama (`Trama-Planet-One/01`, sezione 4.3) fa di «Denaro Rubato» il primo bivio morale del gioco: acciaio,
grazia, patto segreto, tenersi parte dell'oro. Nel gioco il ladro si poteva solo sconfiggere.

## Decisione
- **Parlare al ladro**: finché il giocatore non ha un'arma in pugno e non lo ha colpito né sfidato, il ladro
  non attacca (`QuestState::thief_calm`, letto dall'IA dei nemici e dal danno da contatto) e da vicino gli si
  parla con E (`thief_parley`). Tre risposte:
  - *grazia* — «Ridammi le monete»: accetta se hai letto la sua **tavoletta** al campo (o hai Carisma 7+),
    lascia le dodici monete e se ne va; altrimenti rifiuta e combatte;
  - *patto* — sei monete a testa: la quest passa al ritorno con sei monete;
  - *sfida* — combatte.
- **Alla consegna** il Viandante chiede quante monete hai trovato: tutte e dodici (ricompensa; con il ladro
  risparmiato, anche la **Moneta Madre**) oppure «solo otto» (quattro restano a te, nessuna ricompensa).
- L'esito resta nello stato della quest e nel salvataggio (`sorte_ladro`, `tavoletta_letta`, `monete_tenute`,
  `moneta_madre`), nel diario e nello stato letto dal pilota: servirà al seguito della storia sul continente.
- Testi in `assets/quests/denaro_rubato/dialoghi.txt` (sezioni nuove). Copioni `zz-bivio-grazia`,
  `zz-bivio-patto`, `zz-bivio-ombra`.

## Conseguenze
- Un giocatore appena arrivato (senz'armi) che trova il ladro non viene più attaccato a vista: può parlargli.
- Mancano rispetto alla trama: il borseggio (non c'è un sistema di furtività), la Moneta Madre come oggetto
  nell'inventario, le reazioni degli abitanti e dei personaggi del continente all'esito.
- Le battute nuove non hanno voce registrata.
