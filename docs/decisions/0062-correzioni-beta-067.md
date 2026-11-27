# ADR-0062: Le correzioni della beta 067

- Stato: accettato
- Data: 2026-11-27
- Versione: v0.0.68.0

## Contesto

La prova della v0.0.67.0 del 18 e 19 novembre ha prodotto 70 segnalazioni F12 valide
(`~/.local/share/planet-one/segnalazioni/2026-11-1[89]_*`). Sono state divise in sei blocchi, per
priorità, con un branch per blocco e un copione del pilota per ogni bug riprodotto
(`prove/b1…b6-*.txt`):
1. urgenze;
2. collisioni;
3. modelli;
4. luci e ombre;
5. abitanti, camera e dialoghi;
6. gioco, suoni e interfaccia.

Le funzioni nuove chieste nelle stesse segnalazioni sono nel backlog della ROADMAP.

Qui le decisioni che cambiano il modo di lavorare del codice o dei file di dati. Il resto è nel CHANGELOG.

## Decisioni

### Prestazioni e luci
- **Budget delle ombre delle luci puntiformi**: fanno ombra solo le luci accese più vicine alla camera
  (`graphics::budget_point_shadows`); il calo di FPS nel tempo veniva dalle ombre di tutte le luci accese.
- Una luce sulla stessa entità di una mesh non viene scartata dal frustum (`NoFrustumCulling`): la Sfera di luce
  restava spenta quando la sua pallina usciva dall'inquadratura.
- **La testa del giocatore in prima persona** sta su un render layer che la camera non disegna ma il sole sì
  (`camera::HEAD_LAYER`): l'ombra ha la testa.
- Le luci interne restano accese fino a 20 m dall'edificio (isteresi in `update_interior_lights`).

### Abitanti e paese
- **Grafo delle strade** (`villagers::RoadGraph`): oltre ai punti vicini collega gli incroci a T (un capo di strada
  vicino a un tratto di un'altra) e le piazze (un nodo al centro). Prima interi pezzi restavano isolati e chi partiva
  da lì andava dritto alla meta, attraverso le mura. Un test controlla che sull'isola ogni strada sia raggiungibile,
  e a Fontebruma le strade del paese (i sentieri oltre i ponti del continente restano a parte).
- **Cancelli di notte**: un abitante che arriva al cancello chiuso lo apre e si richiude 2 s dopo il suo passaggio
  (`village::gate_for_villagers`); il «varco pedonale» citato nei commenti non esisteva.
- **Chi parla si volta** verso il giocatore (`villagers::face_speaker`, anche nella chat con l'IA). Un passante
  si ferma solo se è nel mirino (entro 0,6 m dal centro dello schermo) e per 5 s al massimo.
- **La lancia delle guardie** è dritta davanti alla mano, col gomito piegato sopra l'animazione
  (`upright_spears`, `bend_spear_arms` dopo `AnimationSystems`); parlando passa a tracolla, una seconda lancia
  agganciata alla schiena.
- Dati del paese corretti a mano, e aggiornati anche in `gen_villaggio.py` e `gen_mercato.py`, perché rilanciare
  gli script cancellerebbe i ritocchi successivi:
  - casa di Agnese girata, con la porta sulla strada;
  - via Sud-Ovest fra bancarella del pesce e casa di Nina;
  - via Est attorno alla casa P-0064;
  - galline in un cortile libero.

### Missioni e interfaccia
- Nei file delle missioni: `dai-se-manca: <oggetto> <n>` dà l'oggetto solo a chi non ne ha (l'arco della Prova
  dell'Arco) e `regione: isola|nord` dice in che regione stanno i punti delle tappe. Di base vale l'isola: isola e
  continente hanno coordinate proprie, e la bussola non indica un punto fuori dalla sua regione. Un obiettivo
  «parla con …» resta valido ovunque.
- Il risveglio sceglie l'oggetto nel mirino; Esc o «Lascialo lì» chiudono il pannello dell'indole senza scegliere.
- Un mobile vuoto lo dice nel prompt, «(Vuoto)», calcolando il contenuto iniziale senza aprirlo.

### Suoni e voci
- Il passo è di legno se un collider sopra il fondale regge i piedi (moli, barche, ponti), non più secondo la quota
  sul mare.
- Il passo nell'acqua bassa è uno sciabordio con gocce, non più un «bloop» che saliva di tono.
- Tutte le voci sono di Piper in locale (rigenerate quelle del Viandante e del ladro, che erano di edge-tts). Un
  personaggio ha la stessa voce nei dialoghi scritti e nella chat con l'IA.
- La traversata ha la **voce narrante** (`it_IT-paola-medium`, clip `prologo/narratore-*`): le righe stanno in
  `cutscene::departure_lines` e `crossing::return_lines`, e `gen_voci_prologo.py` le legge da lì. Ogni riga dura
  secondo la sua lunghezza.
- I fumetti restano finché la loro voce parla (`VoiceQueue::bark_speaker`).

### Crediti
- `CREDITS.md` alla radice raccoglie autore, licenza e fonte di ogni asset; `assets/models/LICENSES.md` rimanda lì.
  Ogni asset nuovo ci va nello stesso commit.
- Musiche e immagini generate con Gemini, e il brusio del mercato generato con l'IA di Google, sono indicati come
  tali. Il brusio era attribuito per errore a una registrazione dell'autore.

## Conseguenze

- Ogni correzione ha un copione: `scripts/beta_check.sh prove/b*.txt` le riprova tutte.
- `vita-paese` ora finisce nei tempi (5 s per mezz'ora di gioco a velocità 6) e i suoi stati servono a cercare
  abitanti bloccati.
- Il pilota ha valori nuovi per le verifiche:
  - `npc:<nome>:sguardo`, i gradi fra lo sguardo dell'abitante e il giocatore;
  - `/personaggio/risveglio_pannello`;
  - `/interfaccia/seguita/verso_qui`;
  - i tasti `+` e `-`.
- Restano per dopo le funzioni nuove della beta, nel backlog della ROADMAP.
