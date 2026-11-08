# ADR-0005: Dialoghi con IA solo locale

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.15.0 (fase 1)

## Contesto
Si vogliono espandere i dialoghi con una IA, in italiano (poi con TTS e STT), senza toccare le quest
e i dialoghi scritti, con un interruttore per i PC poco potenti.

## Decisione
- **Vincolo assoluto: tutto solo in locale**, in sviluppo e per l'utente finale. Nessun cloud, nessuna API
  remota, nessuna telemetria. Il gioco accetta solo host di loopback (`ai::ollama::local_base`), rifiuta
  `https` e non ha nessun crate TLS nell'albero delle dipendenze (`ureq` senza feature TLS).
- IA testuale: Ollama (servizio locale) con `gemma4:e4b` configurabile (alternativa `gemma4:e2b`); client
  `ureq` + `serde_json` in una thread worker (il gioco non si blocca); risposta JSON strutturata
  (`risposta` + tre `opzioni`).
- Precedenza: il dialogo scritto finisce come prima; l'opzione "Parliamo d'altro…" appare solo senza scelte
  di quest in sospeso. Stato `AppState::Chat` (gameplay sospeso, cursore libero). La IA non vede né cambia lo
  stato di gioco; legge solo ora del giorno e un riassunto della missione.
- Persona dei personaggi in `assets/ai/<nome>.txt` (modificabile, con fallback incorporato).
- Modelli e programmi non fanno parte del gioco: si installano con `scripts/setup_ai.sh|ps1` (download una
  tantum con consenso esplicito, poi offline). Fasi successive: Piper (TTS) e whisper.cpp (STT), sempre
  processi locali.

## Conseguenze
- Con IA spenta o Ollama assente il gioco è identico a prima (nessuna opzione, nessun errore).
- Con la IA in generazione la GPU è condivisa col gioco: il framerate può calare (da misurare).
- `ai_host` non locale ⇒ stato "host non locale: rifiutato".
