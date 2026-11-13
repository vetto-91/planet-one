# ADR-0038: Abilità che crescono con l'uso e livelli

- Stato: accettato
- Data: 2026-11-13

## Contesto
La trama (`Trama-Planet-One/02`, sezione 2.4, «Fai e Diventa») chiede che il personaggio cresca facendo:
ogni azione allena una delle sedici abilità, le abilità sbloccano talenti e fanno salire il livello generale,
che porta punti statistica e Nodi d'Oro. Finora c'erano solo le sette statistiche fisse (ADR 0024).

## Decisione
- **Sedici abilità** (`src/skills.rs`, `Skill`) con un grado da 1 a 100. Dal grado `g` al successivo servono
  `10 + 2g` punti d'esperienza. Undici si allenano già: Atletica (correre), Acrobazia (saltare), Nuoto,
  Tempra del Portatore (camminare con lo zaino almeno a metà), Lama (colpi in mischia), Difesa (colpi presi
  con lo scudo), Arco (frecce a segno), Furtività (stare vicino a un nemico che non ti vede), Erboristeria
  (pozioni), Eloquenza (parlare e comprare), Sapienza (luoghi scoperti, mobili aperti). Armatura, Magie,
  Borseggio, Scassinare e Costruire sono in elenco ma aspettano i loro sistemi.
- **Livello**: ogni grado guadagnato dà 10 punti di livello; una missione principale 60, una secondaria 40,
  un evento 20. Dal livello `L` al successivo servono `40 + 20L`. Ogni livello dà un punto statistica (si
  spende col «+» nel pannello C), un Nodo d'Oro e 5 punti di vita massima.
- **Talenti**: ogni abilità allenabile ha un talento con cinque ranghi (Novizio 15, Apprendista 30, Esperto 50,
  Maestro 75, Leggenda 100). Un rango si attiva con un Nodo d'Oro quando il grado arriva alla soglia; l'effetto
  è lo stesso a ogni rango e si somma (corsa +5%, salto +4% e caduta più alta senza danno, nuoto +8%, portata
  +3 kg, mischia +6%, scudo −10% di danno subito, frecce +8%, vista dei nemici −8%, pozioni +10%, prezzi −3%,
  esperienza di livello +5%).
- **Come arriva l'esperienza**: `skills::train(abilità, punti)` e `skills::reward(punti)` scrivono in una coda
  globale, svuotata una volta per frame da `apply_training`; `skills::rank(abilità)` legge i ranghi attivi da
  un vettore atomico. Così i sistemi esistenti (movimento, combattimento, negozio…) non cambiano firma.
- Stato nel salvataggio (`[abilita]`, sezione del personaggio: segue il viaggio fra regioni). Un salvataggio
  vecchio riparte dal grado 1.
- Shell: `abilita [<nome> <grado>]`, `livello [n]`, `talento <abilità>`. Pilota: `/abilita/<nome>`,
  `/abilita/talento_<nome>`, `/livello`, `/nodi`, `/punti_statistica`.

## Conseguenze
- Aggiornamento (ADR 0039): Scassinare e Borseggio ora si allenano e hanno un talento; le allenabili sono tredici.
- Un solo talento per abilità, con ranghi uguali: i talenti con nome della trama (scatto, disarmo, apnea,
  controparata…) chiedono meccaniche nuove e restano da fare.
- Il grado da solo non cambia nulla nel gioco: contano i ranghi attivati. Le statistiche collegate a ogni
  abilità nella trama non influiscono ancora sulla velocità di crescita.
- La corsa conta la strada comandata, non quella fatta: correre contro un muro allena lo stesso.
- La coda e i ranghi sono globali del processo: nei test i ranghi restano a zero (nessuno li pubblica).
- I numeri (punti per azione, soglie dei livelli) sono una prima taratura, da rivedere giocando.
