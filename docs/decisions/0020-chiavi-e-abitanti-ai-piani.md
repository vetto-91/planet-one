# ADR-0020: Abitanti ai piani, chiavi e serrature

- Data: 2026-11-09
- Stato: accettato (estende ADR 0019)
- Versione: v0.0.32.0

## Contesto
Gli abitanti comparivano a letto senza salire le scale, alcuni stavano fermi davanti alle porte e ogni porta si apriva con E.

## Decisione
- Ogni edificio espone il percorso a piedi per ogni piano (`Layout.routes`, `Interior::route_to_bed`) e ogni letto il punto da cui ci si sdraia. `Villager.indoor` è una coda di punti 3D che ha la precedenza sulla strada: sera strada → porta → rampe → letto, mattina il contrario.
- I punti seguono le rampe in linea retta, gli abitanti non usano la fisica: possono sfiorare un mobile. Le animazioni arrivano dalla velocità misurata.
- `Lock` sulla maniglia: locanda e chiesa `Public`; forno, fabbro e arciere `WorkHours` con gli orari del proprietario in `abitanti.txt`; case e caserma `Private`. Da dentro si esce sempre.
- `NpcInventory`: ogni abitante porta la chiave di casa (`casa:`) e quelle in più (`chiave:`). Una porta si apre solo per chi ne ha la chiave, poi si richiude.
- Il giocatore non ottiene chiavi giocando: il `Passepartout` esiste solo dalla shell (ADR 0021). Furto e consegna delle chiavi restano possibili in futuro.
- I posti di lavoro stanno ad almeno 3 m dalle porte; all'avvio un avviso `[VILLAGERS]` segnala chi finisce davanti a un ingresso.

## Conseguenze
- La casetta del Viandante, il faro e i cancelli non hanno serrature.
- Una bottega si richiude all'orario di chiusura; chi è dentro può sempre uscire.
