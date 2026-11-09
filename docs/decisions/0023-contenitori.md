# ADR-0023: Contenitori e faro visitabile

- Data: 2026-11-09
- Stato: accettato (estende ADR 0019)
- Versione: v0.0.34.0

## Contesto
Casse, bauli, botti, armadi e scaffali erano solo scenografia. Il faro era l'unico edificio fuori dalle mura ancora pieno: caserma e casa del porto avevano già gli interni.

## Decisione
- `src/containers.rs`: ogni mobile è registrato dall'edificio (`Interior.containers`) o dalla casetta, con una maniglia `PropKind::Container` e la finestra `AppState::Container`.
- Capienza da 5 a 10 volte il riferimento `PLAYER_CARRY_KG` (30 kg): scaffale ×5, cassa ×6, botte ×7, baule ×8, armadio ×10. Il giocatore non ha un limite di peso: la costante serve solo da base.
- `Capacity::Unlimited` è già gestito: serve per la casa di proprietà e il deposito del giocatore, quando esisteranno, che avranno contenuto infinito.
- Contenuto iniziale a tema, generato alla prima apertura dalla chiave del mobile (`id piazzamento:punto:indice`) e sempre uguale. La chiave non cambia se l'edificio viene ricostruito con F4.
- Come l'inventario del giocatore, i contenuti non vengono salvati su disco.
- Nessuna meccanica di furto: i mobili nelle case private si raggiungono solo col passepartout.
- Faro: basamento pieno, alloggio al piano terra, rampa al primo piano con ripostiglio, scala a chiocciola nel pozzo centrale, lanterna cava e balconata con ringhiera solida. Chiuso a chiave; Ettore ha la chiave e resta di guardia fuori.

## Conseguenze
- Tutti i mobili futuri creati con gli helper di `interiors.rs` diventano contenitori da soli.
- La scala del faro lascia un vuoto al centro: si può cadere fino al primo piano, senza danni.
