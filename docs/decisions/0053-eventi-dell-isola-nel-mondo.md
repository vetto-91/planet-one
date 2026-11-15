# ADR-0053: Gli eventi dell'isola nel mondo

- Stato: accettato
- Data: 2026-11-15

## Contesto
Terzo passo del piano per completare isola e Prologo (dopo le ricompense, ADR 0051, e le scene delle
secondarie, ADR 0052). Otto degli eventi casuali (ADR 0037, 0046) erano solo raccontati: si arrivava nel
posto all'ora giusta e compariva un pensiero, senza niente nel mondo. Erano il relitto, il fuoco sulla
torre, il serpente di luce, il ronzio del lampione, le farfalle, la nave fantasma, la ronda al freddo e
il messaggio nella bottiglia.

## Decisione
- **`src/island_events.rs`**: ogni evento ha una «cosa» nel mondo (`EventSpec`: posto, ore, meteo, prompt,
  sblocco). Compare solo mentre l'evento è aperto (prima tappa, ore e meteo giusti) e a meno di 90 m dal
  giocatore; altrimenti sparisce.
- **Da toccare con `E`**, quando si è a portata:
  - EC-01, **la cassa del relitto** sulla spiaggia all'alba, con le assi spezzate attorno;
  - EC-07, **il lampione azzurro** fuori da Porta Sud sotto la pioggia: il vetro pulsa e illumina d'azzurro;
  - EC-09, **il tronco cavo** al margine del bosco, con tre farfalle arancioni che gli girano attorno;
  - EC-11, **la mela alla ronda**: nella notte di neve la guardia di Porta Nord batte i denti (fumetto con
    voce, `freddo-ronda` in `battute.txt`); con una mela nello zaino la lasci sul parapetto;
  - EC-12, **la bottiglia verde** sulla riva all'alba, che ora lascia anche il **Messaggio nella
    bottiglia** (`Relic::BottleMessage`).
- **Da guardare per intero**, restando vicino:
  - EC-03, **la scintilla azzurra** in cima alla torre: tre lampi e una pausa, con una luce che illumina
    l'altura (7 secondi);
  - EC-06, **il serpente di luce**: sei strisce dorate sui gradoni della rampa della grotta si accendono una
    dopo l'altra dall'alto (9 secondi). La tappa ora chiede di stare sulla rampa (prima chiedeva la sala);
  - EC-10, **la nave fantasma**: scafo, albero, vela e remi pallidi e luminosi, con due luci a bordo,
    passano al largo da ovest a est in 40 secondi (24 secondi da guardare).
- **Il mondo dà uno sblocco** (`karma::flag`), e ogni tappa lo chiede (`quando: sbloccato …`) insieme a
  luogo, ora e meteo, come prima. Un test controlla che ogni sblocco sia chiesto dalla sua missione.
- **Prova**: `prove/zzzzzzzzzzzzzzzzzzzzzz-eventi-2.txt`. `zz-ultime.txt` chiude questi eventi con la shell.

## Aggiornamento: ciò che restava (2026-11-15)
- **Mobili veri**: la cassa del relitto (ora sulla sabbia, a 38; 90,5) e il tronco cavo sono mobili da
  aprire (`containers::bundle`, chiavi `evento:ec01` ed `evento:ec09`). Dentro c'è quel che racconta la
  trama: monete bagnate e una boccetta nella cassa, un sacchetto con monete e frecce nel tronco
  (`containers::starting_items`). L'evento si completa quando il mobile è stato aperto; i premi della
  missione sono passati nel mobile.
- **Guardare davvero**: per gli eventi da guardare conta solo il tempo in cui la camera è girata verso la
  cosa (entro una cinquantina di gradi, `island_events::looks_at`). La nave fantasma basta guardarla 10
  secondi, e ripassa ogni 50 secondi per chi l'ha persa.
- **Il ronzio del lampione**: un suono sintetizzato in locale (`sfx::synth_hum`, 100 Hz con le armoniche e
  un battito lento), in loop e posizionale dal vetro (`audio::spatial_loop`).
- **Eventi che si ripetono**: la chiave `ripeti: <giorni>` in `eventi.txt`. Compiuto, l'evento segna il
  giorno (`Missions::done_on`, salvato come `ripetuta:<id> = <giorno>`); passati i giorni torna da capo e le
  risposte di allora si dimenticano, con i loro sblocchi. Il pedaggio del ponte torna ogni 7 giorni:
  i banditi si ripresentano al tramonto e si sceglie di nuovo.

## Conseguenze
- ~~Il pedaggio succede una volta sola; il lampione non ronza; cassa e tronco non sono mobili veri; si
  conta il tempo passato vicino.~~ **Risolto**, vedi «Aggiornamento».
- Gli altri eventi non si ripetono: capitano una volta, come vuole la trama.
