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
- ~~I compagni non si salvano e non ricevono danno dai nemici~~: risolto, vedi «Aggiornamento» sotto. Non
  viaggiano ancora sul continente: arriverà con l'Atto I.
- La casa non si amplia ancora (camino che fuma, luce con interruttore).

## Aggiornamento (2026-11-14): ferite, ritirata, salvataggio e voci dei compagni
- **Vita** (`Companion.hp`, 100): i nemici vivi entro la portata del contatto feriscono il compagno come
  il giocatore, e le loro frecce lo colpiscono lungo tutto il tratto percorso in un fotogramma
  (`near_segment`). Dino porta la cotta della guardia e prende il 70% del danno.
- **Bersaglio delle frecce** (`combat::enemy_ai`): un nemico tira al compagno se gli è più vicino del
  giocatore, e comunque una freccia su due quando è in vista (`EnemyMind.shots`). Senza questa regola il
  compagno, che sta qualche passo dietro, non veniva mai colpito.
- **Allarme e ritirata**: sotto 35 il compagno lo dice (una volta, finché non si riprende); a zero lascia
  il gruppo, torna alla sua giornata ed è ferito fino al giorno dopo (`CompanionState.wounded`, con il
  giorno). Intanto nel dialogo non offre «Vieni con me». Lontano dai colpi per 8 secondi si riprende di
  3 punti al secondo.
- **HUD**: sopra la barra della magia, una barra arancio con il nome e la vita del compagno.
- **Salvataggio**: sezione `[compagni]`, con `segue = <nome>|<vita>` e `ferito = <nome>|<giorno>`. Al
  caricamento `restore_and_heal` toglie chi seguiva prima e rimette il compagno salvato appena l'abitante
  c'è (solo sull'isola). Viaggiando verso il continente il compagno resta a casa.
- **Battute con voce**: `assets/world/compagni.txt` (chiavi `segui`, `casa`, `allarme`, `ferita`), dette
  con un fumetto e con la voce generata in locale da `scripts/gen_voci_prologo.py`. Un test controlla che
  ogni battuta abbia la sua clip.
- **Prove**: shell `ferisci <danno>`; nello stato del pilota `/personaggio/compagno_vita` e
  `/personaggio/feriti`; il copione `zzzzzzzzzzzz-sistemi` prova colpo, salvataggio e ricaricamento,
  allarme, ritirata e guarigione a mezzanotte.
- Restano fuori: il compagno non schiva e non si ripara; i nemici in mischia colpiscono solo chi è a
  portata di contatto.
