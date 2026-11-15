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

## Conseguenze
- Il pedaggio del ponte (EC-05) succede ancora una volta sola.
- Il lampione azzurro non ha un suono suo: il ronzio è nel pensiero.
- La cassa del relitto e il tronco non sono mobili veri: il premio arriva dalla missione, come prima.
- Gli eventi da guardare contano il tempo passato vicino, non se li si guarda davvero.
