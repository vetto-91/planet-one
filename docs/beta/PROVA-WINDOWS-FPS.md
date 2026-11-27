# Prova su Windows: il calo di FPS nel tempo

Questa guida si dà all'assistente di programmazione (IA) sul PC Windows, insieme alla cartella del
progetto. Ha due parti: **cosa fa l'autore** (giocare e raccogliere) e **cosa fa l'IA** (leggere e
riassumere). Lo scopo non è correggere il gioco su Windows, ma capire la causa con dati buoni.

## Il problema

Il 27 novembre 2026, con la v0.0.68.0 su Windows, in un'unica partita di un'ora e mezza gli FPS sono
scesi da 122 a 18. Le righe `[PERF]` del log, una al minuto, dicevano:

| ora reale | data di gioco | FPS | entità | luci | mesh | materiali |
|---|---|---|---|---|---|---|
| 11:33 | 21 mar, 10:49 | 122 | 12 830 | 121 | 1 146 | 304 |
| 12:18 | 21 mar, 17:43 | 87 | 12 849 | 121 | 1 135 | 301 |
| 12:22 | 21 mar, 20:14 | 47 | 12 943 | 121 | 1 136 | 300 |
| 12:29 | 1 apr, 10:02 | 27 | 12 740 | 121 | 1 140 | 283 |
| 12:37 | 22 apr (pioggia) | 21 | 12 997 | 122 | 1 181 | 280 |
| 12:58 | 22 giu, 08:34 | 18 | 12 724 | 121 | 1 168 | 297 |

- **Accumulo:** tutto ciò che si contava restava fermo, quindi cresceva qualcosa che non si contava.
- **Stesso punto:** sulla spiaggia, 108 FPS all'inizio e 19 alla fine.
- **Continente:** andando sul continente gli FPS tornavano a 70.
- **Risorse:** la scheda video era all'80%, la CPU al 25–30% (un core pieno), le ventole al massimo.
- **Tick del render:** alle 12:56 il log diceva che nel mondo del render il contatore dei tick aveva
  superato 3 258 167 296. È il tetto degli avvisi di Bevy (`MAX_CHANGE_AGE`) ed è raggiunto in 84
  minuti: circa 650 000 tick al secondo, quando di solito sono qualche migliaio per fotogramma.

Sul Linux di sviluppo il calo non si riproduce con la stessa chiarezza. Da qui la prova su Windows.

## Parte 1 — per l'autore

1. **Pacchetto**: dalla pagina Releases di GitHub scarica lo zip Windows della prima versione che
   ha `diagnosi` e `prestazioni.txt` (dalla v0.0.71.0). Estrailo in una cartella nuova.
2. **Impostazioni**, nel menu Esc → Grafica, come il 27 novembre:
   - preset **Medio**;
   - **VSync spento** e limite di FPS «nessuno», per vedere il costo vero;
   - schermo intero (F11).
3. **Prima di giocare**, a gioco appena avviato in partita, apri la shell con `\`, scrivi `diagnosi`
   e premi Invio, poi chiudi la shell con `\`.
   - Resta fermo circa un minuto: il gioco spegne una cosa alla volta e misura.
   - Poi premi **F12** e scrivi la nota «inizio».
4. **Gioca come il 27 novembre**, per almeno un'ora:
   - giri in paese, al faro, alla torre antica e al laghetto;
   - qualche notte passata con **T**, fino a portare la data a giugno;
   - un po' di pioggia, se capita.
   Non serve fare altro: il gioco scrive da solo una riga al minuto.
5. **Quando gli FPS scendono sotto 30**:
   - ancora `diagnosi` nello stesso punto dell'inizio (la spiaggia del risveglio);
   - poi **F12**, con la nota «FPS bassi».
6. **Se puoi, prova il continente**: vai al Continente del Nord (Gaspare al porticciolo), aspetta un
   minuto e premi **F12** con la nota «continente». Poi torna sull'isola: aspetta un minuto e premi
   ancora **F12** con la nota «ritorno».
7. **Annota a mano**, se ci riesci, con Gestione attività → Prestazioni → GPU:
   - temperatura e frequenza della scheda video, all'inizio e alla fine;
   - se le ventole erano al massimo.
8. **Cosa mandare**:
   - Le segnalazioni stanno in `%LOCALAPPDATA%\planet-one\segnalazioni\`. Ogni cartella ha
     `schermo.png`, `stato.json`, `log.txt`, `prestazioni.txt` e `nota.txt`.
   - Il gioco toglie già i percorsi della tua cartella utente, il nome del personaggio e la chat con
     la IA. Puoi comunque aprirle e controllarle prima di mandarle.
   - Mandale tutte, con le annotazioni del punto 7.

## Parte 2 — per l'IA

Lavori su un PC Windows con la cartella del progetto Planet One. Le istruzioni del progetto sono in
`AGENTS.md`. Rispondi in italiano.

### Cosa fare

1. **Raccogli le segnalazioni** dalla cartella indicata dall'autore, di solito
   `%LOCALAPPDATA%\planet-one\segnalazioni\`, in ordine di data.
2. **Leggi `prestazioni.txt`** dell'ultima segnalazione. Contiene:
   - in testa, la scheda: versione, scheda video, backend (Vulkan o DX12), driver, finestra,
     VSync, grafica, minuti di partita e conteggio degli eventi;
   - sotto, tutte le righe utili della partita:
     - `[PERF]`, una al minuto: `fps`, `frame N ms medio/massimo`,
       `tick principale/render per fotogramma`, `viste ultime/massime`, entità, ora, meteo,
       regione, e `risorse` con luci accese, luci con ombra, animazioni, mesh, materiali, suoni;
     - `[EVENTO]`: attese con T, viaggi, mappa, grafica cambiata, F12;
     - `[DIAGNOSI]`: per ogni parte spenta, il tempo del fotogramma senza di lei;
     - `[GRAFICA]`: le opzioni applicate.
3. **Costruisci una tabella nel tempo** con: ora reale, data di gioco, regione, FPS, ms medi,
   tick per fotogramma (principale e render), viste, entità, luci accese, luci con ombra,
   animazioni, materiali. Segna gli eventi `[EVENTO]` fra una riga e l'altra.
4. **Confronta** l'inizio con la fine e cerca cosa cresce insieme ai ms del fotogramma:
   - **tick del render per fotogramma** (il sospetto principale): se crescono, qualcosa nel render
     gira sempre più volte;
   - **viste** (camera e ombre): se crescono, si disegnano più viste;
   - **luci accese e luci con ombra**: lampioni e lanterne di sera;
   - **pioggia** (meteo) e **ora** (notte);
   - **temperatura della GPU** (annotazioni dell'autore): un calo dovuto al calore non cambia nessun
     contatore e torna da sé lasciando raffreddare;
   - gli **eventi**: il calo comincia dopo un'attesa con T, un viaggio, una mappa?
5. **Confronta le due diagnosi** («inizio» e «FPS bassi»):
   - quale parte costa molto di più alla fine che all'inizio;
   - se la «scala di rendering al 50%» fa guadagnare molto, il limite è la scheda video; se no, è
     la CPU.
6. **Continente e ritorno**: se gli FPS tornano alti sul continente e restano alti al ritorno,
   l'accumulo è nel mondo dell'isola e il viaggio lo azzera; se ricadono subito, è l'isola in sé.
7. **Scrivi il resoconto** in `docs/beta/RESOCONTO-WINDOWS-FPS.md`:
   - la tabella del punto 3;
   - le ipotesi confermate e quelle escluse, ognuna col dato che lo dimostra;
   - le righe di log più utili, copiate così come sono;
   - cosa resta da capire.

### Cosa non fare

- Non installare programmi, driver o strumenti senza chiedere all'autore.
- Non mandare file o dati in rete: niente upload, niente servizi online.
- Non modificare il codice del gioco né le impostazioni dell'autore: questa è una raccolta di dati.
- Non copiare nel resoconto dati personali. Se in un file trovi un nome utente, un percorso
  personale, un nome del computer o il testo di una chat, toglilo e segnalalo come difetto del
  filtro della privacy (`src/privacy.rs`).
- Non tirare conclusioni da una sola misura: gli FPS cambiano molto con luogo, ora e meteo. Si
  confrontano misure nello stesso punto, alla stessa ora, col sereno.
