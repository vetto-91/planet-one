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

## Aggiornamento: ciò che restava fuori (2026-11-15)
- **Dino e Aldo combattono** sulla strada del Sud: il loro posto della storia segue il predone vivo più
  vicino (entro 14 m, fermandosi a 1,5 m; `side_scenes::guard_spot`) e lo colpiscono come un compagno
  (`companions::strike`: Dino 11 ogni 1,2 s). Finita la difesa tornano al loro posto.
- **La Camera di Risonanza si visita** (`places::resonance_chamber`, nel modello del pozzo):
  - una stanza di pietra 7×7 m sotto la piazza, a 3,4 m di profondità, con la colonna del pozzo che
    scende in una vasca d'acqua, il cavo di rame che corre verso est e spirali di rame sui muri;
  - ci si scende da un pozzetto accanto al pozzo, con una scala a chiocciola: lì il terreno ha un buco
    (`places::WELL_HOLE`, come all'imbocco della grotta);
  - il pozzetto è chiuso da una grata di ferro su cui si cammina; quando QS-12 scopre la camera
    (`camera-risonanza`) la grata si alza e si appoggia al pozzo (`places::well_grate`);
  - festa e saluto al molo non mettono nessuno sopra il pozzetto (`lanterns::spot_is_free`);
  - QS-12 ha una tappa in più: scendere nella camera.
- **QS-01, il mulino**: il ramo è ancora nella pala e Gino non lascia il mulino per via di due predoni nel
  bosco. Escono quando arrivi (`side_scenes::mill_raiders`); a terra tutti e due, Gino libera la pala e
  ti dà il **sacco di farina** (5 kg, `Relic::FlourSack`) da portare a Marta.
- **QS-06, la cena dell'anfora**: una tappa nuova per chiedere il pane a Marta. La sera dell'anfora (dalle
  19:30) Carlo, Bruno, Beppe, Gaspare e Marta col pane si mettono davanti alla locanda, girati verso
  Osvaldo (`side_scenes::dinner_at_the_inn`); Osvaldo li saluta a tavola.
- **QS-13, la chiave e il quadro**: Don Anselmo dà la seconda chiave di rame della Luce Viva; dietro la
  stufa del Viandante c'è un **quadro di rame** vero (`PropKind::CopperPanel`) che si apre con `E` solo
  con la chiave (`quadro-rame`), con i fili che scendono nel pavimento.
- **QS-17, la casa di pietra**: dopo la casa finita, Ferruccio (cardini e grappe) e Gino (travi) ti
  aiutano ad ampliarla (`sblocca: casa-pietra`): dodici pietre e sei tronchi, terza tappa della casa
  (`house::STONE_HOUSE`). La casa diventa di pietra: stesso modello con i muri di pietra
  (`casa-naufrago-pietra`, nota «pietra» del piazzamento), con scuri, camino e interruttore.
- **Migrazione**: in `missions::MOVES` anche qs01, qs06, qs13 e qs17; chi era alla grata di QS-13
  riparte dalla chiave; chi doveva tornare dal Viandante in QS-17 fa prima la casa di pietra.
- **Prova**: `prove/zzzzzzzzzzzzzzzzzzzzz-scene-2.txt`.

## Conseguenze
- ~~La Camera di Risonanza si scopre ma non si visita.~~ **Risolto**, vedi «Aggiornamento».
- ~~Nella difesa della strada Dino e Aldo stanno fermi.~~ **Risolto**, vedi «Aggiornamento».
- Il campanile conta se ci si arriva in cima, anche cadendo dall'alto con la shell.
- La promessa di Silvia (`silvia-promessa`) aspetta l'Atto I: ora resta solo segnata.
- ~~Restano da fare QS-06, QS-01, QS-13, QS-17.~~ **Risolto**, vedi «Aggiornamento».
- La Camera di Risonanza per ora è solo da vedere: niente da raccogliere né da fare laggiù.
- Gli ospiti della cena stanno in piedi davanti alla locanda: non ci sono tavole apparecchiate.
