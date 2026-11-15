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
- ~~Mancano gli indizi di Beppe, della chiesa e della torre~~, ~~il pedinare un abitante la sera non c'è~~ e
  ~~la fama cambia una sola battuta~~: risolti, vedi «Aggiornamento» sotto.

## Aggiornamento (2026-11-14): gli indizi mancanti, il pedinamento e la fama
- **Tre indizi nuovi** (oggetti di trama, icone da `scripts/make_item_icons.py`):
  - l'**Astrolabio spezzato** nel primo mobile della casa di Beppe → il segreto di Beppe;
  - le **Pagine antiche degli Approdi** nel baule nuovo della sacrestia, accanto all'altare → Don Anselmo;
  - il **Medaglione di Lauro** nel baule in cima alla torre → Rosa (come nella trama, `04`, sezione 7).
- **Una casa precisa**: in `karma::CLUES` l'edificio può essere un tipo («faro») o una casa precisa scritta
  come in `abitanti.txt` («casa-media 0»). I mobili delle case portano il campo `Container.home`, che passa
  anche dallo scasso (`Lockpicking.home`). Forzare il baule della sacrestia pesa come quello di una casa
  (−2, onestà).
- **Pedinare** (`src/tailing.rs`): con `pedina: dalle alle x z dx dz` in `abitanti.txt` un abitante, la sera,
  va di nascosto in un posto (una fase nuova della routine, `Phase::Secret`, che usa le stesse strade).
  - Marta, dalle 19:30, porta il dolce alla cesta di Gino al mulino; Agnese, dalle 18:12, va ad ascoltare la
    pietra del santuario.
  - Chi le sta dietro mentre camminano (entro 22 m, per almeno 12 secondi) e all'arrivo è entro 16 m ne
    scopre il segreto, e allena la Furtività.
  - Più vicino di 2,5 m, o davanti entro 9 m (meno con la Furtività), si viene notati: per quel giorno è
    fallito, e la fiducia scende di uno.
  - ~~Pietro e la legnaia restano fuori: nel mondo la legnaia non c'è e il suo posto serale è a tre metri dal
    pozzo.~~ **Risolto** (ADR 0055): la legnaia c'è, e la sera Pietro ci va di nascosto.
- **La fama**:
  - i prezzi la seguivano già (ADR 0040);
  - sotto −40 chi tiene all'onestà o alla pietà e non è ancora tuo conoscente non ti parla («Non ho niente da
    dirti. Lasciami in pace.»), e la chiacchierata non gli fa crescere la fiducia. I venditori e le battute
    delle missioni restano;
  - le guardie e un borseggio scoperto (`karma::guard_watch`): sotto −25 lo vedono da 26 m e la multa in più
    è 20 monete; da +40 in su lo vedono da 12 m e la multa è 5; altrimenti 18 m e 10 monete.
- **Prove**: shell `dietro <abitante> <metri>` (a distanza conta il verso in cui cammina); copioni
  `zzzzzzzzzzzzzz-pedinare` (Agnese si accorge di te, Marta no) e `zzzzzzzzzzzzzzz-indizi` (torre, sacrestia,
  Beppe che non ti parla).
- Restano fuori: l'astrolabio non ha un copione (la casa di Beppe è chiusa a chiave); la fama non cambia
  ancora le voci in paese.
