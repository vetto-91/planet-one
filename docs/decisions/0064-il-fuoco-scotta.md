# ADR-0064: Il fuoco scotta

- Stato: accettato
- Data: 2026-11-27
- Versione: v0.0.71.0

## Contesto

Nella prova della v0.0.68.0 su Windows (segnalazione `2026-11-27_12-37-35`, falò nella grotta delle
stele, tappa mp04) si poteva stare in piedi dentro il fuoco senza conseguenze: «il fuoco non fa danno».

## Decisione

- Stare con i piedi nelle fiamme di un **falò** (`village_life::Campfire`, anche quelli dentro i luoghi
  e le gallerie) o di un **focolare acceso** delle case (`world_props::HearthFlame` con il suo
  `PropState` acceso) toglie `BURN_DAMAGE` = 4 punti di vita, subito e poi ogni `BURN_EVERY` = 0,8 s
  finché si resta dentro (`village_life::burn_player`).
- Il raggio delle fiamme è 0,45 m per i falò e 0,35 m per i focolari, più la larghezza del corpo;
  le fiamme valgono da poco sotto il loro centro fino a 0,9 m sopra (saltandoci sopra non si brucia).
- Ogni scottatura fa uno sfrigolio sintetizzato (`sfx::Sfx::Burn`) e l'avviso «Ti stai bruciando!»
  sotto la bussola; il log scrive `[FUOCO]`.
- Con `god` il fuoco non fa danno. La stufa di ghisa della casetta del Viandante resta esclusa: è
  chiusa e il suo collider non lascia entrare nelle fiamme.

## Conseguenze

- Un fuoco nuovo che deve scottare si crea con `village_life::spawn_campfire` (o un focolare con
  `HearthFlame`); le fiamme decorative di altre scene (torce, lanterne) non fanno danno.
- Il danno è piccolo apposta: avvisa senza uccidere chi ci passa sopra per sbaglio.
- Copione del pilota: `prove/beta-068-fuoco.txt`.
