# ADR-0055: Le rifiniture dell'isola

- Stato: accettato
- Data: 2026-11-15

## Contesto
Quinto e ultimo passo del piano per completare isola, Prologo e traversata (dopo ADR 0051–0054). Restavano
delle rifiniture sparse, annotate nelle «Conseguenze» degli ADR precedenti:
- cinque luoghi senza voce `luogo:`: non comparivano sulla mappa e non si raggiungevano col viaggio veloce
  (ADR 0029, 0037);
- la pedinata di Pietro mancava, perché nel paese la legnaia non c'era (ADR 0045);
- sei secondarie non avevano un copione di prova (ADR 0036);
- due talenti con nome della trama mancavano: l'apnea e il borseggio delle lettere (ADR 0049; doc 02 della
  trama, abilità 3 e 11);
- l'MP-05 portava avanti il calendario di 25 giorni, a metà maggio; la trama vuole la partenza all'inizio
  dell'estate (doc 09, §1.5; ADR 0048).

## Decisione
- **Cinque luoghi nuovi** in `assets/world/isola.txt`: Faro, Porticciolo, Laghetto, Ponte sul ruscello e
  Poggio del Naufrago. Si scoprono avvicinandosi, come gli altri, e poi si raggiungono dalla mappa.
- **La legnaia e la porticina** sono due tipi di piazzamento nuovi (`buildings::model`, spostabili con F4):
  - la legnaia è una tettoia a uno spiovente con le cataste, il ceppo con l'accetta e una scaletta sul fianco
    (P-1290, contro le mura a sud-ovest, dietro la chiesa);
  - la porticina è una piccola porta di assi ferrate nello stipite di pietra, appoggiata alle mura dal lato
    del paese (P-1291).
  - Pietro, dalle 19:36 alle 20:54, va di nascosto ai piedi della scaletta e guarda la porticina
    (`pedina:` in `abitanti.txt`). Chi lo segue senza farsi vedere ne scopre il segreto, come per Marta e
    Agnese.
- **Apnea dei Pescatori di Perle** (`src/diving.rs`): il talento attivo del Nuoto (grado 30).
  - In acqua profonda, tenendo premuto `Ctrl`, ci si immerge fino al fondale.
  - Il fiato dura 20 secondi più 4 per ogni rango del talento del Nuoto, e a galla torna in fretta. La barra
    «Fiato» compare sopra la vita solo quando cala.
  - Finito il fiato si risale per forza; finché la testa resta sotto si perde vita, mai sotto 1.
  - Una **cassa sommersa** sta al largo della spiaggia del relitto, sul primo fondale profondo verso sud
    (`diving::sunken_spot`). Dentro ci sono monete, pozioni e grimaldelli.
- **Borseggio delle lettere**: con le Dita di Mercurio, insieme alle monete vengono via anche le carte che un
  abitante porta in tasca. Sono indizi nuovi (`karma::POCKET_CLUES`) e, letti, ne rivelano il segreto:
  - il biglietto di Gino per Marta, mai consegnato;
  - il profilo di cuoio della Guardia Gino, con le montagne del continente;
  - il taccuino di poesie della Guardia Mario.
- **L'estate**: l'effetto `tempo:` accetta `estate` al posto dei giorni e porta al primo giorno d'estate,
  il 21 giugno (`missions::jump`). L'MP-05 ora fa `tempo: 19.5 estate sereno`. Se è già estate, o più tardi,
  il salto non c'è.
- **Il copione delle sei secondarie** (`prove/zzzzzzzzzzzzzzzzzzzzzzzz-secondarie-2.txt`) gioca per intero
  qs02, qs05, qs07, qs08, qs11 e qs13. Le altre missioni sono chiuse all'inizio, così ogni abitante parla
  della sua.
- Il copione `zzzzzzzzzzzzzzzzzzzzzzzzz-rifiniture` prova i luoghi, la legnaia, la pedinata di Pietro, il
  biglietto sfilato, l'apnea con la cassa sommersa e il salto all'estate.

## Conseguenze
- Isola, Prologo e traversata sono completi rispetto al piano. Restano per l'Atto I:
  - le missioni, gli eventi e i segreti del continente;
  - Tiro-Scheria e il Cenote dell'Alba (doc 06).
- Pietro sta ai piedi della scaletta, non sul tetto: gli abitanti camminano solo a terra.
- Nel paese la porticina è solo da guardare: non si apre e non si passa.
- Il laghetto è profondo 0,9 m e non ci si immerge. L'apnea serve in mare.
- Le guardie di turno, nel copione, non sono mai «Ready»: si aspetta che arrivino al loro posto.
- Le tre carte da sfilare hanno icone già esistenti (lettera, mappa, taccuino).
