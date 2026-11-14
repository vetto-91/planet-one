# ADR-0045: Ciò che sbloccano i segreti, la via degli indizi e i valori degli abitanti

- Stato: accettato
- Data: 2026-11-14

## Contesto
Secondo passo del piano per il Prologo. Con l'ADR 0040 i 26 segreti si raccontavano ma non portavano nulla,
si scoprivano solo con la fiducia, e ogni abitante apprezzava le stesse azioni. La trama
(`Trama-Planet-One/04`, sezione 7, e `02`, sezione 4.5) chiede che ogni segreto sblocchi qualcosa, che si
possa scoprire anche con gli indizi, e che ognuno giudichi secondo i propri valori.

## Decisione
- **`sblocca:`** in `assets/world/segreti.txt`, con la sintassi degli effetti delle missioni
  (`dai <oggetto> <n>; sblocca <cosa>`).
  - Alla confidenza gli oggetti arrivano nell'inventario: la Mappa delle Secche da Gaspare, le focacce da
    Marta, il bulbo di Luce Viva da Carlo, la chiave di rame da Pietro, pozioni da Rosa.
  - Le altre voci diventano segni salvati (`Karma.unlocks`, `[karma] sbloccato:<cosa>`): `gaspare-timoniere`,
    `silvia-compagna`, `dino-alleato`, `rito-del-nome`, `via-del-cenote`, `credenziale-diomede`…
  - Le missioni li leggono con `quando: sbloccato <cosa>`; li useranno la partenza, i compagni e l'Atto I.
- **Via degli indizi**: quattro quaderni e lettere nuovi (oggetti di trama), uno nel primo mobile di un
  edificio:
  - il Registro delle Notti al faro (Ettore);
  - la Scatola dei semi al mulino (Gino);
  - la lettera del nonno alla locanda (Osvaldo);
  - il Taccuino di caccia dall'arciera (Silvia).

  Appena sono nell'inventario rivelano il segreto, senza bisogno della fiducia, allenano Sapienza e piacciono
  a chi ama lo studio. Nel diario compaiono anche gli abitanti conosciuti solo così.
- **Valori**: `ama:` dice che cosa apprezza ogni abitante (pietà, onestà, aiuto, lavoro, coraggio, studio,
  esplorazione, quiete). Le azioni hanno un valore (`karma::deed_as`, `karma::praise`):
  - risparmiare il ladro e i doni nascosti valgono come pietà;
  - consegna, furti, patto e monete tenute valgono come onestà;
  - una missione compiuta vale come aiuto, costruire come lavoro, un nemico sconfitto come coraggio,
    un luogo scoperto come esplorazione;
  - nelle missioni si scrive `karma 3 #pieta motivo`.

  Chi tiene a quel valore cambia fiducia della metà della variazione (almeno uno), in su o in giù.
- **Guardie**: se un borseggio va male, ogni guardia entro 18 m l'ha visto, aggiunge 10 monete di multa e
  perde fiducia.
- **Fama nel saluto**: con il karma oltre +25 o sotto −25, chi non è tuo amico apre il dialogo con una battuta
  sulla tua fama.

## Conseguenze
- I segni sbloccati per ora non cambiano il mondo: li useranno i passi successivi (partenza, compagni, Rito
  del Nome).
- Mancano gli indizi di Beppe (l'astrolabio in casa sua), della chiesa e della torre: la loro casa non ha un
  mobile riconoscibile dal codice dei contenitori.
- Il pedinare un abitante la sera non c'è.
- La fama cambia una sola battuta, non le voci né il comportamento.
