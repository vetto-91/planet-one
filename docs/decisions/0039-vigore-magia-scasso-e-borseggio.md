# ADR-0039: Vigore, magia, scasso e borseggio

- Stato: accettato
- Data: 2026-11-13

## Contesto
Dopo le abilità che crescono con l'uso (ADR 0038) restavano fuori Scassinare e Borseggio, con la missione che
li insegna (`Trama-Planet-One/09`, QS-16). In più la sola barra della vita, piccola e in un angolo, non bastava
più: servono il vigore (lo sforzo) e la magia (la riserva per gli incantesimi che verranno), in uno stile
coerente con i pannelli.

## Decisione
- **Vigore e magia** (`src/vitals.rs`, `Vitals`). Il vigore massimo è 100, più 5 per punto di Costituzione e 2
  per punto di Agilità oltre il 10; la magia 50, più 5 per Intelligenza e 3 per Saggezza. Il vigore cala
  correndo (9 al secondo), saltando (7) e a ogni colpo (5); dopo un secondo di respiro risale di 16 al secondo.
  A zero si resta **senza fiato**: niente corsa finché non se ne recupera un quarto. La magia per ora si riempie
  soltanto. Chi consuma chiama `vitals::spend`, come per l'esperienza. Con `god` il vigore non cala.
- **Barre** (`hud::reserve_bar`): tre barre subito sopra la barra degli oggetti e larghe quanto lei: magia a
  sinistra, vita al centro (più larga), vigore a destra. Fondo scuro e bordo d'oro dei pannelli, nome a
  sinistra e valori a destra; il vigore si spegne quando si è senza fiato. La vecchia barra della vita
  nell'angolo non c'è più. I riquadri della barra degli oggetti sono ora davvero centrati.
- **Scasso** (`src/thievery.rs`): i bauli sono chiusi a chiave finché non li si apre una prima volta (non
  quello del Viandante; semplice nelle case, robusta in caserma, antica nella grotta). Senza grimaldello resta
  chiuso. Col grimaldello (oggetto nuovo, 6 monete da Ferruccio) si apre lo stato `Lockpick`: un ago corre
  avanti e indietro e va fermato sulla tacca d'oro, una volta per perno (3, 4 o 5). Uno sbaglio fa ricadere un
  perno e può spezzare il grimaldello. La tacca si allarga col grado di Scassinare, e i grimaldelli reggono di più.
- **Borseggio**: a un passo e alle spalle di un abitante, `F` gli sfila da 2 a 8 monete (una volta al giorno) e
  `G` gliene infila 5 di nascosto (il «borseggio inverso»). La riuscita cresce col grado (30% al grado 1, certa
  al 100), di notte e infilando invece di sfilare. Se va male l'abitante se ne accorge: 10 monete di multa e
  niente tasche fino al giorno dopo.
- I tiri sono **deterministici** (dipendono dal numero di tentativi), così le prove automatiche non ballano.
- **Missione QS-16** «Il Baule Senza Chiave e la Mano Leggera»: Ferruccio dà tre grimaldelli, si aprono tre
  bauli, Guardia Carlo paga, Osvaldo affida 15 monete da infilare nella tasca di Beppe. Il motore delle missioni
  guadagna `quando: conta <chiave> <n>` (`serrature`, `infilate:<nome>`). Le secondarie sono 15.
- Salvataggio: vigore e magia in `[giocatore]`, serrature e monete infilate in `[furti]`. Un baule aperto resta
  aperto perché il suo contenuto è già nel salvataggio. Shell: `dietro <abitante>`, `scassina`.

## Conseguenze
- Scassinare e Borseggio ora si allenano e hanno un talento; restano ferme Armatura, Magie e Costruire.
- La magia non si spende ancora: la barra c'è, gli incantesimi no.
- Nuotare non costa vigore; i nemici non ne hanno.
- Il borseggio guarda solo da che parte è girato l'abitante: gli altri presenti non vedono nulla, e non c'è
  ancora il karma a pesare il furto. Non si borseggiano i nemici (il ladro, i banditi) né oggetti diversi dalle monete.
- Le porte chiuse a chiave non si scassinano: solo i bauli.
- Chi aveva una partita in corso trova chiusi i bauli mai aperti.
- Tutte le foto di riferimento delle prove cambiano (HUD nuovo).
