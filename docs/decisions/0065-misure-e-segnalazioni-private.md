# ADR-0065: Misure delle prestazioni e segnalazioni private

- Stato: accettato
- Data: 2026-11-27
- Versione: non ancora rilasciata

## Contesto

Le segnalazioni F12 del 27 novembre, fatte su Windows, mostrano gli FPS scendere da 122 a 18 in
un'ora e mezza. Entità, mesh, materiali, luci e suoni restano fermi. L'unico indizio è un avviso di
Bevy: il contatore dei tick del mondo del render ha superato i 3,26 miliardi. Sul Linux di sviluppo
il calo non si riproduce con chiarezza, e il PC Windows non è sempre disponibile per le prove.

Servono log che bastino da soli a capire, e che si possano mandare senza esporre l'autore o un
giocatore. Il log di quella partita conteneva il percorso `C:\Users\<nome>\…`, il nome del
personaggio nei dialoghi e, in `stato.json`, il testo dell'ultima risposta della IA.

## Decisioni

- **Riga `[PERF]` più ricca** (`src/perf.rs`, una al minuto, anche giocando):
  - fotogrammi dell'ultimo minuto, tempo medio e massimo del fotogramma;
  - tick per fotogramma del mondo principale e di quello del render. Il render li scrive in un
    `Arc` di atomici, letto dal mondo principale;
  - viste estratte (camera e ombre);
  - luci accese e visibili, luci con ombra, animazioni in corso, grafi, clip e audio caricati;
  - ora, meteo e regione.
- **Eventi della sessione** (`perf::event`): attese con T, viaggi, mappa, grafica cambiata, F12.
  Si contano e si scrivono come `[EVENTO]`.
- **`diagnosi` nella shell** (`src/diagnosis.rs`): spegne per 3 s una parte alla volta e misura il
  tempo del fotogramma senza di lei. Le parti sono: ombre del sole, luci puntiformi, luci interne,
  particelle, vegetazione, animazioni, acqua, SSAO, bloom, antialiasing e scala di rendering al 50%.
  Le opzioni grafiche cambiano solo in memoria, senza `dirty`, quindi non finiscono in
  `settings.txt`, e tutto torna com'era.
- **`salta <ore>` nella shell**: fa passare il tempo come T, così i copioni rifanno una partita
  con giorni saltati (`prove/fps-nel-tempo.txt`).
- **`prestazioni.txt` in ogni segnalazione**: la scheda della macchina (scheda video, backend,
  driver, finestra, VSync, grafica, minuti di partita, eventi) e lo storico di tutte le righe
  `[PERF]`, `[EVENTO]`, `[DIAGNOSI]` e `[GRAFICA]` della partita. Prima c'erano solo le ultime 400
  righe del log, e l'inizio del calo si perdeva.
- **Privacy delle segnalazioni** (`src/privacy.rs`). Tutto ciò che si scrive nella cartella passa
  da `Redactor`:
  - le cartelle dell'utente (`USERPROFILE`, `LOCALAPPDATA`, `APPDATA`, `TEMP`, `HOME`, `XDG_*`)
    diventano `%USERPROFILE%`, `~` e simili;
  - ogni `\Users\<nome>\`, `/Users/<nome>/`, `/home/<nome>/` diventa `<utente>`;
  - il nome del personaggio, solo come parola intera, diventa `<giocatore>`;
  - in `stato.json` il testo della chat con la IA diventa «omessa, N caratteri». Si lavora sui
    valori JSON, non sul testo, perché lì le barre di Windows sono raddoppiate;
  - il gioco non scrive il nome del computer né indirizzi di rete. `nota.txt` dice cosa contiene la
    cartella, che resta sul PC finché il giocatore non decide di mandarla.

  Il log sul terminale resta completo: è sul PC di chi gioca.
- **Guida per la prova su Windows**: `docs/beta/PROVA-WINDOWS-FPS.md`, una parte per l'autore e una
  per l'assistente che legge i dati, senza installare nulla né mandare dati in rete.

## Conseguenze

- Una segnalazione F12 fatta quando gli FPS sono bassi dice quando è cominciato il calo, cosa
  cresceva e cosa costa di più, anche da un PC che lo sviluppo non può provare.
- Gli eventi «grafica cambiata» contano anche le prove di `diagnosi`: le righe `[DIAGNOSI]` vicine
  lo rendono chiaro.
- Un test (`privacy::tests`) controlla che percorsi finti di Windows e Linux, il nome del personaggio
  e la chat non arrivino nei file della segnalazione.
