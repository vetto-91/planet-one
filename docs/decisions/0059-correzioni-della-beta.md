# ADR-0059: Le correzioni della beta v0.0.63

- Stato: accettato
- Data: 2026-11-17

## Contesto
La prova della beta v0.0.63 ha prodotto 47 segnalazioni (`~/.local/share/planet-one/segnalazioni/`, una lista e
46 cartelle F12). Su v0.0.66.0 erano ancora tutte aperte, tranne i panni davanti alla porta, già spostati con F4.
Erano divise in sei gruppi:
- gruppi 1–4 (questo ADR): dati, interfaccia, missioni, personaggio;
- gruppo 5: edifici e arredi (ADR 0060);
- gruppo 6: mare, suoni ed editor F4 (ADR 0061).

Controllando i piazzamenti è venuto fuori un danno: il pulsante «Annulla ultimo» di F4 toglieva l'ultimo oggetto
della lista invece dell'ultima mossa. Così il commit f0605b9 aveva perso cinque oggetti:
- i rottami del risveglio con monete, mela e bisaccia (P-1281, P-1282);
- due lampioni (P-1283, P-1284);
- un carretto (P-1285).

## Decisione
### Dati (gruppo 1)
- I cinque oggetti persi tornano con gli stessi id, seed e rotazioni.
- Il carretto dietro il forno (P-1247) si stacca dalla finestra, verso le casse.
- Le botti (P-1258) escono dalla scala della casa del porticciolo.
- L'introduzione della nuova partita racconta solo il risveglio.

### Interfaccia (gruppo 2)
- **Frecce:** con l'arco in mano, quante ne restano: sul riquadro dell'arco e accanto alle barre, rosse a zero.
- **Comandi:** Impostazioni → Controlli è una tabella funzione | tasti, per gruppi, con tutti i tasti
  (`settings_ui::CONTROLS`).
- **Shell:** l'aiuto ha un comando per riga, comando in oro e spiegazione in bianco (`shell::HELP`).
- **Interfaccia nascosta:** H, o `hud off` nella shell, nasconde l'interfaccia di gioco
  (`AudioSettings::hide_hud`, salvato).
- **Pausa:** nel menu Esc restano visibili ora, data, meteo, monete, bussola, coordinate e FPS
  (`menu::draw_status`).
- **Avvisi:** quelli sotto la bussola restano a schermo da 4 a 12 secondi, secondo la lunghezza, e aspettano in
  coda (`save::notice_secs`, `Session::tick_notice`).
- **F12:** dopo la foto si apre una casella per la nota (`AppState::BugNote`). Una foto con la nota vuota è
  un'altra inquadratura della segnalazione precedente (`schermo-2.png`, `stato-2.json`). Una foto con una nota
  nuova apre una segnalazione a sé.

### Missioni (gruppo 3)
- **Tipi:** ogni voce del diario ha un tipo (`QuestKind`): storia principale, secondarie, eventi e generate
  dall'IA. Quest'ultima per ora è vuota, in attesa delle missioni create dall'IA locale.
- **Missione seguita:** nel diario le voci sono raggruppate per tipo, con «Segui». La missione seguita
  (`quests::Tracked`) si salva (`segue =`); se non è scelta, è la prima aperta della storia.
- **Dove porta una tappa** (`Step::target`):
  - prima il `dove: x z | <abitante>` del file;
  - poi l'abitante di `parla`;
  - poi il punto di `vicino` o del campo da liberare.
  - Le 18 tappe senza luogo hanno ora il loro `dove:`; un test controlla che non ne manchino.
- **Guida a schermo:**
  - la bussola punta alla tappa della missione seguita;
  - un rombo dorato con la distanza segna il punto sullo schermo; fuori vista, o alle spalle, una freccia
    resta sul bordo;
  - Tab mostra la missione seguita con le tappe fatte e quella in corso, anche appena svegli;
  - prima tutto questo funzionava solo per «Denaro Rubato», e solo in parte.
- **Gaspare:** dopo la sua domanda il dialogo finisce senza «Arrivederci», e la casella del nome si apre subito.
  Detto il nome, Gaspare risponde da solo (`awakening::greeting_pending`).
- **Il nome nei dialoghi:** sostituisce «viaggiatore» nei testi di «Denaro Rubato», nei saluti scritti e nel
  prompt dell'IA locale (`Character::address`). Le battute col nome si leggono dal vivo con la voce Piper locale
  del personaggio (`voice::says_player_name`). Se il TTS non è pronto, resta la clip registrata.
- **Sfera di luce:** lei e lo scudo etereo si salvano (`luce =`, `scudo_etereo =`), e la sfera rinasce da
  `magic::ensure_orb`. È più luminosa (1,6 milioni di lumen, portata 26 m) e sta sopra la testa, un passo
  avanti nella direzione dello sguardo.
- **Pausa vera:** nei menu e nei pannelli il tempo del mondo si ferma (`Time<Virtual>`, `menu::pause_world`):
  animazioni, fisica, particelle, abitanti. Pannelli, musica, avvisi, mappa, scasso e pilota automatico contano
  il tempo reale.

### Personaggio (gruppo 4)
- **Prima persona, corpo:** ruota solo col giro dello sguardo. Guardando in basso resta dritto e si vedono petto
  e mani.
- **Bisaccia:** è agganciata all'osso della schiena (`figure::AttachToBack`) e segue il busto in ogni animazione.
  In corsa l'arma sulla schiena le sta sopra.
- **Prima persona, arma:** quella della mano si nasconde. Al suo posto c'è una copia a metà grandezza davanti
  alla camera (`equipment::first_person_weapon`): l'arco a sinistra di profilo, le lame a destra, con un leggero
  ondeggiare. Non passa più davanti agli occhi quando ci si ferma.
- **Rottami del risveglio:** gli oggetti compaiono insieme alla pozza e spariscono solo oltre 14 m.
- **Nemici bloccati** (`entities::Stuck`):
  - dopo un secondo quasi fermi aggirano l'ostacolo, una volta a sinistra e una a destra;
  - i tentativi tornano a zero solo dopo 4 secondi di cammino libero in linea retta: camminare bene di lato
    durante un aggiramento non conta;
  - dopo tre tentativi, mentre inseguono e se il giocatore non li vede, passano oltre il primo punto libero a
    terra 2,5–5,5 m più avanti;
  - di pattuglia, invece, il percorso si accorcia fino al punto dove si sono fermati e tornano indietro (il
    primo punto resta, perché dice da quale campo vengono). Senza questa regola un bandito del campo a
    (76, 40) saltava lo stesso ostacolo avanti e indietro tutta la notte.

## Conseguenze
- Ogni tappa nuova delle missioni deve avere un luogo: il test `every_open_step_shows_the_way` lo richiede.
- La pausa ferma il tempo virtuale: un sistema nuovo che deve andare avanti nei pannelli usa `Time<Real>`.
- Le battute col nome si sentono con la voce sintetica dal vivo, non con quella registrata. Senza Piper resta la
  clip, che dice «viaggiatore».
- La casella della nota F12 non si scrive dal pilota automatico: il copione `f12` prova Esc e le inquadrature.
- Restano da provare a mano:
  - la sensazione dell'arco in prima persona;
  - la pausa con l'IA in chat;
  - lo sblocco dei nemici nei campi dei banditi.
