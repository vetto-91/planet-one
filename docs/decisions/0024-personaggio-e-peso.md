# ADR-0024: Statistiche, peso e zaini

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.35.0

## Contesto
Il peso degli oggetti esisteva ma non limitava nulla, e il personaggio non aveva caratteristiche.

## Decisione
- `src/character.rs`: sette statistiche con 10 come valore neutro e intervallo 1–20. Forza: portata e danno in mischia. Costituzione: vita. Agilità: velocità. Carisma: prezzi. Fortuna: monete dai nemici. Intelligenza e saggezza restano pronte per la magia.
- Portata: `30 + zaino + (forza − 10) + oggetti incantati + effetti`, minimo 15 kg. Con lo zaino più capiente e forza neutra si arriva a 60 kg.
- Oltre il limite si cammina a metà velocità e non si corre, ma nulla impedisce di raccogliere: l'avviso resta a video finché non si posa qualcosa.
- Zaini (Bisaccia +5, tela +10, cuoio +20, spedizione +30) in `Equipment.backpack`, visibili sulla schiena. Oggetti incantati in `Equipment.charm`.
- `ActiveEffects` raccoglie gli effetti a tempo o senza scadenza; la stessa fonte si rinnova e non si somma. La Pozione del mulo lo usa già; le magie da lancio lo useranno quando esisteranno.
- La portata base resta anche il riferimento delle capienze dei mobili (ADR 0023).

## Conseguenze
- Il valore di ogni statistica si cambia da shell con `stat`; come crescono giocando verrà deciso con la progressione del personaggio.
