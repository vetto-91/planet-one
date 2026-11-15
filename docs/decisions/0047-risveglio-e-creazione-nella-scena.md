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
- ~~L'età non cambia ancora i dialoghi degli abitanti.~~ **Risolto in parte**, vedi «Aggiornamento: tono
  secondo l'età ed Ettore».
- Il riflesso nell'acqua non mostra davvero il volto: il modello del personaggio è uno solo.
- ~~Ettore non scende dal faro.~~ **Risolto**, vedi «Aggiornamento: tono secondo l'età ed Ettore».
- ~~Le partite vecchie a metà di `mp01` ritrovano gli indici delle tappe spostati di due.~~ Risolto:
  al caricamento le tappe dei salvataggi della v0.0.62 si riallineano (`missions::upgrade_step`, vedi
  ADR 0048).

## Aggiornamento: tono secondo l'età ed Ettore (2026-11-15)
- **Battute per età** (`missions::lines_for_age`): una battuta di missione può cominciare con
  `[giovane]`, `[adulto]` o `[maturo]`; si dice solo a un personaggio di quell'età (`Facts::age`, da
  `Character::age`). Le battute senza etichetta valgono per tutti.
  - Nel Prologo ce ne sono tre per età: Gaspare quando ti trova, Don Anselmo prima della grotta, Gaspare
    all'alba della partenza.
  - La voce si genera sul testo senza l'etichetta (`scripts/gen_voci_prologo.py`).
- **Ettore scende dal faro** (`awakening::ettore_comes_down`):
  - mentre «Il Nome sulla Sabbia» è alle tappe di Gaspare (trovarlo e dirgli il nome), Ettore lascia il
    faro e aspetta accanto a Gaspare sul molo;
  - quando sei vicino ti saluta con un fumetto e la sua voce; saputo il tuo nome, torna su al faro;
  - per portarlo lì la routine degli abitanti ha un posto «chiamato dalla storia» (`Villager::call_to`,
    fase `Called`), lo stesso della Sera delle Lanterne;
  - le sue battute sono in `assets/world/battute.txt` (fumetti con voce, stesso formato di
    `compagni.txt`).
- **Limiti**: gli abitanti non cambiano ancora tono nelle chiacchiere e nei saluti di tutti i giorni; il
  tono per età vale nelle battute di missione.
