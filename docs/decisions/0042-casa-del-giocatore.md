# ADR-0042: La casa del giocatore sul Poggio del Naufrago

- Stato: accettato
- Data: 2026-11-13

## Contesto
La trama (`Trama-Planet-One/02`, sezione 2.4 D, e `09`, QS-17) vuole che il protagonista si costruisca una
casa con le proprie mani in un terreno libero fuori dalle mura di Fontevivo, allenando l'abilità Costruire.

## Decisione
- **Un terreno fisso** (`src/house.rs`, `PLOT` = −40, 40): a sud-ovest del paese, fuori dalle mura, lontano
  dai luoghi da scoprire. Il terreno è spianato fin dall'inizio; quattro paletti lo segnano.
- **Materiali**: due oggetti nuovi, Legname (2,5 kg) e Pietra (4 kg). Si raccolgono col tasto `R` a due
  passi dal terreno: alla catasta al margine del bosco (serve un'ascia nello zaino) e all'affioramento di
  roccia. Ogni colpo dà un pezzo, costa 10 di vigore e allena Costruire; il peso allena la Tempra.
- **Due tappe**, sempre col tasto `R` sul terreno, che mette in opera il materiale che hai con te (anche un
  po' per volta): **fondamenta** (10 pietre: un cordolo di pietra), poi **casa** (16 tronchi e 6 pietre).
- **La casa finita è un edificio vero**: un piazzamento `casa-naufrago` aggiunto in memoria in fondo
  all'elenco (`DebugPlace::set_runtime`), con lo stesso modello e gli stessi interni di `casa-piccola`. La
  porta è sempre aperta per te, i mobili sono vuoti e senza serratura: sono il tuo deposito. Non entra nel
  file dei piazzamenti e non sposta le case degli abitanti.
- **Missione QS-17** «Un Tetto tra gli Ulivi e il Mare» (Beppe, poi il Viandante), con la condizione
  `quando: conta casa <tappa>`. Le secondarie sono 16.
- Stato nel salvataggio (`[casa]`). Shell: `casa <0|1|2>`. Pilota: `/casa`.
- L'etichetta dell'incantesimo scelto passa a sinistra della barra della magia: sopra, finiva sotto il
  suggerimento dei tasti.

## Conseguenze
- Il posto non si sceglie: c'è un solo terreno, e solo sull'isola. Niente bivacco né capanna intermedia,
  niente ampliamenti (camino che fuma, luce con interruttore, fucina, orto).
- Il letto di casa non fa ancora dormire: per far passare le ore resta il tasto `T`.
- Quando la casa compare tutti gli oggetti del mondo vengono ricreati (un attimo di pausa): le porte del
  paese tornano chiuse, il contenuto dei mobili resta.
- Catasta e roccia non si esauriscono, e il legname non viene dagli alberi veri.
- ~~Costruire ora si allena ma non ha un talento.~~ **Risolto**: «Mano del Mastro», vedi sotto.

## Aggiornamento: luce con interruttore, focolare tuo e Mano del Mastro (2026-11-15)
- La casa finita aveva già un focolare col camino che fuma (è una casa piccola come le altre), ma seguiva
  l'orario del paese, e le luci si accendevano da sole quando entravi.
- **Interruttore**: accanto alla porta, dentro, c'è un interruttore (`[E] Accendi luce`). Le luci della
  casa seguono soltanto l'interruttore (`world_props::SwitchedBy`), anche da fuori, e il suo stato resta
  nel salvataggio.
- **Il focolare è tuo** (`village_life::OwnHearth`): l'orario del paese non lo accende né lo spegne più;
  lo comandi tu con `E`, e il camino fuma quando è acceso.
- **Mano del Mastro** (rango del talento di Costruire): ogni raccolta di legname o pietra mette da parte il
  15% di un pezzo per rango; arrivati a un pezzo intero, se ne raccoglie uno in più (`house::knack_bonus`).
- Restano fuori la fucina e l'orto.
