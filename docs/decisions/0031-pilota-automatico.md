# ADR-0031: Pilota automatico e segnalazioni per il beta test

- Stato: accettato
- Data: 2026-11-10

## Contesto
Per trovare i bug di gioco servono partite ripetibili. Finora si giocava con tastiera, mouse e screenshot
(anche pilotati da xdotool): lento, cieco (non si sa cosa c'è attorno) e non ripetibile.

## Decisione
- Un **pilota automatico** dentro il gioco (`src/autopilot.rs`) esegue un copione di testo (`PLANET_ONE_SCRIPT`).
  Simula i tasti scrivendo in `ButtonInput` dopo i sistemi di input (`PreUpdate`), guida la camera
  (`OrbitCamera`) e cammina con la fisica vera, quindi prova gli stessi sistemi del giocatore.
- Lo **stato del gioco** si fotografa in JSON (sistema esclusivo con accesso al `World`); le `verifica`
  lavorano su quel JSON, così un nuovo controllo non richiede nuovo codice.
- I comandi di movimento e mira (`vai`, `guarda`, `dove`) stanno nella **shell**: valgono anche per chi gioca.
- Ogni partita di prova può usare una cartella dati separata (`PLANET_ONE_DATA_DIR`).
- **F12** salva una segnalazione (schermo, stato, log, nota); il log è copiato in memoria da uno scrittore
  che affianca stdout (`bugreport::TeeWriter`).
- `scripts/beta_check.sh` esegue tutti i copioni e scrive un rapporto; il gioco esce con codice 1 se una verifica fallisce.

## Conseguenze
- Ogni bug di gioco trovato diventa un copione in `prove/` che fallisce finché non è corretto.
- I copioni dipendono dal mondo (posizioni, routine degli abitanti): vanno aggiornati se il mondo cambia.
- `vai` cammina in linea retta: si ferma e lo segnala davanti a un ostacolo (utile anche per trovare passaggi bloccati).
