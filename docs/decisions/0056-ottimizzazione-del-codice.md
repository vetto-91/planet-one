# ADR-0056: Ottimizzazione del codice

- Stato: accettato
- Data: 2026-11-16

## Contesto
Dopo la v0.0.65 il gioco era completo per isola, Prologo e traversata. Era però pesante in alcuni punti:
- **pochi core usati**: nel codice non c'erano `par_iter` né lavori sul `ComputeTaskPool`;
- **copie in RAM**: ogni strato del terreno (sabbia, strade, sterrato, roccia, neve, schiuma) copiava
  l'intera griglia, cioè 361 mila vertici sull'isola e 642 mila sul continente; texture e mesh restavano
  anche in RAM dopo il caricamento sulla GPU;
- **lavoro inutile a ogni fotogramma**: componenti e materiali venivano riscritti anche quando il valore non
  cambiava, quindi venivano ricaricati sulla GPU e facevano scattare i controlli `is_changed()` a valle;
- **ombre sprecate**: lampioni, stufa e lume calcolavano le ombre anche da spenti.

Il vincolo era di non cambiare nulla di quello che si vede e si gioca. Si è misurato prima e dopo con i
53 copioni del pilota automatico (`scripts/beta_check.sh`), con le foto confrontate a
`prove/riferimento/`. Il rapporto di partenza è in `docs/perf/baseline-RAPPORTO.md`.

## Decisione
- **Terreno** (`terrain.rs`):
  - gli strati tengono solo i vertici usati dai loro triangoli, con gli stessi triangoli nello stesso ordine
    e gli stessi valori;
  - restano anche i vertici estremi della griglia, senza triangoli. Bevy ordina le superfici trasparenti dal
    centro del riquadro della mesh: senza questi vertici l'ordine fra mare, schiuma e strati cambiava, e il
    lago delle palafitte sul continente si vedeva diverso. Un test controlla che il riquadro sia lo stesso;
  - quote, pendenze, sabbia, recinto, tangenti e colori si calcolano su tutti i core (`src/par.rs`, a
    blocchi: risultato identico al calcolo in serie);
  - il palo dei ponti è una mesh sola condivisa.
- **Solo GPU** (`RenderAssetUsages::RENDER_WORLD`):
  - terreno, strati, mare, ruscello e schiuma, mesh unite della natura, mappa delle onde e cubemap del cielo;
  - texture PBR (`pbr::load_tex`, `pbr::load_gpu`), texture degli alberi e della casetta;
  - restano in RAM solo le cose rilette dalla CPU: l'heightmap, la panoramica del cielo (tolta subito dopo
    la conversione in cubemap) e i prototipi degli abeti.
- **Condivisione**: cartelli e insegne usano mesh comuni e un materiale per texture.
- **Avvio su più core**, con lo stesso risultato e nello stesso ordine:
  - riquadri della natura uniti in parallelo, poi creati in serie come prima;
  - modelli degli edifici costruiti in parallelo;
  - le sei facce della cubemap del cielo calcolate in parallelo.
- **Scritture solo al cambio di valore**:
  - luci della casetta, stufa, lampioni, lampadine, fumo e coperchio di rame;
  - onde di mare e ruscello, velocità delle animazioni, fiamme delle torce, peso trasportato, testo
    dell'indicazione;
  - particelle di pioggia e neve (in parallelo) e foglie che cadono.
- **Ombre**: i lampioni proiettano ombre solo se accesi davvero; stufa e lume solo da accesi.
- **Missioni**: `Missions` risulta cambiato solo quando cambia qualcosa (tappe, scelte, contatori), non più
  a ogni fotogramma.
- **Abitanti**:
  - il collisore si spegne e si riaccende solo al cambio di stato;
  - i punti dei moli sono in una cache, rifatta quando cambiano i piazzamenti;
  - niente copie di nomi e chiavi a ogni fotogramma.
- **Fumetti**: i raggi di occlusione si lanciano in parallelo.
- **Scheduling**: salvataggio, shell e stato del pilota girano solo quando c'è una richiesta (`run_if`), e
  così non bloccano gli altri sistemi a ogni fotogramma.
- **Log**: i movimenti del mouse e del cursore passano a `debug!`; tasti, click e rotella restano `[INPUT]`.

### Misure (53 copioni, stessa macchina: Core Ultra 9 285K, RTX 3080 Ti)
| Misura | Prima | Dopo |
|---|---|---|
| FPS medio, media dei copioni | 46,0 | 54,7 (+19%) |
| 1% peggiore, media dei copioni | 28,9 | 32,0 |
| Copione più lento (`edifici`), FPS medio / 1% peggiore | 21 / 10 | 29 / 16 |
| `fps` (2560x1440), FPS medio / 1% peggiore | 41 / 21 | 49 / 27 |
| RAM massima, media dei copioni | 3383 MB | 2798 MB (−17%) |
| VRAM totale massima, media dei copioni | 5514 MB | 4858 MB (−12%) |
| VRAM in `edifici` / `viaggio-nord` | 9942 / 9057 MB | 7587 / 7449 MB |
| RAM in `viaggio-nord` | 4057 MB | 2010 MB |
| Cambio di regione (fotogramma più lungo) | 574–924 ms | 271–310 ms |
| Fotogrammi lenti segnalati (> 100 ms) | 13 | 8 |

- RAM: massimo della memoria residente del processo, campionato ogni secondo.
- VRAM: totale della scheda letto con `nvidia-smi`, compresi circa 650 MB usati dal desktop.
- I valori dei singoli copioni oscillano di qualche fotogramma da una corsa all'altra; le medie sui 53 copioni
  sono stabili.

## Conseguenze
- Stesso gioco: tutti i copioni passano e le foto restano entro la soglia come prima. Le foto già fuori soglia
  prima del lavoro (tre della piazza in `giro-camera`, la spiaggia del risveglio, la fonte sotto la quercia)
  restano com'erano; la foto della folla al molo (`il-paese-al-molo`, dopo `aspetta 120`) oscilla fra 0,10 e 0,13 da una
  corsa all'altra: la folla spinge il giocatore in punti diversi a seconda della frequenza dei fotogrammi, e
  l'inquadratura si sposta tutta, mentre il paesaggio resta identico.
- `src/par.rs` è il punto unico per il calcolo in parallelo: `par::map` e `par::collect` restituiscono i
  risultati nell'ordine, quindi la generazione resta deterministica.
- Una mesh caricata con `RENDER_WORLD` non si può più rileggere dalla CPU. Chi vuole rileggerla deve usare
  `RenderAssetUsages::default()`, come fanno heightmap e prototipi degli abeti.
- Scartati, perché avrebbero cambiato resa o comportamento, o perché il guadagno non valeva il rischio:
  - fumo a gradini di trasparenza, cioè materiali condivisi per livello di alfa (cambia la resa);
  - `file_watcher` spento: resta, serve al ricaricamento a caldo;
  - terreno generato in modo asincrono: cambierebbe la sequenza di caricamento letta dai copioni;
  - sistemi del mondo fermi in `Paused`: oggi il tempo virtuale non si ferma;
  - un solo `FirAssets` per isola e natura, e `apply_season` deduplicato per handle: le tinte delle stagioni
    dipendono dall'ordine delle scritture;
  - raggi dei gabbiani più radi e raggi di occlusione solo vicino: cambiano il comportamento di un fotogramma;
  - `keep_shadow_lights_extracted` limitato alle luci visibili: aggira un panic di Bevy e va lasciato com'è.
