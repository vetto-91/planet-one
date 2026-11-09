# ADR-0017: Animazioni dei personaggi

- Data: 2026-11-09
- Stato: accettato (Fase 3 della grafica realistica)
- Versione: v0.0.28.0

## Contesto
I personaggi erano fermi nella posa di riposo: chi camminava scivolava, l'attacco era solo un'oscillazione dell'arma.

## Decisione
- `src/animation.rs`: un componente `Animator` sul corpo, inserito quando il modello è pronto, per giocatore, abitanti e nemici.
- La velocità orizzontale si misura dallo spostamento del corpo (filtrata; i teletrasporti non contano), così vale per ogni routine senza modificarla. La clip si accelera in proporzione: camminata degli NPC (1,3 m/s), corsa lenta (giocatore, 6 m/s), corsa (10 m/s).
- Clip singole (attacco, colpito, atterraggio) con precedenza e blocco fino alla fine. Mentre si attacca le gambe restano ferme nella clip (nessuna maschera per il busto).
- In volo solo se senza appoggio **e** con movimento verticale recente: su oggetti senza marker `Ground` non si resta nella posa di salto.
- Nemici: guardano dove vanno e colpiscono entro 1,8 m. Alla morte resta un corpo senza collider che esegue `Death01` e sparisce dopo 6 s, senza toccare logica di quest e respawn.
- Il nemico generico (cubo rosso) usa lo stesso modello del ladro.
- Opzione grafica "Ferma le animazioni lontane": oltre 60 m dalla camera l'animazione è in pausa.

## Conseguenze
- La velocità del giocatore (6–10 m/s) è molto alta per la scala umana: le clip di corsa sono accelerate, il passo resta rapido.
- Durante l'attacco i piedi possono scivolare se ci si muove.
- Correzione collegata: il colpo in mischia usava la direzione del corpo fisico, che non ruota mai; ora usa quella del modello.
