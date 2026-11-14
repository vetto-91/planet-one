# ADR-0049: Armatura, talenti attivi, compagni, il letto di casa e la magia insegnata

- Stato: accettato
- Data: 2026-11-14

## Contesto
Sesto passo del piano per il Prologo. Restavano scoperti diversi sistemi della trama (`Trama-Planet-One/02`,
sezione 2.4C, e `04`):
- l'abilità Armatura, senza nessuna armatura nel gioco;
- i talenti con nome;
- i compagni (Silvia, Dino);
- il letto della casa del giocatore;
- Agnese che insegna la magia, e l'incantesimo per calmare le creature.

## Decisione
- **Armature** (`ItemKind::LeatherArmor`, `MailArmor`, `PlateArmor`), dal fabbro:
  | Armatura | Danno subito | Peso | Prezzo |
  |---|---|---|---|
  | Cuoio | −15% | 5 kg | 80 |
  | Maglia | −30% | 11 kg | 180 |
  | Piastre | −45% | 18 kg, e −10% di velocità | 360 |

  Si indossano come le armi (slot `Equipment::armor`, salvato come `armatura`). Ogni colpo preso con
  l'armatura allena Armatura; il talento «Seconda Pelle di Ferro» toglie un altro 5% del danno per rango.
- **Talenti attivi** (`skills::ACTIVE_TALENTS`), dal grado 30 (Apprendista), elencati nel pannello C:
  - **Scatto di Ermes** (Atletica): `X` dà un balzo rapido in avanti per 20 di vigore;
  - **Disarmo di Grazia** (Lama): un nemico portato sotto il 30% della vita in mischia getta l'arma, lascia
    qualche moneta e se ne va (pietà +2). Il ladro di «Denaro Rubato» ha la sua storia e non si arrende.
- **Compagni** (`src/companions.rs`):
  - Silvia (dopo la Prova dell'Arco o il suo segreto) e Guardia Dino (dopo la sua missione o il suo segreto)
    offrono nel dialogo «Vieni con me».
  - Il compagno lascia la sua giornata (un ramo nuovo in `villagers::villager_routine`), ti segue a qualche
    passo, ti raggiunge se resti lontano e colpisce il nemico vivo più vicino: Silvia con l'arco fino a 14 m,
    Dino con la spada da vicino.
  - «Torna pure a casa» lo rimanda alla sua routine. Uno alla volta, solo sull'isola.
- **Il letto di casa**: nella casa finita, `R` vicino al letto fa dormire fino alle 7; vita, vigore e magia
  tornano pieni.
- **Magia insegnata**: Sfera di luce e Dardo si conoscono da sé; gli altri incantesimi chiedono anche
  l'insegnamento di Agnese, con la missione secondaria nuova **qs18 «L'Accordo»**
  (`sblocca: accordo`, letto da `magic::taught`). Nelle prove senza missioni valgono tutti.
- **Calma** (grado 20, costa 22): i nemici entro 14 m smettono di cercarti e non ti toccano per 30 secondi
  (componente `magic::Calmed`).

## Conseguenze
- Mancano gli altri talenti con nome della trama (apnea, controparata, borseggio dei nemici, borseggio di
  chiavi e lettere).
- L'armatura non si vede sul modello del personaggio.
- I compagni non si salvano (dopo un caricamento sono di nuovo a casa), non ricevono danno dai nemici e non
  viaggiano sul continente.
- La casa non si amplia ancora (camino che fuma, luce con interruttore).
