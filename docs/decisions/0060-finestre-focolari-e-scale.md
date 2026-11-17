# ADR-0060: Finestre vere, focolari aperti e scale a L

- Stato: accettato
- Data: 2026-11-17

## Contesto
La beta della v0.0.63 ha segnalato tre difetti di costruzione negli edifici del paese:
- **finestre finte** (segnalazioni 33 e 34): il vetro era un quadrato colorato dipinto sul muro pieno,
  fuori, e dentro c'era un pannello chiaro. Non ci si vedeva attraverso e non si apriva, a differenza della
  casetta del Viandante. Il vetro della casetta, a sua volta, era azzurrino e lattiginoso;
- **focolari tappati** (32): un blocco di pietra pieno con un rettangolo arancione davanti, senza fuoco;
- **scale delle mura** (30): partivano da terra contro la testa del muro pieno; ci si saliva solo di lato.

Vincoli:
- non perdere i guadagni di FPS degli ADR 0056–0058;
- le prove esistenti restano valide, in particolare `windows_are_clear_and_chimneys_sit_on_hearths` e
  `roofs_are_closed_seen_from_inside`;
- salvataggi compatibili.

## Decisione

### Finestre
- **Vano vero nella mesh**: `holed_wall` taglia i muri di fronte e retro attorno ai vani
  (`WINDOW_SIZE` 0,62 × 0,86 m). Fra le fasce piene di ogni colonna si fa un solo blocco. Telaio di legno
  sul filo esterno, davanzale di pietra fuori e di legno dentro.
- **Il collider resta il muro intero.** Il vano ha comunque il vetro, e così la camera non passa dalla
  finestra e i controlli di linea di vista (interazioni, combattimento) restano quelli di prima. Non si
  aggiunge nessun collider.
- **Dentro, come nella casetta** (`hut.rs`): vetro scorrevole (`PropKind::WindowGlass`, `Slider`, id
  `vetro:<luogo>:<punto>:<n>`, salvato come gli altri oggetti) e maniglia interna degli scuri
  (`WindowInner` + `WindowLink`), che funziona solo col vetro alzato. Gli scuri da fuori restano
  `scuri:…`, ora usabili solo dal lato esterno (`use_from`). Gli abitanti e l'orario continuano a
  comandare gli scuri come prima.
- **Il vetro sta a metà del muro** e, alzato, sale dentro l'architrave: sparisce senza animare altro.
- **Vetro naturale** (`pbr::glass_material`, anche per `Mat::Glass`): quasi incolore (alfa 0,08), liscio
  (rugosità 0,04), riflettente (0,45), a due facce. Lo usano case, casetta e faro.

### Costo
- Tutti i vetri hanno la stessa mesh e lo stesso materiale e non fanno ombra.
- Oltre i 45 m non si disegnano (`VisibilityRange`): da lontano un vetro trasparente non si nota.
- **`animate_hinges` e `animate_sliders` scrivono solo quando qualcosa si muove.** Prima ogni imposta e
  porta del paese riscriveva la rotazione a ogni fotogramma, quindi ogni volta si ripropagavano le
  trasformazioni. Inoltre ogni porta inseriva o toglieva `ColliderDisabled` a ogni fotogramma.
- Misure: vedi sotto.

### Focolari
- `hearth_back` costruisce piano, stipiti, fondo e cappa attorno a una **bocca vera** (larghezza − 0,5 m,
  alta 0,85 m). Ha la fuliggine sul fondo, i ceppi e un architrave di legno.
- Il collider resta il blocco intero (`furniture`): nel fuoco non si entra, e la vista non si blocca.
- **Fiamma viva** (`world_props::HearthFlame`): la stessa sfera emissiva dei falò
  (`village_life::spawn_campfire`). È visibile e tremola solo quando il focolare è acceso. Non ha luce
  sua: il bagliore resta `HearthLight`.
- `spawn_campfire` e `flicker_fires` non si riusano così come sono: accendono sempre la loro luce e il
  fumo. La fiamma si aggiorna dentro `update_interior_lights`, per non registrare un sistema nuovo.

### Scale delle mura
- Ogni scala comincia con un **pianerottolo** di 1,5 m nella corsia interna, contro la testa del muro. Da
  lì una **rampa corta di tre gradini, perpendicolare al muro**, scende verso il paese.
- La rampa lungo il muro parte dal pianerottolo. `stair_zones` copre pianerottolo e rampa: il muro resta
  sottile e senza cordolo su tutto il tratto, come prima.
- La rampa perpendicolare sporge di circa 1,1 m verso il paese.

### Altre correzioni (stesse segnalazioni)
- **Faro** (45, 46):
  - la prima rampa sale dal fronte verso il fondo;
  - la chiocciola ha una cella d'arrivo (`square_helix_ending`, cella 9, fronte-sinistra), quindi parte
    dall'angolo opposto e passa sopra il vano della rampa solo quando è alta;
  - l'uscita della lanterna è davanti all'arrivo;
  - i vetri della lanterna stanno appena fuori dal pozzo della scala;
  - `square_helix` resta uguale per chiesa, torre in rovina e Camera di Risonanza.
- **Sottotetto** (31, 37):
  - i timpani visti da dentro seguono la pendenza delle falde interne: niente più fessura sopra la scala;
  - la fascia sotto la gronda e il muretto si fermano alla rampa sul retro, e anche sul davanti quando la
    rampa arriva a meno di 45 cm dal muretto (case piccole).
- **Stalla** (42): `Door.leaves = 2`. Il portone di 2,4 m si apre verso fuori, su cerniere sulla faccia
  esterna del muro.
- **Pozzo e pozza** (5): l'acqua è limpida sotto il bordo (`buildings::clear_water_material`, ricavato da
  quello del ruscello, senza onde). Per il pozzo si usa `Interior.water`.

## Misure
Sulla RTX 3080 Ti a 1080p, senza finestra, la v0.0.66.0 e questo ramo si sono alternati con lo stesso
copione. Mentre si misurava, la scheda era condivisa con le prove di altre sessioni, quindi i numeri
ballano di ±15 FPS da una corsa all'altra.

| Copione | v0.0.66.0 | con questo ADR |
|---|---|---|
| `fps`: 3 corse alternate, 8 luoghi di giorno e di notte, media | 34,5 | 36,0 |
| `fps`: 2 corse alternate (rapporto del pilota), media | 39,5 | 35,5 |
| `edifici`: 2 corse alternate, media | 18,5 | 21,0 |

Dentro il rumore non c'è una differenza misurabile. Le entità passano da 18 148 a 18 552 (+404: maniglie
dei vetri e degli scuri interni, vetri e fiamme). Una corsa aveva dato un calo di notte al faro e al
mulino (11–16 contro 32–34), ma nelle tre corse dopo non si è ripetuto (28–41 contro 18–40).

## Conseguenze
- Le foto di riferimento di `edifici` cambiano: ci sono i vani delle finestre.
- I salvataggi vecchi non hanno gli stati `vetro:…`: i vetri partono abbassati.
- Prove nuove:
  - `walk_test::the_attic_can_be_reached_on_foot`;
  - `the_lighthouse_ground_flight_leads_to_the_spiral`;
  - `roofs_are_closed_seen_from_inside`, ora anche nel sottotetto delle case a due piani (i raggi che
    escono da un vano di finestra non contano).
- Copioni `prove/beta-*.txt`, uno per segnalazione riprodotta.
