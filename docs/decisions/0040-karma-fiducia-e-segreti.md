# ADR-0040: Karma, fiducia e segreti degli abitanti

- Stato: accettato
- Data: 2026-11-13

## Contesto
La trama (`Trama-Planet-One/02`, sezione 4.5, e `04`, sezione 7) vuole un karma a due livelli — l'impronta del
personaggio nel mondo e la fiducia di ogni abitante — e un segreto personale per ciascuno dei 26 abitanti
dell'isola. Finora rubare, risparmiare il ladro o tenersi le monete non cambiava nulla.

## Decisione
- **Karma** (`src/karma.rs`, `Karma.global`, da −100 a 100, «la Bilancia di Ma'at»). Lo muovono:
  risparmiare il ladro (+10), restituire tutte le monete (+5), una missione compiuta (principale +4,
  secondaria +3, evento +1), un dono infilato di nascosto (+2); dividere l'oro col ladro (−10), tenere
  monete non proprie (−8), colpire il ladro mentre ascoltava (−6), farsi cogliere a borseggiare (−4),
  derubare qualcuno (−3). Ogni variazione compare a schermo col motivo.
- **Fiducia** per abitante (da −50 a 100): Diffidente (<0), Sconosciuto, Conoscente (10), Amico (30), Anima
  affine (60). Sale con la prima chiacchierata del giorno (+3), un regalo gradito (+6, una volta al giorno,
  tasto `G` di fronte a lui), ogni tappa di missione che passa da lui (+5), ogni acquisto (+1); scende di 15
  se ti coglie con le mani in tasca. La sorte del ladro e la consegna delle monete toccano Don Anselmo e il Viandante.
- **Segreti** (`assets/world/segreti.txt`, modificabile a mano): per ognuno dei 26 abitanti che cosa gli è
  caro, il dono che gradisce, un riassunto e la confidenza. L'abitante la dice al posto del saluto quando la
  fiducia arriva ad «Amico» (via del cuore), oppure già a 22 se una fra Carisma, Saggezza e Intelligenza è
  almeno 14 (via dell'intuito). Una battuta di missione ha sempre la precedenza.
- **Effetti**: i prezzi seguono il karma (da −6% a +15%) e la fiducia del venditore (amico −5%, anima affine
  −10%, diffidente +10%). Nella chat con l'IA locale l'abitante sa quanto si fida di te e, dopo la
  confidenza, può parlare del suo segreto (riga aggiunta al contesto del prompt; sempre e solo in locale).
- **Diario**: terza scheda «Volti e segreti» con la bilancia, gli abitanti incontrati, il legame, ciò che è
  loro caro (da Conoscente in su) e il segreto scoperto, o un cenno che ne custodiscono ancora uno.
- Come per l'esperienza, chi agisce chiama `karma::deed` e `karma::trust` (coda globale).
- Salvataggio: sezione `[karma]`. Shell: `karma [n]`, `fiducia <abitante> <n>`, `davanti <abitante>`, `volti`.

## Conseguenze
- Delle tre vie della trama ci sono il cuore e l'intuito; manca la via degli indizi (lettere, diari e oggetti
  nei bauli, pedinare un abitante la sera).
- I segreti si raccontano ma non sbloccano ancora nulla: niente oggetti (campanaccio, bulbo, mappa delle
  correnti), compagni o missioni che ne discendono.
- Ogni abitante apprezza le stesse azioni: i valori personali della trama («gli è caro») sono solo descritti.
- Il karma non cambia i saluti, non chiude porte e non tocca i finali; prendere roba dai mobili delle case
  non pesa. Le guardie non reagiscono ai furti.
- Le confidenze non hanno voce registrata. Un avviso successivo (un grado guadagnato) può coprire quello
  della confidenza.
- Vale solo per l'isola: gli abitanti del continente non hanno ancora un segreto.
