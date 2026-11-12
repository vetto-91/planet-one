# ADR-0034: Missioni a tappe da file di testo e storia principale del Prologo

- Stato: accettato
- Data: 2026-11-12

## Contesto
La trama (`Trama-Planet-One/`, documenti 01, 06 e 09) prevede sull'isola cinque missioni principali; nel gioco
c'era solo «Denaro Rubato», scritta nel codice con i suoi stadi. Servono molte missioni (principali ora,
secondarie poi) senza scrivere un modulo per ciascuna.

## Decisione
- Un **motore di missioni a tappe** (`src/quests/missions.rs`) legge `assets/quests/prologo/missioni.txt`.
  Ogni missione è una fila di tappe; ogni tappa ha un obiettivo per il diario, una o più condizioni
  (`vicino x z raggio`, `sotto`/`sopra` una quota, `ora da a`, `parla <abitante>`, `quest accettata|completata`),
  le battute dell'abitante con cui parlare (`> testo`, con `{nome}`), un pensiero mostrato a tappa fatta,
  un premio in monete e ciò che sblocca. Le missioni si concatenano con `dopo:`.
- «Denaro Rubato» resta com'è e fa da anello della catena (MP-02) tramite le condizioni `quest`.
- Le quattro missioni nuove: **Il Nome sulla Sabbia** (risveglio sulla spiaggia a est del molo, Gaspare,
  salita a Fontevivo, il Viandante), **Le Voci delle Pietre Antiche** (Viandante, Gino, rovine, santuario,
  torre antica di notte), **Sotto l'Arco dei Sei Gradoni** (Don Anselmo, grotta, laghetto, cunicolo),
  **Oltre il Velo del Nord** (Gaspare, Sera delle Lanterne in piazza, partenza prima dell'alba).
- L'ultima tappa sblocca `continente`: da allora Gaspare offre il passaggio (ADR 0033). Dal continente si
  torna sempre; la shell (`regione`) resta libera.
- Una **partita nuova comincia sulla spiaggia** tra i rottami (botti e casse piazzate), non più in paese.
- Il dialogo di missione ha la precedenza sul saluto dell'abitante; vale solo se letto fino in fondo.
- Stato nel salvataggio (sezione `[missioni]`); nel diario (J) e negli avvisi a schermo.
- Nelle prove automatiche le missioni sono ferme (così i copioni esistenti non cambiano) finché il copione
  non scrive `missioni on`; `missione <id> <tappa>` salta a una tappa. Copione `z-prologo`: tutta la storia.

## Conseguenze
- Aggiungere o ritoccare una missione è un lavoro di testo; le missioni secondarie potranno usare lo stesso motore.
- Mancano rispetto alla trama: la creazione del personaggio dentro la scena del risveglio (nome, classe e
  statistiche si scelgono ancora nel menu), i bivi morali di «Denaro Rubato», i salti di data fra una missione
  e l'altra, gli oggetti di trama (disco di bronzo, quipu), la seconda via sotterranea verso il continente,
  le voci registrate per le battute di missione.
- La ricerca dei nomi nella shell e nel pilota ora preferisce il nome esatto («gino» è il mugnaio, non la
  Guardia Gino): prima il risultato cambiava da un avvio all'altro.
