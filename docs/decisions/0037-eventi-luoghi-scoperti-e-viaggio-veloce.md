# ADR-0037: Eventi casuali, luoghi scoperti e viaggio veloce

- Stato: accettato
- Data: 2026-11-13

## Contesto
Dopo la storia principale (ADR 0034), i bivi (ADR 0035) e le prime secondarie (ADR 0036), sull'isola restavano:
le altre secondarie, i 12 eventi casuali della trama (`Trama-Planet-One/09`, sezione 4) e la regola generale dei
luoghi sulla mappa con il viaggio veloce (`Trama-Planet-One/10`, sezione 8.1).

## Decisione
- **Eventi** (`assets/quests/prologo/eventi.txt`, `tipo: evento`): incontri di una o due tappe che capitano
  passando in un luogo all'ora e col meteo giusti. Il motore guadagna `quando: meteo <tipo>` (il tempo che si
  vede, `WeatherState::shown`) e `quando: fatte <n>` (secondarie già compiute). Undici eventi: relitto della
  mareggiata, carretto nella nebbia, fuoco notturno sulla torre, pecora smarrita, serpente di luce, ronzio del
  lampione, disputa del mercato, sentiero delle farfalle, nave fantasma, ronda infreddolita, messaggio nella bottiglia.
- **Altre tre secondarie**: L'Armatura Lucida di Guardia Dino (qs09), La Croce delle Quattro Ronde (qs12),
  Il Cantiere di Fontevivo (qs15, dopo la grotta e con almeno quattro secondarie compiute). Sono 14 in tutto.
- **Luoghi scoperti** (`places::Known`): un luogo di `isola.txt` (e il paese) si scopre arrivando entro 16 m;
  da allora il suo nome compare sulla mappa. Si ricorda il punto in cui stava il giocatore: è lì che porta il
  **viaggio veloce** (un punto dove di sicuro si può stare). Si viaggia con un clic sul nome nella mappa, o
  dalla shell (`veloce <luogo>`); non con un nemico alle calcagna. Vale per isola e continente; lo stato sta
  nel salvataggio (`[luoghi]`).
- Shell e pilota: `tp` e `npc:` accettano nomi di due parole (`npc:guardia_gino:modo`).

## Conseguenze
- Gli eventi sono «incontri raccontati»: un testo, a volte un premio. Non creano personaggi o oggetti nel mondo
  (il carretto, la pecora, la bottiglia non si vedono). Manca il pedaggio del ponte (EC-05), che chiede nemici.
- La Croce delle Quattro Ronde non passa dal dialogo con le guardie di ronda: sul camminamento l'interazione
  con loro non scatta (difetto da indagare), quindi si sale sui quattro lati e si leggono le tacche.
- Dei quattro stati dei luoghi della trama ci sono «sconosciuto» e «scoperto»; mancano il segnalino «?» dato
  dagli abitanti e «esplorato parzialmente / completamente».
- Sulla mappa prima si vedevano tutti i nomi: ora solo quelli scoperti (una partita già iniziata li riscopre).
- Restano fuori dall'isola, perché chiedono sistemi nuovi: abilità che crescono con l'uso e livelli, karma e
  segreti degli abitanti, scasso e borseggio (qs16), costruzione della casa (qs17), prova dell'arco con i
  bersagli (qs14), creazione del personaggio dentro la scena del risveglio, bottino che si rigenera.
