# Changelog — Planet One

Formato versione: `vMAJOR.MINOR.FEATURE.BUGFIX` (vedi [docs/VERSIONING.md](docs/VERSIONING.md)).

## Non rilasciato

- **Il mare al tramonto** (beta v0.0.63, n. 22; ADR 0061): sole e luna scendono dietro la linea del mare invece di passarle davanti, e il mare diventa opaco allontanandosi. Il fondale si vede solo vicino a te, fino a una decina di metri. Sparisce la fascia chiara sull'orizzonte: il mare prendeva la luce del fondale che aveva sotto, e oltre l'isola, dove sotto non c'è nulla, una luce sbagliata. Ora si vedono le sue onde vere. È il primo shader del gioco (`assets/shaders/distance_fade.wesl`).
- **Passi sull'erba** (n. 26): nel fossato erboso fuori dalle mura, più basso del livello del mare, i passi suonavano d'acqua; ora l'acqua si sente solo sul mare vero.
- **Brusio del mercato** (n. 35): in piazza si sente la registrazione «Vita tra i banchi» dell'autore, in ciclo senza stacchi, al posto del mormorio sintetico. Sale e scende come prima con la distanza, l'ora e la pioggia.
- **Editor dei piazzamenti (F4)** (n. 39 e 40):
  - «Annulla ultimo» toglieva l'ultimo oggetto dell'elenco (anche la casa del giocatore); ora **Annulla** e **Ripeti** tornano indietro e avanti fra le modifiche, senza limite di passi, con **Ctrl+Z**, **Ctrl+Y** o Ctrl+Maiusc+Z e con i pulsanti del pannello;
  - il mirino colpisce anche i modelli veri e non solo il terreno; fra più oggetti vicini **Tab** o la **rotella** passano al successivo;
  - nell'elenco c'è la **ricerca** per id, tipo o nota, e **Vai e seleziona** porta la camera davanti all'oggetto;
  - **camera libera** con **F6** (solo in F4): vola staccata dal personaggio, che resta fermo, per arrivare vicino agli oggetti.

## v0.0.66.0 — Gioco più leggero e schede video integrate (2026-11-16)

- **Risoluzione della scena** (ADR 0058): nel menu Esc → Grafica si può disegnare il mondo al 100%, 75% o 50% della risoluzione dello schermo e poi ingrandirlo; interfaccia, mappa e foto restano nitide. Il preset **Integrata** parte al 75%: sull'Arc integrata Intel da 25 a 31 fotogrammi al secondo a 1080p e da 36 a 41 a 720p. Alta, Media e Bassa restano al 100% e non cambiano.
- **Schede video integrate** (ADR 0057):
  - al primo avvio su una scheda integrata Intel o AMD, o senza scheda video, il gioco sceglie da solo il preset grafico **Integrata**: ombre minime, niente effetti, meno erba e texture a metà risoluzione;
  - chi ha già scelto la grafica dal menu la ritrova com'era, e su una scheda dedicata non cambia nulla;
  - sull'Arc integrata Intel (Core Ultra 9 285K) da 11–16 a 25–36 fotogrammi al secondo (1080p e 720p), senza scatti e con un terzo della RAM;
  - nel menu Esc → Grafica ci sono il preset nuovo, le ombre di stufa, lumi e lanterne, la distanza di erba e fiori e le texture ridotte.
- **Gioco più leggero** (ADR 0056), senza cambiare nulla di quello che si vede e si gioca. Sui 53 copioni di prova:
  - circa il 19% di fotogrammi al secondo in più in media (da 46 a 55);
  - il copione più pesante passa da 21 a 29;
  - il 17% di RAM in meno e il 12% di memoria video in meno;
  - il cambio di regione (isola ↔ continente) si blocca per 0,3 secondi invece di 0,6–0,9.
- La generazione del mondo all'avvio usa tutti i core: terreno, natura, edifici e cielo.
- Terreno, texture e mesh restano solo sulla scheda video; gli strati del terreno non copiano più tutta la griglia.
- Luci, materiali e particelle si aggiornano solo quando cambiano; lampioni, stufa e lume spenti non calcolano le ombre.
- Nel log i movimenti del mouse non compaiono più (sono a livello `debug`).

## v0.0.65.0 — Isola, Prologo e traversata completi: ricompense, scene, eventi, l'approdo e le rifiniture (2026-11-15)

- **Cinque luoghi nuovi sulla mappa** (ADR 0055): il Faro, il Porticciolo, il Laghetto, il Ponte sul ruscello e il Poggio del Naufrago si scoprono avvicinandosi e poi si raggiungono col viaggio veloce.
- **La legnaia e la porticina**: dietro la chiesa, contro le mura a sud-ovest, c'è una legnaia con le cataste e una scaletta, e nel muro una piccola porta. La sera Pietro ci va di nascosto: seguilo senza farti vedere e ne scoprirai il segreto.
- **Apnea dei Pescatori di Perle**, il talento attivo del Nuoto: in acqua profonda, tenendo premuto Ctrl, ti immergi fino al fondale. Il fiato è contato (una barra compare sopra la vita) e al largo della spiaggia del relitto c'è una cassa sommersa da aprire.
- **Le carte nelle tasche**: con le Dita di Mercurio, insieme alle monete, sfili anche il biglietto di Gino per Marta, il profilo di cuoio della Guardia Gino o il taccuino di poesie della Guardia Mario. Leggerli ne rivela il segreto.
- **Si parte d'estate**: l'ultima missione del Prologo porta il calendario al 21 giugno, come vuole la trama.
- **Correzione**: l'avviso di una pedinata scoperta diceva sempre «si è accorta di te», anche per un uomo.
- **La traversata** (ADR 0054): chi passa per la via sotto il mare sbuca da un'antica fonte di pietra a gradoni tra le radici di una quercia gigante, a pochi passi dall'Approdo delle Brume; la barca attracca al molo. La via sotto il mare ora si prende davvero, in tutti e due i sensi: in fondo ai gradini oltre la Porta Sommersa, e nella fonte sul continente per tornare sull'isola. **I compagni partono con te**: Silvia o Dino vengono sul continente e, congedati, ti aspettano all'approdo.
- **Gli eventi dell'isola si vedono** (ADR 0053): la cassa del relitto sulla spiaggia, la bottiglia verde (con il suo messaggio), il tronco cavo con le farfalle, il lampione che pulsa d'azzurro sotto la pioggia, la mela da lasciare alla guardia che batte i denti nella notte di neve; e da guardare, la scintilla azzurra in cima alla torre, il serpente di luce sui gradoni della grotta all'alba e la nave fantasma che passa nella nebbia.
- La **cassa del relitto** (ora sulla sabbia) e il **tronco cavo** sono mobili veri da aprire; il **lampione azzurro ronza**; la scintilla, il serpente di luce e la nave contano solo se li guardi; il **pedaggio del ponte torna ogni settimana**.
- **Scene delle secondarie** (ADR 0052): Dino ti aspetta fuori da Porta Sud per un duello amichevole e, al tramonto, ferma con te due predoni sulla strada del Sud sotto gli occhi di Aldo; nella Croce delle Quattro Ronde si sale in cima al campanile a mezzogiorno e si scopre la Camera di Risonanza; dopo la Prova dell'Arco Silvia ti porta all'imbocco della grotta, dove ritrovi il bracciale del suo maestro, e promette di venire con te al nord; se hai completato il Cantiere, il giorno della partenza tutto il paese scende al molo a salutarti.
- **Altre scene delle secondarie**: la **Camera di Risonanza** sotto il pozzo si visita davvero (una grata accanto al pozzo si apre su una scala a chiocciola); sulla strada del Sud **Dino e Aldo combattono** con te; al **mulino** due predoni tengono Gino chiuso dentro, e il sacco di farina per Marta pesa davvero cinque chili; la **cena dell'anfora** ha gli ospiti davanti alla locanda e il pane di Marta; Don Anselmo ti dà una **chiave di rame** che apre il **quadro di rame** dietro la stufa del Viandante; con Ferruccio e Gino la tua casa diventa una **casa di pietra**.
- Nella **Camera di Risonanza** si tocca l'acqua della vasca: la stanza canta, il cavo di rame si accende e lascia una **lamina di rame** con la rotta verso nord. Alla **cena dell'anfora** c'è la tavola apparecchiata davanti alla locanda.
- I salvataggi della v0.0.64 ritrovano la tappa giusta delle missioni che hanno tappe nuove.
- **Ricompense uniche delle secondarie** (ADR 0051): la **Lanterna di Vetro di Tiro** (una luce da portare, si accende dalla barra), il **Mantello del Navigatore** (borseggiare e pedinare riesce meglio), i **Guanti Isolanti** (scasso più facile), l'**Arco Maestro** di Silvia (frecce più forti) e lo **Scudo dell'Orlo** (il miglior scudo dell'isola, più leggero). Dopo «Il Pane del Vento» Marta vende le focacce del forno.
- **Lo scudo appeso a Porta Nord** dà lo sconto da Ferruccio, come promesso dalla trama.
- **Correzione: Silvia ora dà le frecce** all'inizio della Prova dell'Arco (il nome dell'oggetto nel file non veniva riconosciuto).

## v0.0.64.0 — Il Prologo rifinito: compagni, segreti, eventi vivi, partenza, età e voci (2026-11-15)

- **Le battute del gioco hanno la voce** (ADR 0050): il saluto secondo la fama, il rifiuto di chi non ti parla, il grazie per un regalo, chi ti coglie a borseggiare, la guardia che ti multa, chi si accorge di essere pedinata, la lite di Tonio e Nina, il bandito del ponte e Lino il carrettiere. Ognuno con la sua voce (160 battute nuove, generate in locale con Piper). I testi sono in `assets/world/battute.txt`.
- **Pronuncia dei nomi**: un piccolo lessico (`assets/audio/voci/pronuncia.txt`) corregge come la voce legge Quipu, Ermes, Fontevivo e Calliope, anche nei dialoghi dal vivo con l'IA.
- **L'età si sente nei dialoghi** (ADR 0047): Gaspare e Don Anselmo parlano in modo diverso a un giovane, a un adulto e a una persona matura (nove battute nuove, con voce).
- **Ettore scende dal faro**: quando arrivi da Gaspare il guardiano del faro è lì al molo, ti saluta, e dopo che hai detto il tuo nome torna su.
- **Due talenti nuovi** (ADR 0049): la **Controparata di Atena** (dopo un colpo parato con lo scudo il fendente successivo fa il doppio) e le **Dita di Mercurio** (frugare nelle tasche dei nemici che non ti hanno visto, e sfilare la chiave di casa con le monete di un abitante: la chiave apre quella porta).
- **Mano del Mastro**: salendo in Costruire ogni tanto raccogli un pezzo di legname o di pietra in più.
- **La tua casa** (ADR 0042): un interruttore accanto alla porta comanda le luci, e il focolare lo accendi e lo spegni solo tu.
- **Correzione: i banditi in mischia ora colpiscono davvero.** Si fermavano appena fuori dalla portata del colpo, così quasi non facevano danno.
- **La Porta Sommersa si vede** (ADR 0048): in fondo al cunicolo della grotta c'è una lastra di pietra con fasce e un disco di rame. Quando le stele del laghetto risuonano, scende nel pavimento con un rombo e oltre si vedono gradini che spariscono in un'acqua azzurra.
- **La Sera delle Lanterne con la piazza piena**: la sera prima della partenza gli abitanti lasciano le loro faccende e si radunano attorno al pozzo, girati verso il centro, e otto lanterne si accendono una a una. Alle 23:30 la festa finisce e tutti tornano a casa. Le guardie restano al loro turno.
- **Eventi con persone vere** (ADR 0046): al crepuscolo Lino il carrettiere ha la ruota nel fango sulla strada del nord, e mentre lo aiuti escono dal bosco due predoni; di giorno una pecora impigliata nei rovi ti segue fino al gregge, se la liberi; in tarda mattinata Tonio e Nina litigano a voce alta in piazza finché non fai da giudice.
- **La Croce delle Quattro Ronde** ora passa dal dialogo con le quattro guardie di ronda, ognuna nel suo turno.
- **Un luogo è esplorato del tutto** solo quando hai visto anche ciò che vi accade (la scintilla sulla torre, l'alba sui gradoni…).
- **Correzioni**: pecore, mucche e galline restavano ferme (il controllo degli ostacoli vedeva il loro stesso corpo); i banditi del pedaggio, uccisi, tornavano dopo qualche minuto.
- **Tre indizi nuovi** (ADR 0045): l'astrolabio spezzato nel baule di Beppe, le pagine antiche del Libro degli Approdi nel nuovo baule della sacrestia e il medaglione di Lauro nel baule in cima alla torre. Trovarli rivela il segreto di Beppe, di Don Anselmo e di Rosa.
- **Pedinare un abitante la sera**: Marta e Agnese, in certe ore della sera, vanno di nascosto in un posto legato al loro segreto. Seguile senza farti vedere fino all'arrivo e lo scoprirai; se ti notano, per quel giorno non c'è niente da fare.
- **La fama conta di più**: con una fama pessima chi tiene all'onestà o alla pietà non ti rivolge la parola, e le guardie notano un borseggio da più lontano e multano di più; con una fama ottima chiudono un occhio.
- **Compagni completi** (ADR 0049): Silvia e Guardia Dino ora prendono colpi dai nemici (anche frecce mirate a loro), dicono quando sono allo stremo e, senza forze, tornano a casa a curarsi fino al giorno dopo. Lontani dai colpi si riprendono. La loro vita si vede in una barra sopra quella della magia.
- **Il compagno resta nel salvataggio**: dopo un caricamento chi ti seguiva è ancora con te, con la vita che aveva; anche chi è ferito resta a casa.
- **Battute con voce dei compagni**: accettare, tornare a casa, chiedere aiuto e ritirarsi hanno ognuna la sua battuta, detta con la voce del personaggio (in locale con Piper). I testi sono in `assets/world/compagni.txt`.
- **Correzione: i salvataggi della v0.0.62 ritrovano la tappa giusta delle missioni.** Le tappe nuove del risveglio, di Agnese e delle stele spostavano gli indici: chi aveva salvato a metà di «Il Nome sulla Sabbia», «Le Voci delle Pietre Antiche» o «Sotto l'Arco dei Sei Gradoni» ricaricava la partita su una tappa sbagliata. Ora al caricamento ogni tappa torna al suo posto (una volta sola).

## v0.0.63.0 — Il Prologo completo: risveglio, scelte, segreti, mondo vivo, partenza e voci (2026-11-14)

- **Le voci del Prologo** (ADR 0050): tutte le battute delle missioni, delle secondarie, degli eventi e delle confidenze degli abitanti ora si sentono, ognuna con la voce del suo personaggio (221 battute, generate in locale con Piper).
- **Suoni**: gli incantesimi, l'anello di ferro centrato nella Prova dell'Arco, i perni e la serratura che cede.
- **Correzione: alle guardie di ronda si può parlare.** Continuavano a camminare e uscivano subito dalla portata del dialogo; ora si fermano e si voltano quando ti avvicini.

- **Armature** (ADR 0049): dal fabbro l'armatura di cuoio, la cotta di maglia e l'armatura a piastre (meno danno subito, più peso; quella a piastre rallenta un poco). Si indossano dalla barra o dall'inventario, e incassare colpi con l'armatura allena l'abilità Armatura.
- **Talenti attivi**, dal grado 30: lo **Scatto di Ermes** (Atletica, tasto `X`, un balzo rapido in avanti) e il **Disarmo di Grazia** (Lama: un nemico ferito gravemente in mischia si arrende e se ne va).
- **Compagni**: Silvia (dopo la Prova dell'Arco) e Guardia Dino (dopo la sua missione), o chi dei due ti ha confidato il suo segreto, possono venire con te: ti seguono e colpiscono i nemici vicini. Nel dialogo, «Vieni con me» e «Torna pure a casa».
- **Il letto di casa**: nella tua casa, `R` vicino al letto fa dormire fino al mattino; vita, vigore e magia tornano pieni.
- **L'Accordo**: Agnese insegna gli incantesimi avanzati (missione secondaria nuova, in tutto sono 18), e c'è il sesto incantesimo, **Calma**, che placa i nemici vicini per mezzo minuto.

- **Il tempo della storia** (ADR 0048): le missioni principali fanno avanzare il calendario e il meteo della scena. Il Prologo va dall'equinozio di marzo all'inizio dell'estate (ADR 0055), e la discesa nella grotta avviene sotto la pioggia.
- **Le Voci delle Pietre Antiche**: Agnese riconosce la pietra del santuario, e in cima alla torre trovi il **Disco di Tiro**.
- **Sotto l'Arco dei Sei Gradoni**: nel laghetto sotterraneo un incantesimo fa risuonare le quattro stele. In fondo al cunicolo trovi il **Quipu Maestro**, e la **Porta Sommersa** è aperta.
- **Oltre il Velo del Nord**: alla Sera delle Lanterne chi hai aiutato ti porta un dono. Prima dell'alba scegli come partire: **la via del mare** con la barca di Gaspare (al timone, se ti ha confidato il suo segreto) o **la via delle profondità**, per la Porta Sommersa. La partenza è raccontata, e poi sei sul continente.

- **Il risveglio sulla spiaggia** (ADR 0047): una partita nuova non ha più la schermata di creazione. Ti svegli tra i rottami senza nome: nella pozza d'acqua ferma fra le rocce scopri la tua **età** (giovane, adulto o maturo, novità che segna appena le statistiche); tra i rottami, la prima cosa che raccogli rivela la tua **indole** (sbarra di ferro, arco spezzato, rotolo di nodi o borsa di monete), che puoi accettare o sostituire scegliendo tu i tuoi punti; e il **nome** lo dici a Gaspare, che da quel momento ti chiama così. La cartella dei salvataggi prende il tuo nome.

- **Voci sui luoghi** (ADR 0046): alcuni abitanti, la prima volta che ci parli, nominano un luogo che non conosci; sulla mappa compare con un «?» finché non ci arrivi.
- **Luoghi esplorati**: sulla mappa un luogo scoperto prende la spunta ✓ quando hai aperto ogni mobile e non ci sono più nemici.
- **Il bottino torna**: nei luoghi fuori dalle mura, una notte a caso, i mobili svuotati si riempiono di nuovo.
- **Casse delle mareggiate**: quattro bauli col lucchetto di bronzo sulle spiagge, senza padrone: si scassinano senza pesare sul karma (forzare un baule in una casa invece pesa).
- **Il pedaggio del ponte**: al tramonto e di notte, due banditi sbarrano il ponte sul ruscello. Puoi pagare, sfidarli o, con molto Carisma, mandarli via con le parole.

- **I segreti sbloccano qualcosa** (ADR 0045): chi ti confida il suo segreto ti dà anche un oggetto (la Mappa delle Secche di Gaspare, le focacce di Marta, il bulbo di Luce Viva di Carlo, la chiave di rame di Pietro…) o un aiuto che servirà più avanti (Gaspare timoniere, Silvia compagna, il Rito del Nome di Don Anselmo…).
- **La via degli indizi**: al faro, al mulino, alla locanda e dall'arciera, nel primo mobile, c'è un quaderno o una lettera. Leggerlo ti fa conoscere il segreto del suo proprietario anche se non siete ancora amici.
- **Ognuno ha i suoi valori**: Don Anselmo apprezza la pietà, Ferruccio il lavoro e il coraggio, Tonio l'onestà, Silvia il coraggio e l'esplorazione… Quello che fai cambia la fiducia di chi ci tiene, nel bene e nel male.
- **Le guardie vedono**: se ti colgono a borseggiare vicino a una guardia, anche lei ti multa e si fida meno.
- **La fama nei saluti**: con il cuore molto leggero (o molto pesante) chi non ti conosce bene ti saluta di conseguenza.

- **Oggetti di trama** (ADR 0044): la tavoletta del ladro, la Moneta Madre, il campanaccio, la bussola di Lauro, le piastrine dei banditi, la chiave di rame, le mappe e gli altri ricordi del Prologo sono oggetti veri, nella nuova categoria «Ricordi» dell'inventario. L'Amuleto Piuma si indossa (+5 kg), la Focaccia del Navigante si mangia.
- **Scelte nelle missioni secondarie**: lo scudo di Ferruccio si tiene o si appende sopra Porta Nord; le piastrine dei banditi si portano al rito di Don Anselmo (e da allora i banditi non tornano più) o si vendono ad Aldo per la taglia; Agnese prepara l'infuso del sogno (+1 Saggezza) o della memoria (+1 Intelligenza); per la scatola di Beppe ci sono tre risposte; la bussola si rende a Rosa o si porta a nord. Ogni scelta pesa su karma e fiducia.
- Lo scasso per Guardia Carlo vale solo per i tre bauli della caserma.

## v0.0.62.0 — L'isola si completa: storia, missioni, abilità, karma, magia e casa (2026-11-14)

- **Correzione: la visuale non girava col mouse a schermo intero** su alcuni PC (con Wayland, anche tramite XWayland, e su macOS): con il cursore bloccato il gioco leggeva la posizione del cursore, che lì non cambia più. Ora, a cursore catturato, legge il movimento del mouse vero e proprio; a finestra resta come prima.
- **Correzione: in prima persona si vedeva l'interno della testa.** La camera stava all'altezza del collo e, con le animazioni (soprattutto correndo), la testa le finiva davanti. Ora è all'altezza degli occhi e in prima persona testa e capelli non si disegnano; guardando in basso il corpo si vede ancora.
- **La Prova dell'Arco** (ADR 0043): oltre il ponte di legno sul ruscello pendono tre anelli di ferro, uno vicino, uno a mezza strada e uno lontano. Centrali con tre frecce di fila, in quest'ordine; un tiro a vuoto fa ricominciare dal primo. È la missione secondaria nuova di Silvia (in tutto sono 17).
- **La tua casa** (ADR 0042): a sud-ovest del paese, fuori dalle mura, c'è un terreno segnato da quattro paletti, il Poggio del Naufrago. Con il tasto `R` spacchi la pietra all'affioramento e tagli il legname alla catasta al margine del bosco (serve un'ascia); sempre con `R`, sul terreno, posi prima le fondamenta (10 pietre) e poi alzi la casa (16 tronchi e 6 pietre). La casa finita è un edificio vero: ci entri, e i suoi mobili vuoti sono il tuo deposito. Si salva con la partita.
- **Missione secondaria nuova**: Un Tetto tra gli Ulivi e il Mare (Beppe e il Viandante). In tutto sono 16.
- L'incantesimo scelto ora si legge a sinistra della barra della magia.
- **Incantesimi** (ADR 0041): la magia ora si spende. `Z` sceglie l'incantesimo, `Q` lo lancia. Sfera di luce (un lume che ti segue), Dardo di Luce Viva (un colpo verso il mirino), Soffio di cura, Passo leggero (15 kg in più per due minuti), Scudo etereo (metà danno per venti secondi). I primi due si conoscono da subito, gli altri arrivano ai gradi 5, 10 e 15 di Magie, che cresce lanciandoli. Intelligenza rafforza il dardo, Saggezza la cura.
- **Karma** (ADR 0040), «la Bilancia di Ma'at»: quello che fai pesa. Risparmiare il ladro, restituire le monete, compiere missioni e fare un dono di nascosto alleggeriscono il cuore; rubare, tenersi monete non tue o dividere l'oro col ladro lo appesantiscono. Ogni variazione compare a schermo col suo motivo, e i prezzi dei negozi la seguono.
- **Fiducia degli abitanti**: ognuno dei 26 abitanti dell'isola ti conosce a modo suo, da Sconosciuto ad Anima affine. Cresce parlandogli ogni giorno, con un regalo gradito (tasto `G`, di fronte a lui), comprando da lui e aiutandolo nelle missioni; crolla se ti coglie a borseggiarlo. Un venditore amico fa lo sconto.
- **Segreti**: ogni abitante ne custodisce uno e lo confida a chi è diventato suo amico (prima, a chi ha Carisma, Saggezza o Intelligenza alti). Dopo la confidenza può parlarne anche nella chat con l'IA locale. Testi in `assets/world/segreti.txt`.
- **Diario**: nuova scheda «Volti e segreti» con la bilancia, gli abitanti incontrati, il legame con ciascuno e i segreti scoperti.
- **Vigore e magia** (ADR 0039): due riserve nuove accanto alla vita. Il vigore cala correndo, saltando e colpendo e torna da solo dopo un attimo di respiro; a zero si resta senza fiato e non si corre finché non se ne recupera un quarto. La magia è la riserva per gli incantesimi che verranno. Costituzione e Agilità alzano il vigore, Intelligenza e Saggezza la magia.
- **Barre nuove**: magia a sinistra, vita al centro e vigore a destra, subito sopra la barra degli oggetti e nello stile dei pannelli (fondo scuro, bordo d'oro, nome e valori). La vecchia barra della vita nell'angolo non c'è più.
- **Scasso**: i bauli sono chiusi a chiave finché non li apri una prima volta. Con un **grimaldello** (da Ferruccio, 6 monete) si gioca il gioco dei perni: ferma l'ago sulla tacca d'oro, una volta per perno. Uno sbaglio fa ricadere un perno e può spezzare il grimaldello; con la pratica la tacca si allarga.
- **Borseggio**: alle spalle di un abitante, `F` gli sfila qualche moneta e `G` gliene infila cinque di nascosto. Se se ne accorge: multa e tasche chiuse fino al giorno dopo.
- **Missione secondaria nuova**: Il Baule Senza Chiave e la Mano Leggera (Ferruccio, Guardia Carlo, Osvaldo). In tutto sono 15.
- **Abilità che crescono con l'uso e livelli** (ADR 0038): correre, saltare, nuotare, portare peso, colpire, parare con lo scudo, tirare con l'arco, passare inosservati, bere pozioni, parlare, comprare, scoprire luoghi e frugare nei mobili allenano ciascuno la propria abilità (grado da 1 a 100). Ogni grado e ogni missione compiuta fanno avanzare il **livello**: a ogni livello un punto statistica, un Nodo d'Oro e 5 punti di vita. Con un Nodo d'Oro, ai gradi 15, 30, 50, 75 e 100, si attiva un rango del **talento** dell'abilità (corsa più veloce, salto più alto, più portata, più danno, prezzi più bassi…). Tutto nel pannello del personaggio (C) e nel salvataggio. Shell: `abilita`, `livello`, `talento`.
- **Luoghi scoperti e viaggio veloce** (ADR 0037): un luogo compare sulla mappa quando ci arrivi; da allora un clic sul suo nome nella mappa ti ci riporta (non con un nemico alle calcagna). Vale sull'isola e sul continente e si salva con la partita. Shell: `veloce <luogo>`.
- **Undici eventi casuali** sull'isola, legati a luogo, ora e meteo: il relitto della mareggiata, il carretto nella nebbia, il fuoco sulla torre, la pecora smarrita, il serpente di luce all'alba sulla grotta, il lampione che ronza sotto la pioggia, la disputa del mercato, il sentiero delle farfalle, la nave fantasma nella nebbia, la ronda infreddolita nella neve, il messaggio nella bottiglia.
- **Altre tre missioni secondarie**: L'Armatura Lucida di Guardia Dino, La Croce delle Quattro Ronde (si sale sui quattro lati delle mura), Il Cantiere di Fontevivo (dopo la grotta, per chi ha aiutato almeno quattro abitanti). In tutto sono 14.
- **Undici missioni secondarie sull'isola** (ADR 0036), proposte dagli abitanti quando ci parli: Il Pane del Vento (Marta e Gino), La Luce che Risponde nella Nebbia (Ettore), Lo Scudo dell'Orlo Spezzato (Ferruccio), Tre Nomi per le Ombre (Don Anselmo), Il Pascolo del Sole (Ida), L'Anfora dei Cent'Anni (Osvaldo), Il Compleanno di Pietro, Il Sogno della Farfalla (Agnese e Rosa), La Promessa sulla Torre Antica (Rosa), Il Segreto della Porpora (Nina), I Fili Sotto le Pietre (Ferruccio e il Viandante). Entrano nel diario quando te le propongono; alcune chiedono oggetti, un'ora precisa o un campo liberato dai nemici. Testi in `assets/quests/prologo/secondarie.txt`.
- **I bivi di «Denaro Rubato»** (ADR 0035): se ti avvicini al ladro senz'arma in pugno non attacca e gli si può parlare. Puoi farti ridare le monete e lasciarlo andare (ti ascolta se hai letto la sua tavoletta al campo, o se hai molto Carisma), dividere l'oro in segreto, oppure sfidarlo. Alla consegna puoi dare al Viandante tutte e dodici le monete o tenertene quattro. Chi risparmia il ladro e consegna tutto riceve la Moneta Madre. L'esito resta nel diario e nel salvataggio.
- **Storia principale del Prologo** (ADR 0034): quattro missioni nuove attorno a «Denaro Rubato». *Il Nome sulla Sabbia*: una partita nuova comincia sulla spiaggia a est del molo, tra i rottami; Gaspare ti soccorre e ti manda dal Viandante. *Le Voci delle Pietre Antiche*: il Viandante rivela il continente, poi Gino, le rovine, il santuario e la torre antica di notte. *Sotto l'Arco dei Sei Gradoni*: Don Anselmo e la discesa nella grotta fino al laghetto e al cunicolo. *Oltre il Velo del Nord*: Gaspare prepara la barca, la Sera delle Lanterne in piazza, la partenza prima dell'alba.
- **Il continente si apre con la storia**: Gaspare offre il passaggio solo dopo l'ultima missione del Prologo (dal continente si torna sempre).
- Le missioni sono scritte in `assets/quests/prologo/missioni.txt`, modificabile a mano; compaiono nel diario (J) e si salvano con la partita. Shell: `missioni [on|off]`, `missione <id> <tappa>`.
- Shell e pilota: un nome esatto vince su uno che lo contiene («gino» è il mugnaio, non la Guardia Gino).

## v0.0.61.0 — Continente del Nord: la Piana delle Brume e i suoi abitanti (2026-11-12)

- **Continente del Nord, primo passo** (ADR 0033): una seconda regione di 1.200 × 1.200 m, la Piana delle Brume dell'Atto I. Approdo delle Brume, bosco di querce e betulle, Grande Lago con isolotto, Rio delle Betulle, collina di Barnaba, Fontebruma (mura, due porte, piazza con la Fonte Grigia, locanda, fabbro, forno, chiesa, case) e i 16 luoghi della trama, con i sentieri che li collegano e i nomi sulla mappa. Nebbia e pioggia sottile di base.
- **Viaggio**: Gaspare, al porticciolo, porta in barca al continente e riporta indietro dall'approdo; il personaggio conserva inventario, quest, data e ora, e ogni regione ricorda com'era stata lasciata (anche nei salvataggi). Shell: `regione <isola|nord>`.
- Luoghi nuovi: arco di pietra, carbonaie, palafitte. Vegetazione del continente piantata a zone attorno al giocatore.
- **Abitanti del continente**: i 12 di Fontebruma (l'Anziano Vieri, Dalia la locandiera, Brando, Mirta, Corrado, Luce, Sereno, il Capitano Dario e le guardie Greta, Tancredi e Vespero), Olmo al lago, l'eremita Nereo, le tre famiglie isolate (Giona, Ada e Milo; Rocco e Brenno; Tullio, Sira e Nonna Calliope) e Barnaba sulla collina. Ognuno ha casa, letto, orari, saluti con la voce e una personalità per l'IA locale presa dalla trama; Mirta, Corrado e Sereno vendono. Missioni ed eventi arrivano nei prossimi passi.

## v0.0.60.0 — Beta test: audio posizionale, passi sull'erba, voci, camera e gronde (2026-11-12)

- **Passi sull'erba**: un fruscio leggero di steli invece di un tonfo sordo più forte dei passi sulla pietra.
- **Audio posizionale** (ADR 0032): campane, voci, uccelli, gabbiani, pesci e tuoni si sentono dalla parte da cui arrivano; le voci seguono chi parla.
- Pilota: segnala gli intoppi (frame oltre i 100 ms) con il comando in corso, nel log e nel rapporto.
- **Voci**: non gracchiano più a volume pieno (prima superavano il massimo); mentre un personaggio parla la musica si abbassa un po' di più.
- **Camera**: non entra più nelle gronde dei tetti girando attorno al giocatore; le falde di case, chiesa e pozzo la fermano come i muri.

## v0.0.59.0 — Beta test: prove automatiche fino a 4K, meteo, interfaccia, insegne (2026-11-11)

- **Insegne**: le targhe di arciere, fabbro, forno e locanda mostrano il nome della bottega invece di un rettangolo vuoto.
- **Pioggia e neve**: niente più gocce come lunghe barre e fiocchi come grossi quadrati davanti alla camera; i fiocchi sono tondi. La nevicata è molto più fitta: pioggia e neve cadono soprattutto davanti alla camera (prima quasi tutte dietro o di lato) e restano piene anche girandosi di colpo.
- **Scala dell'interfaccia**: a 1440p e 4K l'interfaccia si ingrandisce da sola (×1,33 e ×2) invece di restare minuscola; nelle impostazioni (Gioco → Scala interfaccia) si sceglie Automatica o 75–200%. Targhette, fumetti e mappa restano al loro posto.
- **Meteo, ombre**: col cielo coperto (pioggia, temporale, nebbia, neve) il sole non proietta più ombre nette: la luce diretta quasi sparisce e quella del cielo diventa diffusa. Col nuvoloso le ombre sono tenui. Il sereno non cambia.
- **Sottotetti**: la rampa finiva contro il muretto e in soffitta non si saliva; ora finisce prima e c'è il percorso fino in cima.
- **Caserma**: due brande nel sottotetto per Livio e Mario, le ronde della notte.
- **IA locale, prima risposta lenta**: il modello si carica con lo stesso contesto piccolo della chat (3072 token). Prima poteva caricarsi col contesto predefinito di Ollama (enorme se configurato a 256k): 9,2 GB invece di 3,3, quasi tutti sul processore, e la chat non rispondeva in 150 s. Il caricamento anticipato avviene solo sotto il pilota automatico; nel gioco normale il modello si carica alla prima richiesta e, dopo la prima risposta (o il primo errore), il log dice quanta parte sta sulla scheda video e avvisa se una parte va sul processore. Il modello resta in memoria 10 minuti dopo l'ultima richiesta (prima 2).
- **IA locale**: la prima risposta può richiedere fino a 150 s (prima 40 s, e la chat si arrendeva); il messaggio d'errore nomina l'abitante giusto invece del Viandante.
- **Inventario**: le monete si contano a parte («Oggetti: 4   Monete: 30»).
- **Targhette degli abitanti**: quando due si sovrapporrebbero, quella più lontana sale sopra l'altra.
- **Modalità senza finestra** (`PLANET_ONE_HEADLESS=1`, ora il modo di base di `beta_check.sh`): la camera disegna in un'immagine, per le prove anche quando lo schermo non c'è; `PLANET_ONE_GFX=basso|medio|alto` per le prove.
- **Prove a più risoluzioni**: `beta_check.sh --risoluzioni giro|tutte|720p|1080p|1440p|4k|LxA`. Di base ogni copione gira a una risoluzione standard diversa, a rotazione (720p, 1080p, 1440p, 4K); `tutte` li prova a tutte e quattro. Le risoluzioni che non stanno sul monitor girano senza finestra, alla misura esatta. Il rapporto ha le colonne Risoluzione, FPS medio e 1% peggiore; il pilota scrive in `RAPPORTO.txt` la risoluzione vera della finestra e gli FPS (un campione al secondo).
- **Prove che non disturbano**: `beta_check.sh` di base gira senza finestra (nessun input da tastiera, mouse o gamepad; i gamepad sono spenti nella modalità senza finestra) e con l'audio in una scheda virtuale di PulseAudio, a casse mute; il comando `registra` registra quella. `--con-finestra` e `--con-casse` tornano come prima. Foto di riferimento rifatte senza finestra per tutti i copioni; `meteo` e `scuri-casetta` inquadrano meglio, `quest-completa` raccoglie le monete a griglia.
- Shell: `volume <musica|ambiente|voci> <0-100>` (non salvato), utile per ascoltare gli effetti senza musica.
- Pilota: `camera <yaw> <pitch> [distanza]`, `ruota`, `prima-persona`, `giro-foto <nome> <n>` (foto tutt'attorno); `registra <nome>`/`ferma` (audio degli altoparlanti in WAV, Linux) e `scripts/ascolta.py` (volume, saturazione, silenzi, eventi, spettrogramma, parlato con whisper.cpp locale); `guarda <nome> <distanza>` mira a metà facciata degli edifici; `compra`, `prendi`, `chiudi`, `mira <oggetto>`, `di <frase>`; il `dump` mostra negozio, mobile aperto e chat. Nuovi copioni `negozi-mobili`, `interfaccia`, `ia-locale`, `edifici`.

## v0.0.58.0 — Beta test: ronde, nuoto, banditi (2026-11-11)

- Banditi: uccisi nei primi 40 s di gioco ricomparivano subito; ora il popolamento iniziale avviene una volta per scena e chi muore torna solo di notte.
- Shell: `dai` accetta singolare e plurale (`dai freccia`).
- **Ronde sulle mura con i turni**: Gino (6-14), Ivo (14-22), Livio e Mario (22-6). A fine turno entrano nella torre più vicina e vanno a riposare; ce n'è sempre almeno una di pattuglia. **Ettore** lavora di notte al faro (17→7) e dorme di giorno. Gli orari possono superare la mezzanotte (`orari: 17 31 31`).
- **Nuoto**: fermi si sta a galla dritti, nuotando il corpo si distende sull'acqua invece di restare in piedi.
- Pilota: `tieni <tasto> <s>` (non blocca il copione); i copioni possono chiedere variabili con `# env: CHIAVE=valore`; il `dump` elenca tutti gli abitanti (fase, nascosto, percorso); nuovi copioni `movimento`, `combattimento`, `salvataggi`, `meteo`, `vita-paese`.

## v0.0.57.0 — Beta test: bug bloccanti (2026-11-11)

- **Morte unica**: qualunque causa porti la vita a zero (contatto, frecce, caduta) il giocatore rinasce allo spawn con la vita piena, 2 s di invulnerabilità e la scritta «Sei svenuto…»; inventario e monete restano. Prima le frecce potevano portare la vita sotto zero senza morte, e da lì il giocatore non moriva più.
- **Rete sotto il mondo**: chi finisce sotto il terreno senza un pavimento sotto i piedi viene riportato in superficie (log `[RETE]`). Le gallerie della grotta non sono toccate.
- **Grotta**: l'imbocco delle gallerie era **impraticabile**: il terreno attraversava la discesa e dalla camera non si passava (la prova di camminata non aveva il terreno). Ora il terreno ha un buco all'imbocco. Chiuso anche un varco nel pavimento verso il laghetto.
- **Viandante**: se ti avvicini interrompe il giro e torna al banco; al mattino apre gli scuri della casetta, che restano aperti fino a sera.
- Le frecce rispettano il trucco `invisibile`. Tolto l'avviso Bevy B0004 (la stufa della casetta).
- Prove: le partite automatiche nascono a finestra, quindi `PLANET_ONE_SIZE` ora vale davvero; avviso se `PLANET_ONE_DATE` non è valida; `scegli` accetta anche il testo; campo `prop:<id>:<campo>`; nuovi copioni `grotta`, `scuri-casetta`, `fps`.

## v0.0.56.0 — Pilota automatico per il beta test (2026-11-10)

- **Pilota automatico** (beta test, ADR 0031): `PLANET_ONE_SCRIPT=copione.txt` gioca da solo (`vai`, `guarda`, `premi`, `avanza`, `scegli`, `foto`, `dump` in JSON, `verifica`, `attendi`, `mai`, `esci`); `scripts/beta_check.sh` esegue i copioni di `prove/` e scrive un rapporto. `PLANET_ONE_SPEED` accelera il tempo, `PLANET_ONE_DATA_DIR` isola salvataggi e impostazioni.
- **F12 «Segnala bug»**: schermo, stato, log e nota in una cartella.
- Shell: `vai`, `corri`, `dove`, `guarda`, `foto`, `dump`, `copione`, `quest <fase>`, `uccidi`, `vicino`, `fps`, `entita`.
- `PLANET_ONE_SIZE` (finestra fissa); `beta_check.sh` confronta le foto con `prove/riferimento/` (`--aggiorna` per rifarle).
- Correzione: `tp <abitante>` usava la posizione locale degli NPC figli di un edificio (il Viandante).
- Copioni in `prove/`: `quest-avvio`, `quest-completa`, `giro`, `f12` passano; `b1-frecce` e `b2-torre` falliscono di proposito e riproducono due bug da correggere (frecce che portano la vita sotto zero senza respawn; caduta sotto il terreno alla torre in rovina).
- Documenti: variabili di debug complete nel README, corsa 1,55×, il ladro non è più «un cubo rosso», `CLAUDE.md` cita i test automatici.

## v0.0.55.0 — Scuri degli abitanti e gallerie (2026-11-10)

- **Scuri**: ora li aprono e chiudono gli abitanti. Al mattino, appena si alzano, vanno alla finestra della loro stanza, fanno il gesto (clip `Interact`, nuova `Anim::Interact`) e gli scuri si aprono a metà gesto; la sera, prima di coricarsi, li chiudono. Le case senza abitanti restano a orario. Nel log: `[VILLAGERS] … apre/chiude gli scuri`.
- **Grotta**: dal fondo della camera si scende (rampa di 4 m) in gallerie sotto la collina: una sala del tesoro con falò, forziere, botte e cassa; un laghetto con stalagmiti; un cunicolo stretto (soffitto basso) che gira ad angolo e finisce in una nicchia con un secondo forziere e un falò. Il forziere ha un bottino migliore (monete, pozioni, a volte una spada lunga). La prova di camminata percorre tutte le gallerie.
- I falò sotterranei non fanno fumo (non ha dove uscire).
- Restano a orario i focolari delle case.

## v0.0.54.0 — Rifiniture di grotta, spada e salvataggio (2026-11-10)

- **Grotta**: fuori dall'arco c'è una rampa di pietra (circa 19°, 6,4 m) con un terrapieno a gradoni sotto, al posto del pendio ripido verso il mare; la prova di camminata ora parte da fuori.
- **Spada in corsa**: invece di sparire si porta sulla schiena (osso `spine_03`) mentre si trotta o si scatta e torna in mano appena ci si ferma. Provato a schermo con la nuova variabile `PLANET_ONE_ANIM=sprint|jog|walk`, che forza l'animazione del giocatore.
- **Salvataggio**: caricando una partita i focolari e gli scuri restano come salvati; l'orario li riscrive solo quando l'orologio avanza normalmente, non quando salta per il caricamento.

## v0.0.53.0 — Grotta e spada (2026-11-10)

- **Grotta**: ora ha un ingresso ad arco a gradoni, un corridoio di 6 m e una camera di 10 m sotto un cumulo di massi irregolari (prima era una stanzetta con tetto piramidale). La prova di camminata la percorre dall'ingresso al fondo.
- **Spada in corsa**: durante il trotto e lo scatto le braccia portano la mano vicino alla testa e l'arma ci finiva dentro; ora viene riposta (invisibile) finché si corre e riappare appena ci si ferma.
- Resta: la grotta non è davvero sottoterra (la mappa di altezza non ha cavità) e l'ingresso dà su un pendio ripido verso il mare. L'arma riposta non è disegnata sulla schiena.

## v0.0.52.0 — Focolari, scuri e scale (2026-11-10)

- **Focolari**: ogni focolare delle case ha lo stato acceso/spento (E davanti al fuoco), un bagliore tremolante e il fumo dal camino sopra segue il suo stato. Si accendono da soli al mattino e alla sera (o col freddo) e si spengono di giorno; il cambio d'ora ripristina l'orario.
- **Scuri**: gli abitanti li aprono al mattino (7:00) e li chiudono la sera (19:30) e col cielo molto coperto; il giocatore può comunque aprirli e chiuderli.
- **Scale delle mura**: il primo gradino partiva dentro la base di una torre o di un torrione del cancello, per cui scendendo si finiva contro il muro; ora la scala parte oltre (0,1–1,1 m), liberando l'imbocco. Il log scrive `[VILLAGGIO] scala da … a …`.
- **Tetti dei camminamenti**: non ho trovato buchi, sui camminamenti non ci sono tetti (solo sulle torri) e dalla diagnostica `PLANET_ONE_SCAN_WALLS=1` i soli «vuoti» sono i gradini bassi delle scale. Se ne vedi uno, servono le coordinate.

## v0.0.51.0 — Scuri e camini (2026-11-10)

- **Scuri apribili**: le finestre delle case del paese hanno due imposte incernierate che si aprono e chiudono con E, da fuori e da dentro (stesso meccanismo delle finestre del Viandante; lo stato è salvato). Di giorno partono aperte.
- **Camini**: nelle case con focolare il camino ora sale proprio dal focolare, incassato nel muro di fondo, fino sopra il tetto (prima era un moncone nel sottotetto, senza niente sotto); il fumo esce da lì.
- **Arredi**: l'armadio dei piani alti non si mette più davanti a una finestra.
- Nuova prova automatica: nessun ostacolo davanti alle finestre e camino sopra ogni focolare.
- Non ancora fatti: focolare acceso/spento con il fumo legato al suo stato e finestre aperte dagli abitanti.

## v0.0.50.1 — Avvio stabile (2026-11-10)

- **Correzione**: all'avvio il gioco poteva chiudersi con «Failed to get point light visible entities» (panic nel renderer di Bevy 0.20.1: `extract_lights` toglie le liste delle ombre alle luci non cambiate nel fotogramma). Ora le luci con ombre vengono segnate come cambiate a ogni fotogramma (`graphics::keep_shadow_lights_extracted`). Prima capitava in circa metà degli avvii in modalità dev, 0 su 6 dopo la correzione.

## v0.0.50.0 — Mura (2026-11-10)

- **Mura e torri d'angolo**: presso le torri restavano cunei vuoti a tutta altezza fra la testa del muro e la faccia della torre (visibili da fuori, p. es. a (-28,-8), (-20,27), (20,23), (30,3)). I tratti di muro, il parapetto e il cordolo ora si prolungano fino a entrare nella torre; verificato con il modello geometrico dell'anello (0 celle scoperte contro 109 prima).
- Nuova diagnostica `PLANET_ONE_SCAN_WALLS=1`: tira raggi contro le mura e dall'alto sul camminamento e scrive `[SCAN]` nel log.
- Non ancora fatti: scale delle mura rifatte a L e tetti dei camminamenti (da verificare con altri punti precisi).

## v0.0.49.0 — Tetti (2026-11-10)

- **Tetti delle case**: la falda parte oltre il muro e fra il bordo del muro e la falda restava una fessura (fino a mezzo metro) da cui si vedeva dentro; ora un listello di muro la chiude lungo i lati lunghi.
- Nuova prova automatica: dall'interno di ogni casa, a due quote, raggi verso l'alto devono sempre colpire il tetto.
- Restano da verificare a occhio le aperture delle mura presso le torri.

## v0.0.48.0 — Rifiniture (2026-11-10)

- **Guardie in ronda** si fermano anche mentre si chiacchiera con la IA (prima proseguivano e ci si allontanava).
- Le lance delle guardie sono impugnate più in basso e dritte, come un bastone d'arma.
- **Luce che attraversava i muri**: le luci interne degli edifici e le lanterne delle guardie vicine (entro 22 m) ora fanno ombra.
- **Case sepolte**: sul pendio un edificio non resta sepolto per più di 35 cm dal lato alto.
- Il camino sul fianco del tetto è spostato un metro verso l'interno.
- **Prima persona**: il corpo resta visibile (guardando in basso ci si vede) e la camera sta poco avanti agli occhi.

## v0.0.47.0 — Voci (2026-11-10)

### Voci
- **Le prime battute degli abitanti hanno la voce** (50 clip, generate in locale con Piper dalle frasi dei file `assets/ai/*.txt`), con voci maschili e femminili come da persona.
- **Arrivederci**: chiudendo il dialogo con «Arrivederci» o la chat con la IA, l'abitante saluta a voce, con uno di sei congedi maschili o femminili (12 clip in `assets/audio/voci/congedo/`).
- Nuovo `scripts/gen_voci_abitanti.py` per rifare i clip dopo aver cambiato le battute (solo Piper e ffmpeg locali, nessun servizio esterno).

## v0.0.46.0 — Interfaccia (2026-11-10)

### Interfaccia
- **Oggetti a terra**: quelli gettati dal giocatore mostrano la loro icona sopra, come nella barra rapida, sempre rivolta verso la camera.
- **Inventario (I)**: in alto c'è la barra rapida con i tasti 1–0. Si riordina trascinando un oggetto dall'elenco o da un tasto a un altro (scambia con quello occupato); il doppio clic su un oggetto lo mette nel primo tasto libero o, se è già nella barra, lo toglie; il doppio clic su un tasto lo svuota.
- **Contenitori**: **R** prende tutto, **Tab** chiude (oltre a E ed Esc).

## v0.0.45.0 — Personaggi e combattimento (2026-11-10)

### Nemici e personaggi
- **Nemici**: pattugliano a passo (non di corsa) e guardano dove vanno (il corpo non viene più ruotato due volte). Se vedono il giocatore, senza ostacoli in mezzo, entro 14 m smettono di pattugliare: da lontano tirano frecce, da vicino lo inseguono e colpiscono in mischia. Se si nasconde o è invisibile (shell) lo perdono.
- **Banditi**: uccisi non tornano dopo pochi secondi ma in una notte a caso, tra le 21:30 e le 4:30.
- Gli spiazzi del campo del ladro, dei banditi e delle rovine sono appianati: fuoco, casse e tende non restano sottoterra.
- **Camminata**: velocità di camminata e corsa ridotte (5 e 7,75 m/s) e animazioni meno vivaci.
- Chi dorme resta immobile nel letto, senza il passo a riposo.
- Le guardie tengono le lanterne riposte di giorno: compaiono soltanto al buio.
- **Dialoghi con la IA**: il prompt ora distingue chiaramente la battuta dell'abitante dalle tre opzioni, che sono sempre frasi del giocatore in prima persona.

## v0.0.44.0 — Suoni e acqua (2026-11-10)

### Suoni e acqua
- **Passi**: l'erba è un fruscio morbido con un tonfo sordo, il legno ha un tocco secco e risonante ben diverso, dentro gli edifici si cammina sempre sul legno; nei campi di grano e negli orti gli steli frusciano contro le gambe (nuova superficie).
- **Brusio del paese** rifatto: mormorio di folla fatto di soffio e voci smorzate, non più un ronzio elettronico.
- **Mare**: onde con salita, frangente e schiuma che si ritira, più brontolio grave; il mare si muove più in fretta e «respira». Nella mappa (M) è di nuovo blu pieno e senza nebbia.
- **Acqua dolce**: più liscia, senza brillantini; si cammina più piano e, dove è profonda (più di 1,25 m), si nuota (Spazio per salire); schizzo quando ci si entra e pesci che guizzano vicino al laghetto.
- **Mulino**: albero delle pale fuori dalla facciata, pale più alte della porta, tinta unita senza l'effetto a quadri da lontano.

## v0.0.43.0 — Scale e torri (2026-11-10)

### Scale e torri (ADR 0030)
- **Le scale si salgono davvero**: sotto i gradini visibili c'è ora una rampa liscia su cui si cammina (prima gli spigoli fra gradini di scatole facevano incastrare il giocatore). Vale per case, soffitte, faro, campanile e torre in rovina.
- **Fori nei solai** dove la testa arriverebbe nel solaio sopra le chiocciole (campanile, faro, torre in rovina), senza strisce sottili che fanno sbattere la testa.
- **Faro più largo** (pozzo da 2,3 m, corsia da 0,8 m) e lanterna in cima ingrandita per non occupare la scala.
- Nuova prova automatica (`walk_test.rs`): un giocatore vero, con la stessa fisica del gioco, sale i percorsi di tutte le case e le tre chiocciole; se resta incastrato il test lo dice e indica dove.

### Altre correzioni
- La campana del campanile è una vera campana a coste, appesa alla trave, e la luce della cella viene da una lanterna appesa.
- Nella locanda dietro il bancone ci sono botti al posto dello scaffale.
- Le luci interne si accendono già avvicinandosi alla porta (fino a 2,5 m) e non solo oltre la soglia.
- Gli alberi stanno più lontani dagli edifici (rami dentro la caserma).

## v0.0.42.0 — Correzioni dopo la prova (2026-11-10)

### Correzioni
- Il gioco non va più in crash spostando con F4 gli oggetti vicino a una porta (cartello della chiesa): i comandi sul collider tollerano le entità già eliminate.
- Con F4, dentro un edificio hanno la precedenza lampioni e cartelli sull'edificio stesso; lampioni e cartelli finiti dentro case e chiesa vengono portati fuori (`scripts/fix_posizioni.py`, solo se sono ancora dove erano).
- Salvataggi, debug e log usano l'ora locale della macchina invece di UTC.
- Il messaggio «Salvataggio automatico» sta su una riga.
- Il cartello di Fontevivo dice «borgo fortificato»; le targhe di locanda e forno sono appese al muro accanto alla porta, non più bandiere che attraversano il soffitto.
- La piazza della fontana è tutta lastricata.
- La campana suona 12 rintocchi solo a mezzogiorno e a mezzanotte, e un colpo prima dell'alba e del tramonto.
- Danno da caduta oltre i 3,5 m di dislivello.
- Il carretto della porta sud sta fuori dalla torre e ha ruote più curate.
- Mucche, pecore e galline hanno collisione (col giocatore e fra loro) e non attraversano muri e pozzo; i gabbiani salgono per scavalcare il faro e i tetti.

## v0.0.41.0 — Luoghi da scoprire (2026-11-09)

- **Luoghi da scoprire** (ADR 0029): campo del ladro nel bosco, fattoria con Ida, mulino a vento di Gino con le pale che girano, torre in rovina sull'altura con scala fino in cima, rovine di un tempio, grotta nella scogliera, accampamento dei banditi con nemici che ricompaiono, santuario nella radura, cartelli e panchine sui sentieri.
- I luoghi hanno bauli e casse da aprire, il nome sulla mappa e sentieri che li collegano.
- Gli altri abitanti restano invariati; i salvataggi vecchi continuano a funzionare e i banditi compaiono anche lì.

## v0.0.40.0 — Paese vissuto (2026-11-09)

- **Paese vissuto**: fumo dai comignoli (la mattina, la sera e col freddo; forno, fabbro e locanda per tutta la giornata di lavoro), torce sulle mura accese al buio, carretti, botti, casse, panni stesi, reti al porto, insegna del forno.
- I nuovi carretti, botti e casse si aprono come gli altri contenitori e si spostano con F4.

## v0.0.39.0 — Suoni e animali (2026-11-09)

- **Suoni dell'isola** (ADR 0028): onde, ruscello, vento in quota, uccelli, gabbiani, grilli, brusio della piazza e passi diversi per superficie, sintetizzati in locale, con il nuovo volume «Ambiente» (ora include anche meteo e campane).
- **Animali** (ADR 0028): mucche e pecore animate (modelli CC0 Quaternius) che pascolano e si scostano, galline in paese, gabbiani sul porto, uccelli che si alzano in volo, pesci nel laghetto, farfalle nella stagione dei fiori. Si ricreano a ogni avvio, non vengono salvati.
- **Latifoglie nuove**: betulle e alberi frondosi da modelli CC0 Quaternius al posto delle chiome che sembravano palme, con corteccia propria e versione semplificata da lontano.

### Correzioni
- La superficie del ruscello non si sovrappone più a quella del laghetto.

## v0.0.38.0 — Boschi e vegetazione (2026-11-09)

- **Boschi e vegetazione** (ADR 0027): boschi di abeti, latifoglie e misti su tutta l'isola, con radure, cespugli, rocce, fiori ed erba alta che seguono le stagioni. Nuova opzione «Densità vegetazione».

## v0.0.37.0 — Terreno e acque dell'isola (2026-11-09)

- **Isola** (ADR 0026): alture percorribili, scogliere a est e a sud-ovest, spiagge più larghe, roccia sui pendii e sentieri di terra battuta che collegano le zone dell'isola.
- **Acqua** (ADR 0026): un ruscello con laghetto e ponte di legno; onde sul mare e schiuma lungo la riva, spegnibili dalle impostazioni grafiche.

## v0.0.36.0 — Menu principale e salvataggi (2026-11-09)

- **Menu principale** (ADR 0025): Continua, Nuova partita con classi o punti da distribuire, Carica partita per personaggio, Impostazioni a schede (Gioco, Audio, Video, Controlli), Esci.
- **Salvataggi permanenti** (ADR 0025): un file di testo modificabile per ogni salvataggio, con tempo, personaggio, oggetti, porte, mobili, quest, nemici e abitanti. Automatico ogni 5 minuti e all'uscita, F5 rapido e F9 per ricaricarlo.
- **Getta a terra**: gli oggetti gettati restano dove sono e si riprendono con E.
- Sensibilità del mouse, FPS a schermo e schermo intero restano impostati tra una sessione e l'altra.

## v0.0.35.0 — Statistiche, peso e zaini (2026-11-09)

- **Statistiche** (ADR 0024): forza, costituzione, agilità, intelligenza, saggezza, carisma e fortuna, con il pannello del personaggio (tasto C).
- **Peso** (ADR 0024): si portano 30 kg di base; oltre il limite si cammina lenti e non si corre.
- **Zaini e oggetti nuovi**: quattro zaini fino a 60 kg dal fabbro, Pozione del mulo dall'erborista, Cintura del portatore dal Viandante.

## v0.0.34.0 — Contenitori e faro (2026-11-09)

- **Contenitori** (ADR 0023): bauli, armadi, botti, casse e scaffali di case, botteghe, bancarelle, molo e casetta si aprono con E: si prendono e si posano oggetti, con capienza da 5 a 10 volte quanto porta il giocatore. Contengono poche cose a tema, sempre le stesse.
- **Faro visitabile** (ADR 0023): alloggio arredato, ripostiglio, scala a chiocciola fino alla lanterna e balconata. Chiuso a chiave.

## v0.0.33.0 — Microfono locale (2026-11-09)

- **Microfono nella chat** (ADR 0022): tenendo premuto F2 o «Parla» si parla agli abitanti, con whisper.cpp solo in locale. Il testo si può rileggere prima di inviarlo. Spento finché non lo si attiva; si installa con `setup_ai --stt`.

## v0.0.32.0 — Abitanti ai piani, chiavi e shell (2026-11-09)

- **Abitanti ai piani** (ADR 0020): di sera entrano dalla porta, salgono le scale e vanno a letto; al mattino scendono ed escono. Nessuno sta più fermo davanti a un ingresso.
- **Chiavi e serrature** (ADR 0020): locanda e chiesa aperte, botteghe aperte negli orari di lavoro, case e caserma chiuse. Ogni abitante porta le sue chiavi.
- **Shell dei trucchi** (tasto `\`, ADR 0021): godmode, noclip, invisibilità, teletrasporto, passepartout, oggetti, ora, meteo, modelli IA e voci, collider e piazzamenti. F3, F4 e coordinate restano spenti giocando normalmente; modello e voce escono dal menu Esc.
- **Correzione**: i soffitti erano trasparenti visti da sotto.

## v0.0.31.0 — Interni degli edifici (2026-11-09)

- **Interni visitabili** (ADR 0019): case, locanda, caserma, fabbro, forno, arciere e chiesa hanno porte vere, piani con scale, sottotetto e arredi per tipo di edificio; il campanile si sale fino alla cella delle campane. Terreno spianato sotto gli edifici, finestre visibili anche da dentro, luci nell'edificio in cui si è (nuova opzione grafica).
- **Abitanti a letto**: di notte tornano a casa a piedi e dormono nel proprio letto; le guardie fuori turno dormono in caserma (`casa:` in `abitanti.txt`).

## v0.0.30.0 — Mura percorribili (2026-11-09)

- **Mura percorribili**: il muro è più spesso verso l'esterno, con un camminamento continuo lungo tutto il perimetro, ponti sopra i cancelli e torri cave attraversabili; scale dentro lo spessore del muro (una per cancello, una a est e una a ovest). Le quattro guardie sulle mura ora fanno la ronda sull'intero anello (ADR 0018).

## v0.0.29.0 — Inventario, barra a 10 tasti e FPS (2026-11-09)

- **FPS**: nel menu grafica casella VSync e limite Illimitati/30/60/90/120/144/165/240; predefinito senza limiti.
- **Barra oggetti**: sempre 10 riquadri (1-9 e 0) con icone disegnate, assegnabili dall'inventario.
- **Inventario** (tasto I): categorie, ordine alfabetico, ricerca, filtri (equipaggiati, nella barra, peso massimo), usa/impugna, assegnazione ai tasti e peso reale in kg (monete e munizioni non pesano, le monete non vanno in barra).

## v0.0.28.0 — Animazioni dei personaggi (2026-11-09)

- **Animazioni (Grafica realistica, Fase 3)**: camminata, corsa, salto, attacco per arma, colpito, conversazione per giocatore, abitanti e nemici, con clip accelerate secondo la velocità reale; il nemico colpito e morto si anima e resta a terra qualche secondo; opzione per fermare le animazioni lontane (ADR 0017).
- **Correzione**: in mischia si colpivano solo i nemici a nord, perché si usava la direzione del corpo fisico che non ruota mai.

## v0.0.27.0 — Materiali completi e impostazioni grafiche (2026-11-09)

- **Impostazioni grafiche**: nuovo sotto-menu nel menu Esc con preset Alta/Media/Bassa, qualità delle ombre, ombre dei lampioni, occlusione ambientale, bagliore, antialiasing e particelle (solo aspetto visivo, salvate in `settings.txt`).
- **Materiali**: tende e merci in tela, lampioni e gambe delle panchine in ferro, molo, barche, casse e pali in legno, pagliai in paglia, armi e scudo adeguati; tangenti anche sulle primitive, così le normal map funzionano ovunque.

## v0.0.26.0 — Mondo e materiali PBR (2026-11-09)

- **Mondo e materiali PBR (Grafica realistica, Fase 2)**: terreno, sabbia, selciato, intonaco, travi, pietra, tegole, ardesia e paglia con texture fotografiche CC0 (colore, normal, ruvidità); abeti veri con corteccia e rami al posto dei coni;
  rendering HDR con tonemapping, bagliore delle luci, occlusione ambientale, ombre a cascata e luce d'ambiente dal cielo; prato, chiome e neve adattati alle nuove texture (ADR 0016).

## v0.0.25.0 — Personaggi veri (2026-11-09)

- **Personaggi veri (Grafica realistica, Fase 1)**: giocatore, Viandante, abitanti, guardie e ladro sono ora modelli umani con scheletro (asset CC0 Quaternius), al posto dei parallelepipedi, con posa di riposo;
  armi, scudo, lancia e lanterna si agganciano alle ossa delle mani (ADR 0015). Il nemico rosso resta un cubo; chi cammina scivola finché non arriva la Fase 3.

## v0.0.24.0 — Meteo (Fase 3) (2026-11-08)

- **Meteo (Fase 3)**: sereno, nuvoloso, pioggia, temporale (lampi e tuoni), nebbia e neve, deterministici per data con probabilità per mese; temperatura e meteo nel chip sotto la data;
  nebbia di distanza, luce più cupa, pioggia e neve come particelle nel mondo, neve che si accumula e si scioglie, vento, suoni di pioggia/vento/tuono sintetizzati in locale;
  gli abitanti si riparano in casa con pioggia o neve forti, la stufa del Viandante resta accesa sotto gli 8 °C, i lampioni si accendono col temporale (ADR 0014).

## v0.0.23.0 — Guardie con turni, casa delle guardie e mura percorribili (2026-11-08)

- **Guardie con turni**: 6 guardie ai cancelli su tre turni (6–14, 14–22, 22–6), sempre almeno una per cancello, posti subito oltre l'anta aperta; chi finisce il turno va alla
  nuova **casa delle guardie** (fuori da Porta Nord) e riposa; 4 guardie di ronda sulle mura (ADR 0013).
- **Mura percorribili**: camminamenti con parapetto e scale (una per cancello, una a est e una a ovest) dove salgono le guardie di ronda.
- **Correzioni**: la neve compare anche su torri, mura e merli; le foglie d'autunno cadono in coordinate di mondo (non seguono più la camera) e si spostano col vento;
  lampioni spostati vicino alla chiesa.

## v0.0.22.0 — Stagioni visibili (Fase 2) (2026-11-08)

- **Stagioni visibili (Fase 2)**: erba e sabbia cambiano tinta, chiome verdi/autunnali/innevate, fiori che sbocciano da primavera, neve a terra e sui tetti d'inverno,
  foglie che cadono in autunno, luce stagionale (ADR 0012).

## v0.0.21.0 — Calendario (Fase 1), mani e molo (2026-11-08)

- **Calendario (Fase 1)**: data di gioco dal 21 marzo anno 0 (mesi reali), stagioni, sole stagionale con alba e tramonto che cambiano nell'anno,
  velocità del tempo ×1/×2/×4/×8 nel menu Esc, musica giorno/notte sincronizzata con alba e tramonto, "Aspetta" che avanza anche i giorni; cancelli, lampioni, faro, lanterne e Viandante seguono alba e tramonto
  (ADR 0011).

- **Correzioni**: gli abitanti sul molo (Gaspare) ora camminano sul piano del molo invece di sprofondare nel fondale; tutti i personaggi umani (giocatore,
  Viandante, abitanti, guardie, ladro) hanno le mani (cubetti color pelle ai lati del corpo).

## v0.0.20.0 — Porto, faro e campane (2026-11-08)

- **Strada del sud** dal cancello al mare, **faro** (luce girevole di notte, guardiano Ettore) e **porticciolo** con molo calpestabile, tre barche, casetta dei pescatori e il
  pescatore Gaspare; cartelli e lampioni lungo la strada (ADR 0010).
- **Campana** della chiesa: suono sintetizzato in locale, rintocchi alle ore piene dalle 7 alle 21 con volume secondo la distanza.
- **Correzioni**: di notte gli abitanti nascosti non mostrano più la targhetta; le bancarelle del mercato, la bottega dell'arciere, i pagliai e gli spaventapasseri
  ora compaiono (in v0.0.19.0 mancavano per una lista di tipi incompleta, ora c'è un test).

## v0.0.19.0 — Mercato, negozi, armi e campi (2026-11-08)

- **Negozi e mercato**: pannello di acquisto con le monete (30 iniziali, i nemici ne lasciano); venditori: Viandante (pozioni), armaiolo Ferruccio, arciera Silvia e tre bancarelle in
  piazza (pozioni, mele, pesce). Nuovi oggetti: spada corta, spada lunga, ascia, scudo, arco, frecce, mela, pesce (ADR 0009).
- **Armi e scudo**: si impugnano dalla barra, con attacco e statistiche proprie; l'arco spara frecce verso il mirino; lo scudo riduce i danni. `PLANET_ONE_GIVE=1` per provare.
- **Campi** di grano e orto fuori dalle mura, pagliai, spaventapasseri; nuovi tipi F4 (`arcere`, `bancarella-*`, `campo`, `orto`, `pagliaio`, `spaventapasseri`).
- Cancelli: solo due ante piene; i bottoni stanno sui piloni (dentro e fuori) e si usano col mirino; le guardie stanno accanto ai cancelli.

## v0.0.18.0 — Abitanti e guardie, correzioni al villaggio (2026-11-08)

- **Abitanti e guardie** di Fontevivo (6 + 4) con saluto scritto e chat con la IA locale (carattere e voce propri), routine giornaliera (lavoro, piazza la sera,
  in casa di notte; le guardie ai cancelli con lanterna notturna) (ADR 0008).
- Cartelli: il palo sta dietro la tabella. Cancelli: ante piene (si chiudono davvero, con collisione) e porticina centrale; ogni anta si apre col mirino.
- La pozione è ora sul tavolo dentro la casetta del Viandante. Mura di pietra più chiare.
- La quest del Viandante avanza solo con il Viandante (non con gli altri abitanti).

## v0.0.17.0 — Villaggio di Fontevivo (2026-11-08)

- **Villaggio di Fontevivo**: mura con torrette (dai piazzamenti `muro` di F4), due cancelli (Porta Nord e Sud) con battenti che si aprono di giorno e si
  chiudono di notte (varco pedonale sempre libero), strada a selciato e stradine, piazza con pozzo, chiesa con campanile, case di vario tipo,
  locanda, forno, lampioni e cartelli con il nome; nome sulla mappa e banner d'ingresso; terreno spianato dentro le mura (ADR 0007).
- F4: nuovi tipi (`casa-*`, `locanda`, `fabbro`, `forno`, `stalla`, `chiesa`, `pozzo`, `cartello`); gli alberi non vengono creati dentro le mura.
- Il mare ora si disegna sopra sabbia e strade (anche dalla mappa). Script: `gen_villaggio.py`, `make_selciato_texture.py`, `make_cartelli.py`.

## v0.0.16.0 — Voce IA locale con Piper, fase 2 (2026-11-08)

- Voce IA (fase 2): le risposte dei dialoghi IA sono lette ad alta voce con Piper **in locale** (programma esterno, senza rete); voce per personaggio,
  casella "Voce IA" e scelta della voce nel menu, `PLANET_ONE_TTS_MOCK=1` per provare (ADR 0006).
- `scripts/setup_ai.sh|ps1 --tts` installa Piper e le voci italiane con consenso esplicito; `scripts/gen_tts_voci.py` ora usa Piper locale (niente più edge-tts online).

## v0.0.15.0 — Dialoghi con IA locale, fase 1 (2026-11-08)

- Dialoghi con IA **solo locale** (Ollama, `gemma4:e4b` configurabile), fase 1 testuale: opzione "Parliamo d'altro…" a fine dialogo, chat con
  tre repliche suggerite e testo libero, interruttore e stato nel menu Esc; i dialoghi e le quest scritti hanno la precedenza (ADR 0005).
- CI di release solo manuale, con una casella per Windows e una per macOS (meno minuti a pagamento).
- `scripts/setup_ai.sh` / `setup_ai.ps1`; persona del Viandante in `assets/ai/viandante.txt`; `PLANET_ONE_AI_MOCK=1` per provare senza modello.

## v0.0.14.0 — Windows e macOS (2026-11-08)

- Porting Windows e macOS: archivi costruiti da GitHub Actions (Windows x86_64 `.zip`, macOS universale `.tar.gz`), impostazioni nella cartella di
  configurazione di ogni piattaforma, cursore `Confined` su Windows, niente console in release su Windows (ADR 0004).
- Piazzamenti (alberi, lampioni, fiori) incorporati nel gioco: compaiono anche nei pacchetti distribuiti.

## v0.0.13.0 — Interazioni col mirino e oggetti a terra (2026-11-07)

- Interruttore luce, pulsante serranda e maniglie interne delle finestre usabili col mirino da dentro (e non da fuori): ogni oggetto a parete ha il suo lato d'uso.
- Si parla con il Viandante (e gli altri personaggi) solo guardandolo col mirino.
- Oggetti piazzati (alberi, lampioni, fiori…) appoggiati subito al terreno vero all'avvio (prima restavano a y = 0,5 finché non si ricostruivano).
- Pannello F4 a larghezza fissa (niente tremolio sopra "Ruota"); maniglie delle finestre invisibili.

## v0.0.12.0 — Casetta da dentro, finestre a vetro scorrevole, spiaggia (2026-11-07)

- Casetta: porta, interruttore, stufa, serranda e finestre di nuovo usabili da dentro (gli occhi del giocatore erano dentro il collider del tetto);
  finestre a due stadi da dentro (alza il vetro, poi apri gli scuri); apertura del banco e vani delle finestre senza collider pieni, quindi il
  Viandante si vede (etichetta) e gli si parla dal finestrone e dai vetri; serranda abbassata e scuri chiusi bloccano la vista.
- Lampioni: la luce non attraversa più le pareti (ombre per i 3 più vicini).
- Isola: il mare è solo attorno all'isola (niente acqua nelle depressioni interne), barriera di riva che segue la costa, spiaggia con la texture
  sabbia che sfuma nel prato.
- F3: il collider del terreno non viene più disegnato per intero (60 → 7 FPS): griglia sparsa e paletti rossi sulla riva attorno al giocatore.

## v0.0.11.0 — Isola, debug con mirino, oggetti ruotabili (2026-11-07)

- Isola: terreno esteso a 300 m con costa irregolare, mare attorno e muro invisibile sulla riva; il quadrato della heightmap resta al centro.
- Debug F4: mirino al centro per piazzare (Ctrl tenuto libera il mouse per il pannello), rotazione con freccia rossa (trascina, R, strumento Ruota),
  niente compenetrazioni (anche tra aree), "Ridistribuisci", pannello sopra le linee e a scorrimento.
- Oggetti piazzati affondano nel terreno (niente basi sospese sui rilievi).
- Niente interazioni, dialoghi, etichette o barre vita di NPC attraverso i muri.

## v0.0.10.0 — Strumenti debug, oggetti veri, musica notturna (2026-11-07)

- Viandante: di notte, se la luce è accesa si alza quasi subito, chiude prima porta/finestre aperte, spegne la luce e torna a letto.
- Casetta: tetto più spesso (sottofaccia e bordo di gronda), niente luce tra tetto e muro.
- Mappa: spostamento con frecce/WASD e trascinamento, C ricentra sul giocatore.
- Debug F4: cursore libero in 3ª/1ª persona (tasto destro = guarda), poligono con OK, rettangolo a 4 angoli,
  distribuzione regolare, allineamento a griglia/linea, anteprima del cerchio, strumenti Gomma e Sposta.
- Musica notturna (`Lanterns_In_The_Grove`) dal tramonto all'alba con dissolvenza incrociata.
- Oggetti veri (alberi, fiori, panchina, lampione con luce notturna) creati da `debug/placements.txt` (`world_props.rs`).

## v0.0.9.0 — Mappa, debug piazzamenti, casetta rifinita (2026-11-07)

- Mappa in gioco (M): vista dall'alto della scena reale (camera ortografica, nord in alto), bussola in alto a sinistra,
  segnalino del giocatore, zoom con rotella e +/- (`MapView`).
- Modalità debug piazzamenti (F4): segnaposto singoli, in gruppo o ad area, aggiunti/eliminati dalla mappa o in 3D,
  salvati in `debug/placements.txt` e riletti all'avvio.
- Casetta: pulsante della serranda solo all'interno (parete +X, vicino allo spigolo), scuri spessi e più larghi, vetro
  alle 3 finestre, giunti muro/fondamenta chiusi (niente luce dal retro), luci con bias ridotto.
- Il Viandante chiude porta e finestre lasciate aperte dopo un ritardo casuale (45 s – 3 h di gioco); sera e mattina
  controllano anche le finestre; di notte si alza entro 1 h di gioco se qualcosa viene aperto.
- `scripts/sync_drive.sh --copy` (nessuna cancellazione su Drive).

## v0.0.8.0 — Interazione col mirino, casetta rifinita, routine del Viandante v2 (2026-11-07)

- Prima persona: occhi a ~1,57 m da terra. Chip delle coordinate con anche la quota Y dei piedi.
- Targhette di Viandante, Ladro e nemici solo entro 10 m dal player; i fumetti (anche "Zzz…") stanno sotto il nome.
- Interazione: gli oggetti (porta, finestre, stufa, interruttore, serranda) si usano solo **guardandoli col mirino**
  (entro 3 m, non dietro un muro) e hanno la priorità sull'NPC; il Viandante si parla solo entro 3 m.
- Casetta più bassa (affondata di 30 cm). Luci interne con ombre (la luce non attraversa i muri).
  Nuovi oggetti: interruttore della luce, serranda del negozio con scritta CHIUSO, 3 finestre apribili con imposte.
- Routine del Viandante v2: stufa 6–10 e 17–21 (non toccata se l'ha cambiata il giocatore), mattina (luce, porta,
  stufa, serranda), sera (stufa, serranda, luce, porta, letto).
- Voci: clip troppo basse normalizzate (`scripts/normalize_voci.py`), guadagno ×1,4 e musica abbassata mentre parla un personaggio.
- Variabili d'ambiente di debug (`PLANET_ONE_POS`, `PLANET_ONE_LOOK`).

## v0.0.7.0 — Colonna sonora, casetta esplorabile, voci (2026-11-07)

- Colonna sonora in loop (`audio/music/legacy_of_the_first_light.ogg`) con slider "Volume musica" nel
  menu Esc; il volume è salvato in `~/.config/planet-one/settings.txt`.
- Casetta-chiosco (mesh con le 4 viste della foto, tetto a padiglione, apertura del banco trasparente)
  al posto del Viandante, che ora fa da negoziante dentro il chiosco; l'insegna dice "GUSTO DEL BOSCO".
- NPC e casetta appoggiati al terreno (`SnapToGround`): l'NPC non è più mezzo sepolto.
- `docs/voci/viandante.txt` e `docs/voci/ladro.txt` con tutte le battute, generati da `scripts/export_voci.py`.
- Voci dei personaggi nel gioco (`assets/audio/voci/`, mappa in `voci.txt`): partono con ogni riga di dialogo,
  il congedo del Viandante e la battuta del ladro; la riga con direzione/distanza del ladro è composta da due
  clip. Slider "Volume voci" nel menu Esc.
- Casetta esplorabile 4x6 m: fondamenta e scale di pietra (texture generata `pietra.png`), porta apribile (E),
  interni (letto, cucinino con stufa, tavolo con sedie, baule, scaffale, lanterna), stufa accendibile con camino e fumo.
- Camera in terza persona che non attraversa pareti e tetto della casetta.
- Routine del Viandante: dopo le 21 accende la stufa, chiude la porta e va a dormire; alle 6 si sveglia,
  spegne la stufa, apre la porta e torna al banco. Da addormentato risponde "Zzz…".
- Versione del gioco in basso a destra nel menu Esc (`src/version.rs`).

## v0.0.6.0 — Rifiniture: diario, giorno/notte, skin, UI condivisa (2026-11-06)

- Diario delle quest (J) con tab "Da fare" / "Completate" e quest espandibili; pannello TAB
  alimentato dalla stessa voce.
- Quest in cartelle: logica in `src/quests/<quest>/`, testi modificabili a mano in
  `assets/quests/<quest>/dialoghi.txt` (ricaricati dal Reset scena, fallback incorporato).
- Dialoghi: E/Invio per avanzare, scelta Sì/No, il Viandante saluta se ci si allontana;
  il ladro apostrofa il giocatore quando lo vede (fumetti sopra la testa).
- Quest attivata da sola se il ladro muore prima di parlare col Viandante.
- Texture erba low-poly sul terreno e cielo a nuvole (panoramica convertita in cubemap a runtime).
- Ciclo giorno/notte (1 h = 60 s) con sole e luna, luce e cielo che cambiano; tasto T "Aspetta".
- Skin per player, ladro e viandante (mesh a UV per faccia); targhette "Viandante"/"Ladro".
- Tema UI condiviso (`ui_style`): menu, diario e HUD coerenti; chip più leggibili, coordinate
  su una riga, banner quest più in basso.
- Fix: A/D invertiti; luna meno luminosa; la quest si completa solo se il dialogo finale è letto.
- Repository: `.gitignore` completo, `scripts/sync_drive.sh`, documentazione allineata a `main`.

## v0.0.5.0 — Fase 5: Vertical slice RPG "Denaro Rubato" (2026-11-05)

- Quest completa in memoria: NPC Viandante allo spawn -> dialogo -> accetta
  "Denaro Rubato" (overlay titolo 3 s). Ladro rosso (QuestThief, 30 HP) in
  nord-ovest: ucciderlo (morte definitiva, no respawn) -> spawn 12 monete
  gialle (QuestCoin) -> raccolta automatica entro 2 m -> torna dal Viandante
  -> dialogo finale -> "Missione completata".
- Bussola sempre visibile in alto al centro: coordinate x,z + freccia rossa
  verso il target corrente (ladro / moneta piu' vicina / NPC).
- TAB tenuto: pannello obiettivi con checklist ([✓]/[ ] e conteggio monete).
- Overlay titolo quest in alto (banner oro).
- Reset scena resetta anche QuestState (full reset).
- Nessun salvataggio su file (come da piano).
- Test: quest accettata via log [QUEST], bussola+TAB verificati via screenshot, 0 panic.

## v0.0.4.9 — Avvio fullscreen + pulsante "Esci dal gioco" (2026-11-05)

- Avvio a schermo intero di default (WindowMode::Fullscreen(Current, Current));
  F11 alterna fullscreen/finestra. Cursore catturato subito in Playing.
- Menu Esc: nuovo pulsante "Esci dal gioco" → AppExit::Success (chiusura pulita,
  nessun kill forzato: pronto per salvataggi futuri).
- Test: avvio fullscreen 4480x1440 verificato via screenshot; 0 panic.

## v0.0.4.8 — Cursore in fullscreen + menu + pitch 180° (2026-11-05)

- apply_cursor_for_mode: cattura cursore (Locked+nascosto) SOLO in fullscreen
  E Playing; in Paused (menu Esc) o a finestra il cursore torna visibile e libero.
- warp_cursor_to_center: in fullscreen+Playing riporta il cursore al centro ogni
  frame → su X11 (Confined) la rotazione yaw resta infinita (360°), i delta
  continuano ad arrivare anche al bordo.
- Pitch verticale in 1a persona: ±1.54 rad (~±88°, quasi 180°): al massimo in
  alto (cielo) o in basso (terreno) la visuale si ferma, niente ribaltamento pg.
- Log [CURSOR] per stato cattura; [INPUT]/[EVENT] invariati.
- Test: fullscreen cattura ON, menu Esc cursore libero, yaw 360, 0 panic.

## v0.0.4.7 — Cursor capture in fullscreen + log completo input (2026-11-05)

- F11 fullscreen: cursore nascosto + `CursorGrabMode::Locked` (tutti gli input
  restano nel gioco). A finestra: cursore visibile e libero (esce e interagisce
  col sistema). Su X11 Locked ricade su Confined (max disponibile).
- Nuovo sistema `log_input_events`: logga TUTTI gli eventi con prefisso
  [INPUT]/[EVENT] — key press/release, mouse button, motion, wheel, cursor moved.
  Utile per debug via grep: `cargo run 2>&1 | grep "\[INPUT\]"`.
- Test: log 16 eventi catturati (tasti W/Space/F11, click, movimento mouse),
  fullscreen 4480x1440 con cursore bloccato, ritorno 1280x720 libero — 0 panic.

## v0.0.4.6 — F11 fullscreen + naso allineato al crosshair (2026-11-05)

- F11: toggle fullscreen nativo (`WindowMode::Fullscreen(Current, Current)`)
  gestito dal SO (cattura input mouse/tastiera). Aggiunto alla lista comandi
  del menu Esc. Testato: 1280x720 -> fullscreen -> 1280x720.
- PlayerVisual (naso) ora segue il crosshair:
  - 3a persona fermo: ruota verso la direzione della camera (yaw).
  - 3a in movimento: ruota verso la direzione di movimento (come prima).
  - 1a persona: rotazione = yaw+pitch della camera (corpo segue lo sguardo).
- Test: F11 on/off verificato via dimensioni finestra, rotazione naso
  sinistra/droite misurata via pixel gialli — 0 panic.

## v0.0.4.5 — Pitch coerente 3a/1a persona (2026-11-05)

- Alla transizione 3a->1a (V o RMB) il pitch viene azzerato: la mira parte
  orizzontale, niente salto verticale (prima: guardavi giu' in 3a e vedevi
  il cielo in 1a).
- In 1a: `rotation_x(-pitch)`, stesso segno dy in entrambe le viste:
  mouse giu' = guardi giu', mouse su = guardi su, coerente con la crosshair 3a.
- Hiatus del cursore: lasciato al SO (Ctrl+Alt+Invio full-screen cattura input).
- Test: transizione 3a(giu')->1a(orizzonte), poi 1a su/giu' — 0 panic.

## v0.0.4.4 — Fix prima persona e proiezione barra nemico (2026-11-05)

- Fix rotazione prima persona: `Quat::from_rotation_y(yaw) * from_rotation_x(pitch)`
  invece di `EulerRot::ZYX` (che metteva lo yaw sull'asse Z = visuale ruotata 90°).
- Mouse: letto via `CursorMoved` (delta) invece di `MouseMotion` raw: su X11 i
  device events non arrivano sempre, CursorMoved sì. In 1a persona dy invertito
  (mouse su = guarda su); in 3a persona comportamento storico invariato.
- Player nascosto in prima persona (sia corpo che mesh), riapparso in 3a.
- Fix proiezione barra nemico: `world_to_viewport` in Bevy 0.20 gia' restituisce
  pixel y-down — rimossa la doppia inversione Y che la spostava sotto il nemico.
- Test: 3a persona (orizzonte si sposta col mouse), 1a persona (mouse su = cielo,
  destra/sinistra = yaw corretto, player invisibile) — 0 panic.

## v0.0.4.2 — Fase 4.3: prima persona (RMB/V), barra enemy, barra player in basso (2026-11-05)

- Camera in prima persona VERA: la camera va dentro il player (stessa posizione
  e rotazione yaw/pitch), il pg non si vede. Crosshair centrato e sincronizzato.
- RMB tenuto → prima persona temporanea (rilascio = torna in terza persona).
- Tasto V → toggle lock prima persona (RMB non interferisce col lock attivo).
- Input RMB letto via MessageReader<MouseButtonInput> raw in PreUpdate-equivalente,
  per non essere filtrato da egui/ButtonInput.
- Barra vita player spostata in basso a sinistra, lunghezza dimezzata (110 px).
- Barra nemico sopra la testa: nome "enemy" + barra rossa proporzionale,
  proiettata world→viewport con Camera::world_to_viewport.
- Test: V lock prima persona verificato (0 pixel player = camera dentro il pg);
  RMB/release; movimento in prima persona — 0 panic.

## v0.0.4.0 / v0.0.4.1 — Fase 4: HUD, inventario, combattimento base (2026-11-05)

### v0.0.4.0 — HUD + inventario
- Barra vita in alto a sinistra (egui, colori dinamici verde/giallo/rosso).
- Crosshair centrale. Slot inventario in basso con icone colorate + quantita'
  e tooltip nome. Pickup raccoglibili (Pozione rossa, Moneta gialla) con
  raccolta automatica entro 2 m. Inventario in resource (sopravvive al reset
  scena). PlayerHealth { current, max } sul player.

### v0.0.4.1 — Combattimento base
- Attacco melee su click sinistro: cooldown 0.6 s, range 2.5 m, danno 4,
  target = nemico piu' vicino nel cono frontale (dot > 0.3).
- Nemico con EnemyHealth (20 HP):barra vita sopra il mesh (da fare in UI
  world), morte -> despawn + respawn dopo 3 s sul primo waypoint.
- Danno da contatto player<->nemico entro 1.2 m, ogni 0.8 s, iframe 0.3 s.
- Morte player (vita <= 0): respawn allo spawn con vita piena, camera resettata.
- Nessun attacco durante dialogo aperto.
- Test: click攻击 С Chamb; 0 panic.

## v0.0.3.0 — Fase 3: Entità (NPC, dialogo, nemico patrol) (2026-11-05)

- Nuovo modulo `entities.rs`: NPC (Viandante, blu) con dialogo lineare,
  inimigo (rosso) in patrol tra waypoint con lo stesso MoveAndSlide del player.
- Interazione: prompt "[E] Parla" quando nel range (5 m), tasto E apre il dialogo.
- Dialogo: pannello in basso stile RPG (egui), Enter/Space avanza, Esc chiude.
  Esc durante dialogo chiude il dialogo SENZA aprire il menu pausa.
- Movimento libero durante il dialogo (come richiesto).
- Reset scena include NPC e nemico (SceneNode).
- Test: dialogo aperto/avanzato/chiuso (4 righe), movimento durante dialogo,
  patrol nemico verificato via log — 0 panic.

## v0.0.2.5 — Fase 2.3: salto robusto su pendenze (multi-ray + coyote + hit-history + buffer) (2026-11-05)

- Ground-check multi-ray: 5 raycast (centro + 4 offset XZ) entro 1.5 m,
  normale minima normal.y >= 0.5: rileva il terreno anche su pendenze ripide.
- Coyote time alzato a 0.25 s + cronologia hit fisici di MoveAndSlide
  (last_ground_hit, 0.2 s): il salto resta valido anche se i ray non vedono
  il terreno durante la corsa in salita.
- Jump buffer 0.15 s: se premi Space poco prima di toccare terra, il salto
  parte appena diventi grounded.
- Snap al terreno solo entro 0.95 m (non in caduta libera).
- JUMP_SPEED 6.0 -> 7.5.
- Anti double-jump: buffer + coyote + hit consumati al salto.
- Test: salti in camminata e corsa ripetuti — 0 panic, salto sempre attivo.

## v0.0.2.4 — Fase 2.2: salto affidabile con coyote time (2026-11-05)

- Nuovo `GroundedState { grounded_now, coyote_left }` sul player.
- Coyote time 0.12 s: il salto resta valido 0.12 s dopo l'ultimo contatto
  reale, risolvendo i salti mancati in camminata/corsa sulle pendenze.
- Raycast ground-check esteso a 1.25 m (semi-altezza 0.85 + tolleranza 0.40)
  e filtro sulla normale (solo superfici con normal.y >= 0.6 contano come
  suolo: i muri non danno grounded).
- Anti double-jump: dopo un salto valido il coyote viene consumato.
- Tutto il movimento del player (ground-check, input, gravity, MoveAndSlide,
  facing) spostato in FixedUpdate: pipeline unica, delta stabile.
- JUMP_SPEED 6.0 → 7.0 per un salto visibilmente più alto.
- Camera resta in Update dopo il FixedUpdate.
- Test: salto da fermo, in camminata e in corsa (3 ripetizioni) — 0 panic.

## v0.0.2.3 — Fase 2.1: allineamento definitivo mesh/collider (2026-11-05)

- Fix definitivo: matrice collider costruita come heights[x][z] (riga=X,
  colonna=Z), trasposta rispetto alla prima versione: ora il wireframe F3
  copre esattamente tutta la mesh verde su tutto il terreno 100 m.
- Sonda numerica rimossa dopo la verifica (angoli + centro heightmap).
- Verifica: wireframe copre tutta la mesh, camminata 10 s stabile sul
  pendio, salto, menu Esc — 0 panic.

## v0.0.2.1 — Fase 2.1: character controller kinematico + fix collisioni terreno (2026-11-04)

- Player migrato da `RigidBody::Dynamic` a character controller cinematico
  Avian (`RigidBody::Kinematic` + `CustomPositionIntegration` + `MoveAndSlide`
  in `FixedUpdate`): elimina sprofondamenti e cadute nel vuoto sulle pendenze
  dell'heightfield a celle grandi (100 m / 256 = ~0.39 m).
- Gravità manuale (20 m/s²) in `movement`; salto solo se `Grounded`.
- Terreno allargato a 100 m (da 40 m); heightmap mappata su -1..+1 (grigio
  medio = 0) per avere colline sopra e sotto il livello base.
- Collider heightfield Avian corretto: matrice `heights_zx` (righe=Z, colonne=X)
  come richiesto da Parry, allineata alla mesh.
- Cielo procedurale in `Rgba8Unorm` (lineare) invece di Srgb: sRGB causava un
  "double gamma" che rendeva il cielo grigio; `Tonemapping::None` sulla camera.
- Nebbia di distanza rimossa (era troppo densa a 80 m sul terreno piccolo);
  riattivabile in futuro con slider.
- Luci: `GlobalAmbientLight` 80 cd/m², `DirectionalLight` 4000 lux con
  `shadow_depth_bias 0.05` / `shadow_normal_bias 3.0` per evitare shadow acne.
- Debug: F3 toggle wireframe collider Avian (disattivato di default).
- Diagnostica: log `terrain diagnostics` (image size, height range) a ogni rebuild.
- Test automatici: camminata 6 s su colline (player sempre a y≈0.85, mai nel
  vuoto), salto, menu Esc, 0 panic.

## v0.0.2.0 — Fase 2: Scena (terreno, cielo, nebbia, hot-reload) (2026-11-04)

- Terreno procedurale da heightmap PNG (`assets/textures/terrain_heightmap.png`,
  256×256 in scala di grigi): mesh generato a runtime con un piano suddiviso e
  vertici sollevati per pixel; grigio medio = altezza zero.
- Collider Avian `heightfield` generato dalla stessa heightmap: il player
  cammina realmente sulle colline.
- Hot-reload dell’asset: `bevy/file_watcher` + `AssetEvent<Image>::Modified`
  rigenerano mesh e collider senza riavviare il gioco.
- Cielo procedurale: cubemap generata a runtime (`sky.rs`) con gradiente
  orizzonte/zenit calcolato per pixel, senza asset esterni né HDRI da scaricare.
- Nebbia di distanza (`DistanceFog` + `FogFalloff::from_visibility`) e luce
  ambientale globale (`GlobalAmbientLight`) più calda e neutra.
- Rimossa la vecchia luce ambientale per-entità (spawn sbagliato) e il piano
  piatto segnaposto; il terreno ora è un’entità dedicata (`TerrainMesh`).
- Script `scripts/gen_heightmap.py` per rigenerare il terreno senza asset
  esterni (CC0, nessuna dipendenza aggiuntiva).
- Moduli nuovi: `terrain.rs` (heightmap → mesh/collider) e `sky.rs`
  (cubemap procedurale).
- Test: build pulita, runtime stabile, hot-reload verificato con
  heightmap modificata a runtime (mesh rigenerato senza panic).

## v0.0.1.4 — Fase 1.4: fix menu Esc e FPS overlay egui (2026-11-04)

- Fix regressione "menu Esc scomparso": il toggle era rimasto monodirezionale
  (solo chiusura) dopo la riscrittura della 1.3. Ripristinato il toggle
  bidirezionale `Playing ↔ Paused`.
- FPS overlay riscritto come Area egui in alto a destra (dentro
  `EguiPrimaryContextPass`): il vecchio `Text2d` era in coordinate world
  e risultava fuori schermo con la camera orbit.
- Rimosso il modulo `hud.rs` (Text2d + posizionamento viewport): il contatore
  ora vive in `menu.rs` insieme al resto della UI, con checkbox "Mostra FPS".
- `ShowFps` default `true`, persistente tra pause e reset.
- Test automatici: toggle Esc aperto/chiuso/riaperto, 2 reset consecutivi dal
  menu via click simulato — 0 panic, 0 ambiguità camera.

## v0.0.1.3 — Fase 1.3: fix reset e contatore FPS (2026-11-04)

- Fix del crash "Reset scena": introdotto lo stato transitorio `Resetting`;
  il reset avviene in `OnEnter(Resetting)` dove tutti i sistemi di gameplay
  (`run_if(in_state(Playing))`) sono spenti, eliminando la corsa critica con
  il ground-check che scriveva su entità despawnate.
- Fix della camera duplicata al reset: separato `spawn_camera` (solo startup)
  da `spawn_world` (startup + reset). La camera persiste e viene solo
  riposizionata; sparisce il warning "Camera order ambiguities".
- HUD FPS in alto a destra, attivabile/disattivabile dal menu Esc tramite
  checkbox "Mostra FPS". Usa `FrameTimeDiagnosticsPlugin` e `Text2d`.
- Rimossa risorsa `ResetSceneRequest` (ora obsoleta); codice documentato.

## v0.0.1.2 — Fase 1.2: Menu, salto e corsa (2026-11-04)

- Menu Esc (bevy_egui 0.43 + egui 0.36): lista tasti, slider sensibilità mouse,
  pulsanti "Riprendi" e "Reset scena" (solo da menu, per evitare pressioni accidentali).
- Stato applicativo `AppState { Playing, Paused }`: gameplay sospeso in pausa.
- Salto con Space: ground-check via raycast Avian (solo collider marcati `Ground`),
  impulso y=6.0 solo da terra, anti double-jump.
- Corsa con Shift: moltiplicatore 1.7× su WASD.
- Sensibilità mouse default 0.003 (da 0.005), regolabile a runtime dal menu.
- Reset scena da zero: despawn ricorsivo delle entità `SceneNode` (terreno, luci,
  player) e ricostruzione. La camera persiste (il contesto egui è agganciato ad essa
  e bevy_egui non lo ricrea) e viene riposizionata.
- Architettura: nuovi moduli `scene.rs` (spawn/reset) e `menu.rs` (UI).

## v0.0.1.0 — Fase 1: Movimento (2026-11-04)

- Player: capsule dinamica (Avian) con rotazione bloccata, gravità e collisione col terreno.
- Movimento WASD/frecce relativo all'orientamento della camera (piano XZ), 6 m/s.
- Mesh visivo separato (corpo + "naso") con rotazione smooth verso la direzione di movimento.
- Camera terza persona: orbita col mouse (yaw/pitch clampato) + follow con smoothing.
- Codice organizzato in moduli: `main.rs`, `player.rs`, `camera.rs`.
- Verificato: il player si assesta a y=0.850 (capsula r=0.35, semi-lunghezza 0.5).

## v0.0.0.2 — Fase 0 (2026-11-04)

- fix: inserito `CameraRenderGraph::new(Core3d)` esplicitamente prima di `Camera3d`
  per evitare il warning spurio dell'hook di spawn in Bevy 0.20.
- Nota successiva: il warning "render graph not configured" all'avvio è in realtà
  rumore upstream di Bevy 0.20 (si riferisce a entità Observer interne, non alla nostra
  camera). Il fix sopra è superfluo ma innocuo; la fonte è documentata nel codice.

## v0.0.0.1 — Fase 0: Bootstrap (2026-11-04)

- Progetto scaffoldato: Rust + Bevy 0.20.1.
- Finestra 1280×720 con titolo "Planet One".
- Scena base: cubo placeholder (personaggio), piano terra, luce ambientale + direzionale con ombre.
- Camera terza persona statica (il follow arriva in Fase 1).
- Licenza doppia MIT OR Apache-2.0, README, template issue/PR, convenzioni versioning.
