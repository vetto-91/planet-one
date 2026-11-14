# ADR-0044: Oggetti di trama e scelte nelle missioni

- Stato: accettato
- Data: 2026-11-14

## Contesto
Primo passo del piano per completare l'isola e il Prologo. Gli oggetti della trama (tavoletta, Moneta Madre,
campanaccio, bussola, piastrine…) esistevano solo come testo, e le missioni secondarie non avevano le scelte
che la trama prevede (`Trama-Planet-One/09`, sezione 3): lo scudo da tenere o donare, la taglia o il rito per i
banditi, l'infuso di Agnese, la scatola di Beppe.

## Decisione
- **Oggetti di trama** (`inventory::Relic`, dentro `ItemKind::Relic`): quattordici, nella categoria nuova
  «Ricordi». Pesano quasi nulla e non vanno nella barra, tranne la Focaccia del Navigante (cura 30) e l'Amuleto
  Piuma (si indossa: +5 kg). Icone disegnate da `scripts/make_item_icons.py`.
- **Scelte**: una tappa con `scegli: testo => effetto; effetto` mostra le risposte alla fine del dialogo
  (lo stesso meccanismo dei bivi di «Denaro Rubato»). La risposta data aggiunge i suoi effetti a quelli della
  tappa e resta nel salvataggio (`scelta:<missione>.<tappa>` in `[missioni]`).
- **Effetti** (nelle scelte, o come righe della tappa): `dai`, `prendi`, `monete`, `karma <n> <motivo>`,
  `fiducia <abitante> <n>`, `stat <statistica> <n>`, `sblocca <cosa>`, `fine` (la missione si chiude lì: le
  tappe saltate non contano e non sbloccano nulla, e non vale come compiuta per il karma).
- **Condizioni nuove**: `quando: sbloccato <cosa>`; i segreti confidati diventano contatori
  (`quando: conta segreto:<abitante> 1`); le serrature si contano anche per edificio
  (`conta serrature:caserma 3`).
- `inventory::grant` dà un oggetto da un sistema che non ha l'inventario (la tavoletta al campo del ladro).
- **Missioni aggiornate**:
  - qs03: lo scudo si tiene o si appende sopra Porta Nord (karma, fiducia di Ferruccio e Aldo);
  - qs04: piastrine vere; le piastrine si portano al rito di Don Anselmo, e dopo il rito i banditi non
    tornano più (`sblocca: rito-banditi`, letto da `places::keep_bandits`), oppure si vendono ad Aldo per la
    taglia (`fine`);
  - qs05: campanaccio e Amuleto Piuma; qs06: Mappa delle Correnti; qs07: tre risposte per Beppe (con la chiave
    di rame); qs08: +1 a Saggezza o Intelligenza; qs10: la bussola si rende a Rosa o si porta a nord;
  - qs16: valgono i tre bauli della caserma.
- «Denaro Rubato»: la tavoletta e la Moneta Madre sono oggetti veri.

## Conseguenze
- Gli oggetti di trama si possono gettare come gli altri; nessuno li vende né li compra.
- Le scelte sono solo alla fine di un dialogo; il Viandante non ne offre (il suo dialogo è quello della quest).
- I segreti degli abitanti non danno ancora oggetti: arriva nel passo successivo (`sblocca:` in `segreti.txt`).
- Il copione `zz-secondarie` sceglie: scudo appeso, bussola portata a nord, rito per i banditi.
