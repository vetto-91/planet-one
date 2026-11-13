# ADR-0041: Incantesimi dell'Accordo

- Stato: accettato
- Data: 2026-11-13

## Contesto
La barra della magia c'era (ADR 0039) ma non la spendeva nulla, e l'abilità Magie (ADR 0038) non si poteva
allenare. La trama (`Trama-Planet-One/02`, sezioni 2.4 e 3.3) chiama la magia «l'Accordo» e prevede incantesimi
d'attacco, di cura, di protezione e di utilità (luce, carico più leggero).

## Decisione
- **Cinque incantesimi** (`src/magic.rs`, `Spell`), imparati col grado di Magie:
  - Sfera di luce (grado 1, costa 8): un lume che segue il personaggio per due minuti;
  - Dardo di Luce Viva (grado 1, costa 12): un colpo luminoso verso il mirino, 14 danni (+4% per punto di
    Intelligenza oltre il 10);
  - Soffio di cura (grado 5, costa 20): 25 punti vita (+4% per punto di Saggezza);
  - Passo leggero (grado 10, costa 18): 15 kg di portata in più per due minuti;
  - Scudo etereo (grado 15, costa 25): per venti secondi i colpi subiti fanno metà danno.
- **Comandi**: `Z` passa all'incantesimo conosciuto successivo, `Q` lo lancia; 0,8 secondi fra un lancio e
  l'altro. Sopra la barra della magia si leggono incantesimo scelto e costo (grigio se la magia non basta).
- Ogni lancio allena Magie (metà del costo; 4 punti in più se il dardo va a segno). Il talento «Maestro
  dell'Accordo» toglie l'8% del costo per rango.
- Il pannello del personaggio elenca gli incantesimi, con il grado a cui si imparano quelli ancora ignoti.
- Lo scudo si legge da `magic::ward_factor()` nei due punti in cui il giocatore subisce danno.
- Shell: `incantesimo <luce|dardo|cura|passo|scudo>`, `magia <n>`. Pilota: `/incantesimo`, `/scudo_etereo`,
  `/sfera_di_luce`, tasti `q` e `z`.

## Conseguenze
- Aggiornamento (ADR 0042): l'etichetta dell'incantesimo scelto sta a sinistra della barra della magia.
- Gli incantesimi si imparano da soli col grado: nessun maestro, libro o altare li insegna ancora.
- Non c'è un'animazione di lancio né un suono; il dardo è una sfera luminosa.
- Scudo e sfera di luce non si salvano: caricando una partita sono spenti (la magia spesa resta spesa).
- Nessun incantesimo per calmare le creature, né lancio multiplo; i nemici non usano la magia.
- Le allenabili sono quattordici; restano ferme Armatura e Costruire.
