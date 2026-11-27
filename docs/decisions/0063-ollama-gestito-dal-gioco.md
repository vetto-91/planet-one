# ADR-0063: Ollama avviato e chiuso dal gioco

- Stato: accettato
- Data: 2026-11-27
- Versione: v0.0.69.0

## Contesto

Finora i dialoghi con la IA (ADR 0005) chiedevano al giocatore di installare Ollama nel sistema. Così
Ollama gira come servizio sempre acceso, anche a gioco chiuso: tiene occupata la porta 11434 e, finché il
modello resta caricato, la memoria. Si vuole un'IA che esista solo mentre si gioca: parte con il gioco, si
chiude con lui, anche in caso di crash, e non lascia nulla in ascolto.

Due strade possibili:
1. Ollama dentro il pacchetto del gioco: aggiunge 1–2 GB a ogni release, per tutti, anche per chi non usa
   la IA.
2. Ollama scaricato una volta dallo script di installazione, nella cartella dati del gioco.

Si è scelta la seconda (decisione dell'autore): la release resta leggera e il modello va comunque scaricato
a parte (circa 10 GB per `gemma4:e4b`).

## Decisioni

- **Dove sta**: `scripts/setup_ai.sh --ollama` (Windows: `setup_ai.ps1 -Ollama`) scarica la versione
  fissata di Ollama (`v0.40.18`) dalle release ufficiali su GitHub, ne controlla lo SHA-256 con il
  `sha256sum.txt` della stessa release e la estrae in `<dati>/planet-one/ai/ollama/`. Poi scarica il
  modello, con un server temporaneo che si chiude a fine script. Niente servizio di sistema e niente
  `PATH`. Gli script viaggiano nei pacchetti di rilascio (`scripts/package_linux.sh`, `release.yml`).
- **Avvio** (`src/ai/server.rs`): con la IA accesa, all'avvio il gioco lancia `ollama serve` con:
  - `OLLAMA_HOST=127.0.0.1:<porta libera scelta dal sistema>`, mai la 11434 e mai fuori dal loopback;
  - `OLLAMA_MODELS` nella sua cartella, separata da un Ollama di sistema;
  - `OLLAMA_NO_CLOUD=1`, e `OLLAMA_ORIGINS` di sistema tolta;
  - il log in `ai/ollama/server.log`.
  Il client si collega solo quando il server risponde (stato «avvio di Ollama…»).
- **Chiusura**: spegnendo la IA dal menu o dalla shell, e all'uscita (`main`, dopo `App::run`). Prima
  `SIGTERM` all'intero gruppo di processi, poi `SIGKILL` dopo 3 s.
- **Crash del gioco**: il server non deve sopravvivere al gioco, e nemmeno i runner che tengono il modello
  nella scheda video.
  - **Linux e macOS**: Ollama sta in un gruppo di processi suo. Il gioco lancia sé stesso come guardiano
    (`planet-one --ollama-guard <gruppo>`), con lo stdin collegato al gioco. Quando il gioco muore, in
    qualunque modo, anche con `kill -9`, lo stdin si chiude e il guardiano chiude il gruppo. Il guardiano
    sta in un gruppo a parte, così il Ctrl+C del terminale non lo ferma prima del tempo. Su Linux c'è in più
    il segnale di morte del padre (`PR_SET_PDEATHSIG`) sul server.
  - **Windows**: un job object con `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Il sistema lo chiude con
    l'ultimo handle del gioco e porta con sé Ollama e i runner. Il processo parte senza finestra di console.
  - **Ultima rete (Linux)**: il server e i runner hanno `PLANET_ONE_OLLAMA_OWNER=<pid del gioco>`.
    All'avvio successivo il gioco chiude i processi del suo Ollama il cui padrone non è più un Planet One
    vivo. Un Ollama di sistema o quello dello script non hanno la variabile e non vengono toccati.
- **Compatibilità**: senza l'Ollama del gioco resta il comportamento di prima: si usa `ai_host`, un Ollama
  già avviato sul PC, sempre e solo in loopback. Serve anche alle prove con un server di sviluppo.
  `PLANET_ONE_OLLAMA=<eseguibile>` indica un altro programma da avviare (prove).

## Conseguenze

- A gioco chiuso non resta nulla in ascolto e la memoria si libera. Con la IA spenta, Ollama non parte.
- La prima risposta della partita aspetta anche l'avvio del server, di solito sotto il secondo, poi il
  caricamento del modello come prima.
- Chi ha già Ollama e il modello nel sistema li ritrova senza fare nulla, tramite `ai_host`. Per passare
  all'Ollama del gioco, il modello va scaricato di nuovo nella cartella del gioco.
- Nuova dipendenza solo per Windows: `windows-sys` (job object), già nell'albero delle dipendenze di Bevy.
- **Provato su Linux** con un finto `ollama` che inoltra a un server di prova:
  - avvio su una porta a caso e chat (copione `ia-locale`);
  - IA spenta e riaccesa;
  - uscita;
  - `kill -9` del gioco: nessun processo rimasto.
- **Da provare**: Windows e macOS, e l'archivio vero di Ollama v0.40.18.
