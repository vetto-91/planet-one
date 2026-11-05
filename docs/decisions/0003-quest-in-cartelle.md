# ADR-0003: Quest in cartelle con dialoghi in file di testo

- Data: 2026-11-05
- Stato: accettato
- Versione: v0.0.5.x (non ancora rilasciato)

## Contesto
La quest "Denaro Rubato" aveva logica e testi mescolati in `src/quest.rs`. Servono
più quest e dialoghi modificabili a mano senza toccare il codice.

## Decisione
Ogni quest ha una cartella `src/quests/<quest>/` (logica) e un file
`assets/quests/<quest>/dialoghi.txt` (testi). Formato: sezioni `[nome]`, righe di testo,
`? scelta`, `# commento`, segnaposto `{hint}`/`{n}`/`{total}`/`{reward}`. Parser interno,
nessuna dipendenza. Il file viene letto all'avvio e al reset scena; una copia incorporata
con `include_str!` fa da fallback per file o sezioni mancanti. Il registro delle quest
(`quests::journal_entries`, `quests::npc_dialog`) alimenta diario (J), pannello TAB e dialoghi.

## Conseguenze
- Modificare un testo non richiede ricompilare (basta Reset scena).
- Aggiungere una quest: nuova cartella, un `mod` e una riga nel registro.
- Nessuna validazione dei testi oltre al log di sezioni mancanti.
