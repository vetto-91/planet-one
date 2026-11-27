# ADR-0008: Abitanti e guardie con dialoghi IA

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.18.0

## Contesto
Il villaggio (ADR 0007) deve essere abitato, con personaggi che parlano e con cui si può chiacchierare tramite la IA locale
(ADR 0005) e la voce locale (ADR 0006).

## Decisione
- Dati degli abitanti in `assets/world/abitanti.txt` (posto, sguardo, orari, aspetto) e persona in `assets/ai/<nome>.txt`
  (carattere, voce, saluto, conoscenze, segreti). Un solo sistema `villagers::villager_routine` per tutti.
- Routine semplice: lavoro di giorno, piazza la sera (percorso Dijkstra sul grafo delle strade di `villaggio.txt`),
  di notte nascosti in casa; le guardie sempre al posto con lanterna notturna.
- Solo il Viandante parla delle quest; per gli altri vale il saluto della persona e poi l'offerta "Parliamo d'altro…".
- Skin ricolorate dalla skin del Viandante (`make_skin_abitanti.py`, solo locale). Superato dai personaggi glTF di ADR 0015: script e skin tolti a novembre 2026.

## Conseguenze
- Tutto resta locale; con IA o voce spente restano il saluto scritto e il testo.
- Le posizioni sono statiche nei file: se si spostano le case in F4, vanno aggiornate a mano.
- Le voci italiane di Piper sono tre (riccardo, paola, serena): i personaggi maschili condividono riccardo con velocità diverse.
