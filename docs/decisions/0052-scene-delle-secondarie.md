# ADR-0052: Le scene delle missioni secondarie

- Stato: accettato
- Data: 2026-11-15

## Contesto
Secondo passo del piano per completare isola e Prologo (dopo le ricompense, ADR 0051). La trama
(`Trama-Planet-One/09`, sezione 3) vuole quattro scene con persone sul posto che nel gioco mancavano:
- il duello e la difesa della strada con Dino e Aldo (QS-09);
- il campanile della chiesa (QS-12);
- la grotta con Silvia e il bracciale del maestro (QS-14);
- il paese al porticciolo per la partenza (QS-15).

## Decisione
- **QS-09, L'Armatura Lucida** (tre tappe nuove dopo l'acqua bassa):
  - **duello amichevole**: Dino aspetta fuori da Porta Sud. Ogni fendente in mischia che lo raggiunge,
    davanti a te e a portata dell'arma, è un colpo a segno; al terzo si chiude (`duello-dino`). Il
    sistema guarda i fendenti (`combat::SwingTimer`) senza toccare il combattimento: Dino non si ferisce;
  - **la strada del Sud al tramonto** (18–22:30): Dino e Aldo si mettono sulla strada, girati verso il
    bosco; quando arrivi escono due predoni (`SouthRaider`, che non ricompaiono). A terra tutti e due:
    `predoni-sud-vinti`;
  - **Aldo** ti ringrazia: fiducia di Dino e Aldo, karma +3 (aiuto).
- **QS-12, La Croce delle Quattro Ronde**: prima del pozzo si sale **in cima al campanile** a
  mezzogiorno (vicino alla cella, sopra gli 8 m; la scala a chiocciola c'era già). Al pozzo si scopre la
  **Camera di Risonanza** (`sblocca: camera-risonanza`).
- **QS-14, La Prova dell'Arco** (tre tappe nuove dopo la prova):
  - Silvia aspetta all'imbocco della grotta a est;
  - nella prima sala si trova il **bracciale del maestro** (`Relic::MasterBracelet`);
  - riportato a Silvia, promette di venire con te nelle foreste del nord (`sblocca: silvia-promessa`).
- **QS-15, Il Cantiere di Fontevivo**: se è compiuto, prima dell'alba della partenza (MP-05, ultima
  tappa, dalle 3 alle 9:30) il paese scende al molo dietro Gaspare, in file sfalsate girate verso il
  mare (`side_scenes::crowd_spots`, posti liberi controllati nella fisica come alla Sera delle Lanterne).
  Restano al loro posto Gaspare, Ettore (al faro) e le ronde sulle mura. La scena della partenza ha una
  riga in più: il paese che saluta dal molo (`cutscene::departure_lines`, per tutte e due le vie).
- **I posti della storia non si pestano i piedi**: ogni scena rimette a ogni frame il posto che ha dato,
  e lo toglie solo se è ancora suo (`side_scenes::call`). La Sera delle Lanterne rimanda a casa solo chi
  aveva chiamato lei, e non chiama chi ha già un posto.
- **Chi è per strada cambia subito direzione** quando il suo posto della storia cambia o sparisce
  (`Villager::recall`): prima finiva il percorso vecchio. Alla fine della Sera delle Lanterne, con l'ora
  saltata all'alba, Gaspare camminava ancora fino alla piazza prima di tornare al molo.
- **Migrazione dei salvataggi**: le tappe aggiunte stanno in una tabella per versione
  (`missions::MOVES`, `STEP_LAYOUT = v0.0.65.0`). Le missioni compiute con la v0.0.64 restano compiute;
  chi era fermo al pozzo di QS-12 resta al pozzo.
- **Battute**: Dino e Aldo hanno i fumetti del duello e della difesa in `assets/world/battute.txt`, con
  le voci; le battute nuove di Dino, Silvia e Aldo nelle missioni hanno le loro voci (Piper locale).
- **Prova**: `prove/zzzzzzzzzzzzzzzzzzzz-scene.txt` (duello, strada del Sud, Aldo, campanile e pozzo,
  Silvia e il bracciale, il paese al molo). `zz-ultime.txt` era rimasto indietro dalla v0.0.64 (eventi e
  ronde cambiati): ora salta con la shell ciò che provano i copioni più nuovi.

## Conseguenze
- La Camera di Risonanza si scopre ma non si visita: non c'è ancora una stanza sotto il pozzo.
- Nella difesa della strada Dino e Aldo stanno fermi: non combattono (non sono compagni).
- Il campanile conta se ci si arriva in cima, anche cadendo dall'alto con la shell.
- La promessa di Silvia (`silvia-promessa`) aspetta l'Atto I: ora resta solo segnata.
- Restano da fare, delle scene della trama: QS-06 (il pane e i quattro ospiti), QS-01 (la pala e i
  predoni), QS-13 (la chiave antica e il quadro di rame), QS-17 (la casa di pietra).
