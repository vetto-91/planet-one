# ADR-0025: Menu principale e salvataggi

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.36.0

## Contesto
Il gioco entrava subito in partita e alla chiusura perdeva tutto.

## Decisione
- Nuovo stato iniziale `AppState::MainMenu`; `AppState::Loading` attende che terreno ed edifici siano pronti dopo `scene::reset_scene`, poi applica la richiesta (`LoadRequest`).
- Ogni personaggio ha una cartella in `<dati utente>/planet-one/saves/<nome>/`; ogni salvataggio è un file di testo a sezioni, leggibile e modificabile. Scrittura su file temporaneo e rinomina.
- `SaveData::to_text` e `SaveData::parse` sono pure. Righe sconosciute o errate vengono ignorate con un avviso; una sezione tolta lascia quella parte del mondo come appena creata.
- Automatici ogni 5 minuti e all'uscita (`automatico-1..3.txt`), rapido su F5 (`rapido.txt`, F9 lo ricarica), manuali con data nel nome. «Continua» prende il più recente in assoluto.
- Salvati: personaggio, data e ora, neve, meteo forzato, posizione e vista, vita, inventario, barra, equipaggiamento, effetti, quest, porte e cancelli (`PropId`), mobili aperti, oggetti a terra, monete della quest, nemici, ricomparse, abitanti e Viandante.
- Oggetti a terra: «Getta» dall'inventario; quelli gettati si riprendono con E, gli altri restano a raccolta automatica.
- Impostazioni a schede condivise tra menu principale e pausa; sensibilità, FPS a schermo e schermo intero diventano persistenti.

## Conseguenze
- Una sequenza in corso del Viandante riparte; chi è a metà di una scala viene riportato alla fine del percorso.
- I piazzamenti F4 restano del mondo, non del salvataggio; i trucchi della shell non si salvano.
- La data nei file è in UTC.
- Con le variabili `PLANET_ONE_*` di prova si salta il menu.
