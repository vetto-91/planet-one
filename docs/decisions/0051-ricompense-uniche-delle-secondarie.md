# ADR-0051: Le ricompense uniche delle missioni secondarie

- Stato: accettato
- Data: 2026-11-15

## Contesto
Il confronto fra la trama (`Trama-Planet-One/09`, sezione 3) e il gioco ha mostrato che le secondarie del
Prologo davano pozioni, mele o frecce dove la trama promette oggetti unici: la Lanterna di Vetro di Tiro,
il Mantello del Navigatore, i Guanti Isolanti, l'Arco Maestro di Silvia, lo Scudo dell'Orlo e la ricetta
delle Focacce del Navigante. È il primo passo del piano per completare isola, Prologo e traversata
(gli altri: scene mancanti delle secondarie, eventi nel mondo, traversata e approdo, rifiniture).

## Decisione
- **Oggetti nuovi** (`src/inventory.rs`), ognuno con un effetto piccolo ma vero:

| Oggetto | Missione | Effetto |
|---|---|---|
| Lanterna di Vetro di Tiro (`Relic::TyreLantern`) | QS-02, da Ettore | dalla barra si accende e si spegne una luce portata al fianco, il doppio di una lanterna da guardia (`equipment::carried_lantern`) |
| Mantello del Navigatore (`Relic::NavigatorCloak`) | QS-11, da Tonio | portato con sé vale 1,5 ranghi di Borseggio e di Furtività (`inventory::gear_perk`, `thievery::pocket_chance`, `tailing`) |
| Guanti Isolanti (`Relic::CopperGloves`) | QS-13, dal Viandante | 1,5 ranghi di Scassinare: la tacca dello scasso è più larga (`thievery::zone_width`) |
| Arco Maestro (`ItemKind::MasterBow`) | QS-14, da Silvia (con 20 frecce, la «faretra») | arco più forte: le frecce fanno il 40% di danno in più e volano più tese (`inventory::arrow_power`, `combat::Arrow::power`) |
| Scudo dell'Orlo (`ItemKind::RimShield`) | QS-03, se lo tieni | il miglior scudo dell'isola: passa il 30% del danno invece del 40%, pesa 2,5 kg invece di 3,5; in mano è di bronzo (`Equipment::rim`, `shield_factor`) |
| Ricetta delle Focacce | QS-01, da Marta | tre focacce subito, e da allora il forno di Marta vende la Focaccia del Navigante (`negozio: forno`, `shop::open_for`) |

- **Lo scudo appeso a Porta Nord** (l'altra scelta di QS-03) ora dà lo sconto permanente del 10% da
  Ferruccio, come vuole la trama (`sblocca scudo-porta-nord`, `shop::vendor_factor`).
- **QS-08**: la gemma d'ambra è di Agnese, che la usa per l'infuso; come premio in più, una Pozione del mulo
  (trama: «una Pozione del mulo speciale»).
- **Negozi e missioni**: gli sblocchi che toccano i negozi si copiano dalle missioni in `shop::PERKS`
  (`shop::sync_perks`), perché i prezzi si calcolano dove le missioni non ci sono (`karma::price`).
- **Salvataggio**: `scudo_orlo` e `lanterna` nella sezione dell'equipaggiamento.
- **Correzione**: i nomi degli oggetti nei file delle missioni si cercano come nei salvataggi
  (`ItemKind::from_name`: maiuscole, accenti, singolare e plurale). Prima «dai: freccia 12» non
  corrispondeva a nessun oggetto («Frecce»), e Silvia non dava le frecce della Prova dell'Arco. Un test
  controlla ora ogni `dai:`, `prendi:` e «dai …» delle scelte.
- **Prova**: `prove/zzzzzzzzzzzzzzzzzzz-ricompense.txt` (le ricompense ricevute parlando con chi le dà, il
  forno aperto, scudo e arco impugnati, la lanterna accesa di notte in grotta); lo stato del pilota ha
  `/personaggio/scudo_orlo` e `/personaggio/lanterna`.

## Conseguenze
- Mantello e Guanti non si indossano: valgono se li si porta nello zaino (come un amuleto senza slot).
- La lanterna non consuma nulla e non si spegne da sola.
- Le scene mancanti delle secondarie (QS-14 seconda metà, QS-12 campanile, QS-09 duello, QS-15 il paese al
  porto…) sono il passo successivo.
