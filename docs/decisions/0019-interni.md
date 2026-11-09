# ADR-0019: Interni degli edifici

- Data: 2026-11-09
- Stato: accettato
- Versione: v0.0.31.0

## Contesto
Gli edifici erano volumi pieni con un collider unico. Case, locanda, caserma, fabbro, forno, arciere e chiesa devono essere visitabili, su più piani e arredati; di notte gli abitanti tornano a casa e dormono a letto.

## Decisione
- `Building.interior` raccoglie pareti (`BlocksCamera`), pavimenti e gradini (`Ground`), mobili (fermano solo il giocatore), letti, luci e porta. Gli oggetti pieni (faro, molo, bancarelle) restano come prima.
- Piani da 2,5 m e impronta invariata: casa piccola e media 2 piani, casa grande, locanda e caserma 3, fabbro, forno, arciere 2, più il **sottotetto** sotto un tetto ripido (colmo a 0,8 volte la profondità). Il pavimento sta 42 cm sopra la base, su uno zoccolo di pietra.
- Scale (`interiors.rs`): rampe piene, alzata ≤ 0,28 m, pedata 0,27 m. Le rampe tra i piani stanno lungo la parete di fondo alternando il verso; l'ultima sale lungo la parete destra e arriva dove nel sottotetto ci si sta in piedi. Il campanile ha una scala a chiocciola quadrata con la cella delle campane; le aperture sono vere, ma il collider resta pieno.
- Mobili generati nel codice con le texture PBR esistenti, nessun download: arredi diversi per tipo di edificio; i corridoi porta-scala restano liberi.
- Porte vere (`Hinge`, `PropState`, `Interactable` della casetta) che si aprono verso l'interno.
- Terreno: sotto ogni edificio visitabile uno spiazzo piano (`flatten_pads`) con raccordo di 3,5 m, altrimenti il terreno sbucherebbe nei pavimenti. Si calcola una volta alla costruzione del terreno: spostando un edificio con F4 lo spiazzo segue al riavvio.
- Luci interne (una per piano) accese solo nell'edificio in cui si trova il giocatore, senza ombre: costo minimo; opzione «Luci negli edifici» in Impostazioni grafiche.
- Abitanti: nuova chiave `casa: tipo n letto` in `abitanti.txt`. Di notte camminano fino alla porta, poi compaiono a letto; le guardie fuori turno dormono in caserma e lasciano lancia e lanterna. Non salgono le scale: da fuori si vede l'arrivo alla porta, da dentro il risultato.

## Conseguenze
- Ci sono circa 1000 collider statici in più nel villaggio; nelle prove gli FPS restano tra 50 e 95, senza un confronto preciso con la versione precedente.
- Le case sono più alte (fino a circa 12,8 m al colmo).
- Il faro e la casetta del Viandante non cambiano.
