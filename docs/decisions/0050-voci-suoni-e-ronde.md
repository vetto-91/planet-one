# ADR-0050: Le voci delle battute del Prologo, i piccoli suoni e le ronde che si fermano

- Stato: accettato
- Data: 2026-11-14

## Contesto
Ultimo passo del piano per il Prologo. Le battute delle missioni e le confidenze non avevano voce (solo i
saluti degli abitanti). Incantesimi, anelli dell'arco e serrature non facevano rumore. E alle guardie di
ronda sul camminamento non si riusciva a parlare (PIANO-BETA, 13/11).

## Decisione
- **Voci** generate in locale con Piper (`scripts/gen_voci_prologo.py`, nessuna rete):
  - per ogni battuta («> testo») di missioni, secondarie ed eventi, detta da chi è nella condizione
    `quando: parla` della tappa, e per ogni confidenza di `segreti.txt`, una clip
    `assets/audio/voci/prologo/<abitante>-<hash>.ogg` con la voce della sua persona (`assets/ai/<nome>.txt`);
  - l'hash è FNV-1a a 64 bit del testo così com'è nel file: il gioco lo ricalcola (`voice::line_clip`) e
    trova la clip senza una mappa a parte. Una battuta cambiata ha semplicemente una clip nuova;
  - nella voce `{nome}` diventa «viaggiatore»: il nome del giocatore non si può registrare prima;
  - 221 battute, circa 12 MB; si generano in un paio di minuti (solo quelle mancanti; le vecchie si
    tolgono). Due test controllano che ogni battuta e ogni confidenza abbia la sua clip.
- **Suoni** sintetizzati (`src/sfx.rs`, sullo stesso principio di `ambience.rs`, volume dell'ambiente):
  - un accordo che sale per l'incantesimo;
  - un «ting» di ferro per l'anello centrato;
  - un clic per il perno;
  - due scatti e un tonfo per la serratura che cede.
- **Ronde**: la guardia di ronda continuava a camminare (1,3 m/s) e in un attimo usciva dai 3 m entro cui
  si può parlare. Ora si ferma e si volta verso il giocatore quando le è entro 2,8 m
  (`villagers::PATROL_STOP`). Il copione `zzzzzzzzzzzzz-ronde` lo controlla.
- Documenti superati aggiornati: ADR 0036 (le sei secondarie mancanti ora ci sono) e l'intestazione di
  `secondarie.txt` (sintassi completa).

## Conseguenze
- Le battute della quest «Denaro Rubato» hanno le loro voci da prima. ~~Quelle scritte nel codice (la fama
  nei saluti, il pedaggio) non hanno voce.~~ **Risolto**, vedi «Aggiornamento: battute del gioco e lessico».
- Le voci sono quelle di Piper: comprensibili ma piatte. ~~Qualche nome proprio è pronunciato male.~~
  **In parte risolto** col lessico di pronuncia, vedi sotto.
- La Croce delle Quattro Ronde (qs12) resta fatta di luoghi; ora potrebbe tornare a passare dal dialogo con le
  ronde.

## Aggiornamento: battute del gioco e lessico di pronuncia (2026-11-15)
- **Le battute scritte nel codice stanno in un file**: `assets/world/battute.txt` (sezioni per chi
  parla, righe «chiave: battuta», letto alla compilazione; `quests::bark_line`).
  - La sezione `[tutti]` vale per ogni abitante: la fama nel primo saluto, il rifiuto di chi non ti
    parla, il grazie per un regalo, il «giù le mani» del borseggio, la multa della guardia. Lo script ne
    fa una clip con la voce di ciascun abitante di `abitanti.txt`.
  - Poi le battute di un personaggio: Ettore, Marta e Agnese pedinate, la lite di Tonio e Nina
    (`lite-<n>`, in ordine, `quests::numbered_lines`), il bandito del ponte e Lino il carrettiere
    (`dialogo`, `quests::talker_lines`).
- **Ogni fumetto cerca la sua voce** (`Barks::say` → `voice::request_bark` → `voice::bark_voice`): se
  la clip della battuta esiste, si sente dalla bocca di chi parla. Non copre la voce di un dialogo aperto.
  Compagni ed Ettore non chiamano più la voce a mano.
- **Dialoghi scritti nel gioco**: le righe del rifiuto, della fama e dei personaggi di passaggio cercano la
  clip della riga (`voice::dialog_clips`); se manca, restano i saluti generati per l'abitante.
- **Lessico di pronuncia** (`assets/audio/voci/pronuncia.txt`, «parola => come leggerla», solo parole
  intere): vale per le voci generate (`scripts/gen_tts_voci.py::respell`, usato da tutti gli script) e
  per la voce dal vivo dei dialoghi con l'IA (`ai::tts::respell`).
  - Le grafie si sono scelte facendo leggere a Piper le varianti e trascrivendole con whisper.cpp locale:
    Quipu → Kìpu (si sentiva «qui più»), Ermes → Èrmes (si sentiva «Ernest»), Fontevivo → Fontevìvo
    (si sentiva «Funte vivo»), Calliope → Callìope.
  - Per Intihuatana, Tiro-Scheria, Cenote, Osvaldo e Viandante la grafia del gioco suona già meglio delle
    varianti; Ma'at resta difficile per Piper con ogni grafia provata.
  - Il lessico non cambia il nome delle clip (si calcola sul testo del gioco): dopo una modifica si
    rifanno a mano le clip coinvolte (cancellarle e rilanciare gli script, o `--force`).
- **Voci**: 413 battute del Prologo (160 nuove, quasi tutte le battute di `[tutti]`) e quattro saluti
  degli abitanti rifatti per «Fontevivo».
- **Limiti**: la multa della guardia non dice più la cifra a voce (la dice l'avviso); «Zzz…» resta senza
  voce.
