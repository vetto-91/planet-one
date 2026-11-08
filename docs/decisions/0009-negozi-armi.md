# ADR-0009: Negozi, armi e mercato

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.19.0

## Contesto
Il villaggio ha un mercato e due botteghe oltre al Viandante, con pozioni, spade, ascia, scudo, arco e frecce da comprare.

## Decisione
- **Economia**: monete (30 iniziali, drop dai nemici comuni). Assortimenti per chiave in `shop.rs` (`stock`); pannello egui
  (`AppState::Shop`), acquisto applicato da `shop_buy`. Dopo il dialogo scritto l'offerta finale è dinamica:
  comprare (se venditore) / chiacchierare (IA pronta) / arrivederci (`Offer`).
- **Equipaggiamento**: `Equipment { weapon, shield }`, armi impugnate dalla barra (tasti 1–0); statistiche in
  `inventory::weapon_stats`; scudo ×0,4 sui danni; arco con frecce (`combat::Arrow`) che volano verso il mirino, cadono
  leggermente e si fermano su terreno ed edifici. Modelli semplici attaccati al player (`equipment.rs`) con oscillazione.
- **Mercato e botteghe**: nuovi tipi di piazzamento (`arcere`, `bancarella-*`, `campo`, `orto`, `pagliaio`, `spaventapasseri`);
  i campi sono una mesh unica per piazzamento (colori di vertice). I venditori sono abitanti con `negozio:` in `abitanti.txt`.

## Conseguenze
- Gli oggetti curativi si consumano, armi e scudo no; niente vendita al negozio né durata delle armi (per ora).
- I prezzi stanno nel codice (`shop::stock`).
