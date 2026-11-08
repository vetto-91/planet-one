# ADR-0010: Strada del sud, faro, porticciolo e campane

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.20.0

## Contesto
A sud del villaggio servono una strada fino al mare, un faro e un piccolo porto vicini tra loro, e le campane della chiesa.

## Decisione
- Posizioni scelte dalla forma dell'isola (costa a z ≈ 97 per x tra −10 e 10): faro sulla scogliera a ovest, molo a est, strada
  dal cancello sud. Tutto generato da `scripts/gen_porto.py` come piazzamenti (modificabili in F4) più un blocco di strade in `villaggio.txt`.
- `molo` e `barca` hanno quota fissa a livello del mare; il piano del molo è `Ground` e interrompe la barriera di riva (impronta letta dai
  piazzamenti in `terrain::build_barrier`), così ci si cammina sopra senza poter cadere in acqua.
- Faro: testa girevole con `SpotLight` (accesa al tramonto) e bagliore fisso; Ettore è una guardia sempre al posto.
- Campane: suono sintetizzato in locale (parziali non armonici), rintocchi alle ore piene 7–21 con volume secondo la distanza.

## Conseguenze
- La barriera di riva si ricalcola solo all'avvio: spostare il molo in F4 richiede il riavvio per aprire il varco.
- Un test (`buildings::tests`) verifica che ogni tipo con modello sia riconosciuto come edificio: in v0.0.19.0 bancarelle, bottega dell'arciere,
  pagliai e spaventapasseri non comparivano per una lista incompleta.
