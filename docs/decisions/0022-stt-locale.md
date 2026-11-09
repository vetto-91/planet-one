# ADR-0022: Riconoscimento vocale locale

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.33.0

## Contesto
Nella chat con gli abitanti si doveva poter parlare, rispettando il vincolo "tutto solo in locale" (ADR 0005, 0006).

## Decisione
- **whisper.cpp come programma esterno** (`whisper-cli -l it -nt -np`), come Piper: nessuna rete, nessun link con il gioco. Va installato con `scripts/setup_ai.sh --stt` (Windows `setup_ai.ps1 -Stt`), che compila il sorgente v1.9.5 (MIT) e scarica `ggml-base.bin` (148 MB, hash verificato) con consenso esplicito.
- Su Linux x86 si compila senza `-march=native`: gli assembler non aggiornati non conoscono le istruzioni dei processori più nuovi.
- Microfono con `cpal`, su una thread che possiede il flusso. È aperto solo mentre si tiene premuto F2 o il pulsante «Parla», per al massimo 30 s.
- L'audio passa da un WAV temporaneo a 16 kHz nella cartella dati dell'utente, cancellato subito dopo la trascrizione qualunque sia l'esito.
- Il testo finisce nella casella della chat, da rileggere: non viene inviato da solo.
- Il microfono è spento finché non lo si attiva dal menu Esc o con `stt on`. `stt modello` sceglie tra i file `ggml-*.bin` installati.

## Conseguenze
- Senza whisper o col microfono spento il gioco si comporta come prima.
- `PLANET_ONE_STT_MIC=<wav>` sostituisce il microfono con un file, per le prove automatiche.
- La cattura reale dal microfono non è verificabile da remoto; Windows e macOS non sono provabili da Linux.
