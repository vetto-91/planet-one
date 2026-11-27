# ADR-0006: Voce dei dialoghi IA con Piper locale

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.16.0 (fase 2)

## Contesto
Le risposte della IA (ADR 0005) devono essere lette ad alta voce, in italiano, rispettando il vincolo
"tutto solo in locale" (sviluppo e utente finale).

## Decisione
- TTS con **Piper** come programma esterno (`piper --model <voce>.onnx --output-raw`, testo su stdin, PCM su
  stdout): nessuna rete, nessun file temporaneo, nessun link con il codice del gioco (il fork mantenuto è GPL-3.0,
  il repo `rhasspy/piper` con i binari è MIT e archiviato).
- Il gioco crea un WAV in memoria (feature `wav` di Bevy) e lo riproduce come una voce normale (`VoiceClip`):
  volume voci e ducking della musica restano quelli esistenti. Thread worker con canali, una sintesi alla volta.
- Piper e voci vivono nella cartella dati dell'utente (`…/planet-one/ai/`), installati da
  `scripts/setup_ai.sh|ps1 --tts` con consenso esplicito; il gioco **non ridistribuisce voci** (le model card
  dei dataset italiani dicono solo "License: See URL": si mostra il link prima del download).
- Una voce per personaggio (`voce:` in `assets/ai/<nome>.txt`), override nel menu, casella "Voce IA".
- `scripts/gen_tts_voci.py` passa da edge-tts (servizio Microsoft online) a Piper locale; i clip esistenti non vengono
  toccati (solo i mancanti, `--force` per rigenerare).
- Aggiornamento (beta 067, ADR 0062): i clip fatti con edge-tts sono stati tutti rigenerati con Piper
  (`--force --solo viandante|ladro`); oggi ogni voce del gioco è locale e un personaggio ha la stessa voce nei dialoghi
  scritti e nella chat con la IA.

## Conseguenze
- Senza Piper o con la voce spenta il gioco mostra solo il testo, senza errori.
- Voci italiane disponibili: riccardo (maschile, qualità x_low), paola (femminile, medium), serena (medium/high).
- Latenza di sintesi ~1 s per battuta su CPU; da misurare con la voce reale.
