# ADR-0047: Il risveglio sulla spiaggia: la creazione del personaggio nella scena

- Stato: accettato
- Data: 2026-11-14

## Contesto
Quarto passo del piano per il Prologo. La trama (`Trama-Planet-One/02`, sezione 2, e `01`) vuole che nome, età e
classe si scelgano nella storia, sulla spiaggia del naufragio:
- l'età guardandosi nell'acqua ferma («lo Specchio della Marea»);
- la classe da ciò che il corpo fa per primo tra i rottami;
- il nome detto al soccorritore, il primo atto di libero arbitrio.

Finora tutto si sceglieva in una schermata del menu, e l'età non esisteva.

## Decisione
- **Età** (`character::Age`): giovane, adulto, maturo. Segna appena le statistiche di partenza (giovane +1
  Agilità −1 Saggezza; maturo +1 Saggezza +1 Intelligenza −1 Agilità −1 Costituzione) e si salva (`eta`).
- **Menu**: «Nuova partita» ora dice solo che ci si sveglia sulla spiaggia, con il pulsante «Svegliati». Il
  personaggio parte come `Character::castaway()`, «Nessuno», con la cartella dei salvataggi «Naufrago» (o
  «Naufrago 2»…).
- **Sulla spiaggia** (`src/awakening.rs`, stato `AppState::Awakening` per i pannelli):
  1. una pozza d'acqua ferma fra le rocce: E apre lo Specchio della Marea e si sceglie l'età;
  2. quattro oggetti tra i rottami:
     - una sbarra di ferro (Guerriero, spada corta);
     - un arco spezzato (Esploratore, arco e frecce);
     - un rotolo di nodi (Studioso, quipu e pozioni);
     - una borsa (Mercante, bisaccia e monete).

     Il primo che si raccoglie rivela l'indole: si accetta, oppure «scelgo io i miei modi» apre la
     distribuzione libera dei 14 punti (Personalizzato). Si ricevono anche 2 punti statistica da spendere
     nel pannello C;
  3. Gaspare chiede il nome alla fine del suo dialogo: il pannello si apre da solo, il nome si scrive, la
     cartella dei salvataggi lo prende (`save::rename_session`). Parlandogli di nuovo, ti chiama per nome.
- **Missione `mp01`**: le tappe nuove aspettano `quando: conta risveglio 1/2/3` (salvato come `risveglio` in
  `[personaggio]`; le partite vecchie risultano già sveglie).
- I rottami della spiaggia (P-1280…1282) contengono mele, una pozione, qualche moneta, una bisaccia.
- Shell e pilota: `risveglio nuovo`, `risveglio eta <età>`, `risveglio classe <classe>`,
  `risveglio nome <nome>`; nello stato del pilota `/personaggio/{risveglio,eta,classe,nome}`.

## Conseguenze
- L'età non cambia ancora i dialoghi degli abitanti (la trama prevede toni diversi per giovani e maturi).
- Il riflesso nell'acqua non mostra davvero il volto: il modello del personaggio è uno solo.
- Ettore non scende dal faro: lo cita Gaspare.
- Le partite vecchie a metà di `mp01` ritrovano gli indici delle tappe spostati di due.
