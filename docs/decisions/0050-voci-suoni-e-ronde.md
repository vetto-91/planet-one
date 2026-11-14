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
- Le battute della quest «Denaro Rubato» e quelle scritte nel codice (la fama nei saluti, il pedaggio,
  i compagni) restano come prima: le prime hanno le loro voci, le altre no.
- Le voci sono quelle di Piper: comprensibili ma piatte, e qualche nome proprio è pronunciato male.
- La Croce delle Quattro Ronde (qs12) resta fatta di luoghi; ora potrebbe tornare a passare dal dialogo con le
  ronde.
