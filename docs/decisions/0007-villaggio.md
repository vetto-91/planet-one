# ADR-0007: Villaggio murato con edifici procedurali

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.17.0

## Contesto
Serve riempire l'isola: un villaggio murato (perimetro disegnato dall'utente in F4) con due cancelli, strada
selciata, stradine, case di vario tipo, chiesa, cartelli e nome sulla mappa.

## Decisione
- Le **mura sono i piazzamenti `muro`** (poligono chiuso in ordine di id): spostarli in F4 rigenera muro, torrette e cancelli.
  Nome, cancelli e strade stanno in `assets/world/villaggio.txt` (modificabile, con copia incorporata).
- **Edifici = piazzamenti** (tipi `casa-*`, `locanda`, `fabbro`, `forno`, `stalla`, `chiesa`, `pozzo`, `cartello`), così si
  spostano, ruotano ed eliminano in F4. Sono costruiti a runtime (`buildings.rs`) come **una mesh con colori di vertice** per
  edificio (stile cel-shaded, un solo materiale) più collider a parallelepipedo; lo zoccolo affonda sotto il terreno.
- Cancelli ad arco con piloni e battenti (`Hinge`/`PropState` della casetta): aperti 6–21, chiusi la notte con un varco
  pedonale di 1,2 m sempre libero.
- Terreno: spianamento morbido dentro le mura (+6 m) e strade a selciato come terzo strato di alfa sul terreno (come la
  sabbia), peso dalla distanza alle polilinee, texture generata in locale.
- Il layout iniziale lo genera `scripts/gen_villaggio.py` (con controlli di sovrapposizione e anteprima); poi si rifinisce in F4.

## Conseguenze
- Il terreno spianato si calcola all'avvio: spostare i vertici del muro in F4 aggiorna subito il muro, il terreno al riavvio.
- Le strade seguono `villaggio.txt`, non i vertici del muro.
- Interni delle case e abitanti: lavoro futuro (le porte sono chiuse).
