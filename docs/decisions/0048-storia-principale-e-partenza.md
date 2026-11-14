# ADR-0048: La storia principale del Prologo completa e la partenza a due vie

- Stato: accettato
- Data: 2026-11-14

## Contesto
Quinto passo del piano per il Prologo. Le missioni MP-03, MP-04 e MP-05 (ADR 0034) erano fatte di luoghi e
pensieri. Per la trama (`Trama-Planet-One/06`, sezioni 1–2, e `09`, sezione 1.5) servono anche:
- gli oggetti veri (il Disco di Tiro, il Quipu Maestro);
- Agnese che studia la pietra del santuario;
- le stele del Cenote che risuonano e aprono la Porta Sommersa;
- il tempo della storia che avanza;
- la Sera delle Lanterne con i doni;
- la scelta fra la via del mare e la via delle profondità.

## Decisione
- **Salti di tempo**: effetto `tempo <ora> [giorni] [meteo]` (anche come riga di tappa `tempo:`).
  - Alla prima tappa di MP-03 si va avanti di 10 giorni, alle 9, col sereno.
  - In MP-04 di 20 giorni, alle 10, con la pioggia che gonfia il Cenote.
  - In MP-05 di 25 giorni, alle 19:30.
  - Il Prologo passa così dal 21 marzo a metà maggio.
  - Il meteo di scena (`WeatherState::scene`) vale solo per quel giorno e cede a un meteo forzato.
- **MP-03**: una tappa nuova con Agnese, che riconosce nella pietra del santuario un'Intihuatana. In cima alla
  torre si trova il **Disco di Tiro**.
- **MP-04**: una tappa nuova al laghetto sotterraneo (`magic::CENOTE`). Un incantesimo lanciato lì fa
  risuonare le quattro stele (`karma::flag("stele-risonanti")`, letto con `quando: sbloccato`). In fondo al
  cunicolo si trovano il **Quipu Maestro** e la **Porta Sommersa** aperta (`sblocca: porta-sommersa`).
- **MP-05**:
  - Gaspare racconta la preparazione della barca;
  - alla Sera delle Lanterne arrivano i **doni** (`doni:`): ogni abitante che ti aveva affidato una missione
    secondaria compiuta porta qualcosa (`missions::gifts`), e un avviso dice da chi;
  - prima dell'alba Gaspare offre la scelta:
    - **la via del mare**;
    - **la via delle profondità**, solo se la Porta Sommersa è aperta (risposta `[sbloccato porta-sommersa]`);
    - **«non ancora»**.
  - Le prime due avviano una **scena raccontata** (`src/cutscene.rs`): dissolvenza al nero, quattro o cinque
    righe e poi il viaggio sul continente (`viaggio nord mare|profonda`). Nella via del mare Gaspare tiene il
    timone solo se ti ha confidato il suo segreto (`gaspare-timoniere`).
- **Segnali per l'Atto I e i finali**: la via scelta (`via-mare`, `via-profonda`) resta tra le scelte
  salvate. Sorte del ladro, karma, rito dei banditi e segreti erano già nel salvataggio.

## Conseguenze
- La traversata non si guida: è raccontata (scelta presa con l'utente).
- Le due vie arrivano allo stesso approdo del continente: il Grande Cenote dell'Alba e il porto di
  Tiro-Scheria non esistono ancora.
- La Porta Sommersa non si vede nel cunicolo: si sa aperta dal pensiero e dalla scelta.
- Alla Sera delle Lanterne la piazza non si riempie e non si accendono lanterne in più: ci sono i doni e il
  saluto.
- Le partite vecchie a metà di MP-03 o MP-04 ritrovavano gli indici delle tappe spostati (le tappe nuove
  si inseriscono prima della torre e del cunicolo; MP-05 ha le stesse tappe di prima). **Risolto** con la
  migrazione dei salvataggi:
  - `missions::upgrade_step` ha, per `mp01`, `mp03` e `mp04`, la tappa nuova che corrisponde a ogni tappa
    vecchia. `SaveData::parse` la applica ai file salvati prima della v0.0.63.0 (riga `versione`), anche
    alle regioni annidate;
  - chi era fermo dove ora c'è una tappa nuova (Agnese, le stele) la fa; chi aveva già parlato a Gaspare
    salta la tappa del nome;
  - dopo lo spostamento il file letto vale come v0.0.63.0, così una regione annidata riscritta non si
    sposta due volte;
  - prima della v0.0.62 le missioni non esistevano: non c'è altro da spostare.
