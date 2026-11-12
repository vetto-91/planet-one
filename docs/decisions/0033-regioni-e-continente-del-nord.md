# ADR-0033: Regioni del mondo e Continente del Nord

- Stato: accettato
- Data: 2026-11-12

## Contesto
La trama (`Trama-Planet-One/`, documenti 06 e 10) porta il giocatore dall'Isola di Fontevivo al Continente
del Nord. L'Atto I si svolge nella Piana delle Brume: circa 1.000 × 1.200 m (13 volte l'isola), con
l'Approdo, il bosco di querce e betulle, il Grande Lago, il paese di Fontebruma e 16 luoghi.
Il gioco caricava un solo mondo di 300 × 300 m, tutto insieme, ed era già limitato dal processore.

## Decisione
- **Regioni separate** (`src/region.rs`): `Isola` e `Nord`. Se ne carica una alla volta; la regione corrente
  è globale (la leggono anche funzioni che non sono sistemi). Ogni regione ha la sua misura e il suo passo
  del terreno (isola 300 m a 0,5 m; continente 1.200 m a 1,5 m, cioè 801² vertici contro 601²), i suoi dati
  (`assets/world/<file>` e `assets/world/nord/<file>`) e i suoi piazzamenti (`debug/placements-nord.txt`).
- **Viaggio**: `save::TravelRequest` fotografa la partita (`SaveData`), ricorda lo stato del mondo che si
  lascia (`RegionMemory`: porte, contenitori, oggetti a terra, nemici, abitanti) e ricostruisce la scena
  nell'altra regione (`scene::reset_scene`); il personaggio porta con sé inventario, quest, data e ora.
  Il salvataggio scrive la regione e, in sezioni `[regione:<nome>]`, lo stato delle altre regioni visitate.
- **Accesso**: per ora libero. Gaspare, al porticciolo, offre «Portami al Continente del Nord» e aspetta
  all'Approdo delle Brume per il ritorno; dalla shell `regione <isola|nord>`. Il blocco di trama (missioni
  del Prologo fino a MP-05) si aggiungerà con quelle missioni.
- **Terraferma** (`terrain::continent_height`, dati in `nord/isola.txt`): costa a sud (`costa:`), confine
  percorribile con crinali e barriera invisibile (`confine:`), Grande Lago e Rio delle Betulle sotto il
  livello del mare (`lago-grande:`, `canale:`): li allaga lo stesso riempimento del mare, così riva, sabbia
  e nuoto funzionano senza un secondo sistema d'acqua. Il paese sta in piano a una quota sola.
- **Vegetazione a zone** (`nature::stream_nature`): sul continente i riquadri di 30 m si piantano attorno
  al giocatore (raggio 6 riquadri, 3 per volta) e spariscono oltre 8; sull'isola resta tutto piantato insieme.
  Boschi nuovi: `querce`, `betulle`, `querce-betulle` (modelli già presenti: le querce sono la latifoglia ingrandita).
- **Dati generati** da `scripts/gen_nord.py` da un'unica tabella di coordinate: forma della regione,
  Fontebruma (mura, porte, strade, edifici), sentieri, i 16 luoghi. Luoghi nuovi: `arco`, `carbonaia`, `palafitta`.
- **Clima**: nella Piana delle Brume, senza meteo forzato, c'è nebbia con pioggia sottile (neve d'inverno).

## Conseguenze
- Il primo traguardo è la **regione esplorabile**: abitanti (a parte Gaspare), negozi, missioni, eventi,
  stati dei luoghi sulla mappa, viaggio veloce e bottino che si rigenera arrivano nei passi successivi.
- Le mura di Fontebruma sono di pietra come a Fontevivo (la trama parla di palizzata).
- Sulla mappa del continente il bosco si vede solo attorno al giocatore (è piantato a zone).
- Il lago ha le onde del mare e la stessa acqua trasparente.
- Aggiungere un copione in `prove/` sposta le risoluzioni assegnate a quelli che lo seguono in ordine
  alfabetico: `viaggio-nord` precede solo `vita-paese`, che non ha foto di riferimento.
