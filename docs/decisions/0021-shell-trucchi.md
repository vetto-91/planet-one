# ADR-0021: Shell dei trucchi

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.32.0

## Contesto
Strumenti di debug, cambio dei modelli e coordinate erano sempre a portata di mano. Giocando normalmente non devono esserci trucchi.

## Decisione
- `src/shell.rs`: stato `AppState::Shell` aperto dal tasto **logico** `\`, così funziona su ogni layout (sulla tastiera italiana è a sinistra dell'1). Parser in funzione pura, testata.
- `Cheats` non viene salvata: a ogni avvio è tutto spento.
- Comandi per godmode, noclip, invisibilità, teletrasporto, passepartout, oggetti, ora, meteo, modelli IA e voci TTS, collider (F3) e piazzamenti (F4).
- F3 e F4 sono inattivi in gioco normale e tornano attivi con `debug on`; le coordinate nell'HUD si vedono con `coordinate`.
- La scelta di modello IA e voce esce dal menu Esc, dove restano gli interruttori e lo stato.
- Nessun comando può cambiare l'host della IA: resta solo loopback (ADR 0005).
- `PLANET_ONE_SHELL` esegue comandi all'avvio, per le prove automatiche.

## Conseguenze
- L'invisibilità toglie il modello, il danno da contatto e l'avvistamento del ladro; gli abitanti continuano a parlare normalmente.
- La shell si apre solo dal gioco in corso, non durante dialoghi e menu.
