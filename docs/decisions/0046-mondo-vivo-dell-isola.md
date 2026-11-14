# ADR-0046: Mondo vivo dell'isola: voci sui luoghi, esplorazione, bottino, mareggiate, pedaggio

- Stato: accettato
- Data: 2026-11-14

## Contesto
Terzo passo del piano per il Prologo. Dei quattro stati dei luoghi della trama (`Trama-Planet-One/10`,
sezione 8.1, e `02`, sezione 4.7) c'erano solo «sconosciuto» e «scoperto». Mancavano anche:
- il bottino che si rigenera;
- le casse delle mareggiate da scassinare onestamente (`02`, sezione 2.4E);
- l'evento EC-05, il pedaggio del ponte.

## Decisione
- **Voci** (`places::RUMORS`): alla prima chiacchierata alcuni abitanti nominano un luogo che non conosci
  (Ida il santuario, Gino e Rosa la torre, Silvia il campo dei banditi, Nina la grotta…). Il luogo compare
  sulla mappa con un «?» grigio e l'avviso «Nuovo luogo annotato».
  - La voce sta nella stessa tabella dei luoghi scoperti, con la chiave `?regione|nome`, e si salva con loro.
  - Arrivandoci diventa un luogo scoperto.
- **Esplorazione** (`places::Exploration`, ricalcolata ogni secondo, non salvata): un luogo scoperto è
  «esplorato» (✓ sulla mappa) quando ogni mobile entro 18 m è stato aperto e non ci sono nemici vivi.
  - Se i nemici tornano, torna «in parte».
- **Bottino che torna** (`island_life::refill_loot`): a ogni notte nuova i mobili svuotati dei luoghi fuori
  dalle mura e delle casse delle mareggiate si riempiono di nuovo con probabilità 0,4.
  - Il contenuto è diverso ogni volta (la chiave del tiro cambia con la notte).
- **Casse delle mareggiate**: quattro bauli col lucchetto di bronzo sulle spiagge, trovati lungo la costa a
  quota di sabbia e distanti fra loro (sempre gli stessi). Dentro ci sono monete, a volte pozioni, frecce o
  un grimaldello.
  - Si scassinano senza toccare il karma; forzare un baule in una casa invece pesa (−2, onestà).
- **Pedaggio del ponte** (EC-05): al tramonto e di notte, avvicinandosi al ponte sul ruscello, due «Banditi
  del ponte» sbarrano il passo. Il dialogo (`eventi.txt`, `ec05`) offre tre risposte:
  - pagare 10 monete;
  - minacciarli citando l'uomo dal mantello rosso: la risposta compare solo con Carisma ≥ 12;
  - sfidarli.

  Chi paga o li convince li vede andarsene; chi li sfida li trova nemici.
- **Risposte con requisito** nel motore delle missioni: `scegli: [carisma 12] testo => …`. La risposta è
  nascosta se la statistica non basta, e la scelta mostrata viene ricondotta a quella della tappa.

## Conseguenze
- Il pedaggio capita una volta sola (è un evento a tappa unica).
- ~~Gli eventi EC-02, EC-04 ed EC-08 sono ancora incontri raccontati~~ e ~~l'esplorazione conta solo mobili e
  nemici~~: risolti, vedi «Aggiornamento» sotto.
- Il bottino torna anche nei mobili dei luoghi che non si vedono (nessun limite di distanza).

## Aggiornamento (2026-11-14): eventi con persone sul posto, le quattro ronde, i segreti dei luoghi
- **EC-02, il carretto nella nebbia** (`island_life::cart_in_the_mud`): al crepuscolo, sulla strada del nord,
  c'è davvero un carretto con la ruota nel fango e **Lino il carrettiere** (personaggio di passaggio, voce
  generata).
  - Al primo dialogo escono dal bosco due predoni.
  - Quando sono a terra (`sbloccato predoni-vinti`) si aiuta Lino: mele e karma (+3, aiuto).
  - A evento chiuso Lino e il carretto spariscono.
- **EC-04, la pecora smarrita** (`animals::lost_sheep`): di giorno una pecora vera, impigliata al margine dei
  rovi a est del pascolo.
  - Avvicinandosi la si libera senz'armi; poi ti segue, correndo se resta indietro, e non scappa.
  - Al pascolo torna una pecora del gregge (`pecora-liberata`, `pecora-al-gregge`; karma +2, pietà).
- **EC-08, la disputa del mercato** (`island_life::market_quarrel`): in tarda mattinata Tonio e Nina si
  rimbeccano a voce alta (fumetti a turno) finché non fai da giudice; tre verdetti, con fiducia e karma.
- **Nemici di un evento** (`combat::NoRespawn`): i predoni e i banditi del pedaggio, uccisi, non ricompaiono.
  Prima i banditi del ponte tornavano come nemici comuni dopo qualche minuto.
- **QS-12 dalle ronde**: le tappe sulle mura diventano dialoghi con le quattro guardie di ronda, ognuna nel
  suo turno (Gino di mattina, Ivo nel pomeriggio, Livio e Mario di notte), con +3 di fiducia ciascuna. Resta
  la tappa finale al pozzo a mezzogiorno.
- **I segreti dei luoghi** (`Missions::events_near`): un luogo è «esplorato» solo quando anche gli eventi che
  capitano entro 18 m (la scintilla sulla torre, l'alba sui gradoni della grotta, la nave fantasma dietro il
  mulino…) sono stati vissuti.
- **Animali**: il raggio che cerca gli ostacoli davanti a un animale colpiva il suo stesso corpo (un collider
  figlio), quindi pecore, mucche e galline restavano ferme; ora il corpo è escluso (`blocked_ahead`).
- **Prove**: copione `zzzzzzzzzzzzzzzz-eventi` (lite e verdetto, pecora, prima ronda, carretto e predoni).
