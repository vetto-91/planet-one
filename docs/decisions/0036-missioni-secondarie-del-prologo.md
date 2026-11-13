# ADR-0036: Missioni secondarie del Prologo

- Stato: accettato
- Data: 2026-11-13

## Contesto
La trama (`Trama-Planet-One/09`, sezione 3) elenca 17 missioni secondarie sull'isola. Il motore delle
missioni a tappe (ADR 0034) reggeva solo la storia principale.

## Decisione
- Le secondarie stanno in `assets/quests/prologo/secondarie.txt`, stesso formato, con `tipo: secondaria`:
  la missione entra nel diario solo dopo la prima tappa, cioè quando l'abitante che la propone ha parlato.
- Il motore guadagna: `quando: ha <oggetto> <n>`, `quando: libero <x> <z> <raggio>` (nessun nemico vivo),
  `prendi:` (oggetto consegnato) e `dai:` (oggetto ricevuto). Gli oggetti si chiamano col nome italiano
  dell'inventario («pesce», «mela», «scudo», «pozione del mulo»).
- Un dialogo di missione vale per **una sola missione**: se due incarichi chiedono lo stesso abitante,
  il secondo aspetta la prossima volta che gli si parla (prima viene la storia principale).
- **Undici missioni** scritte con ciò che il gioco ha già: Il Pane del Vento (qs01), La Luce che Risponde
  nella Nebbia (qs02), Lo Scudo dell'Orlo Spezzato (qs03), Tre Nomi per le Ombre (qs04), Il Pascolo del Sole
  (qs05), L'Anfora dei Cent'Anni (qs06), Il Compleanno di Pietro (qs07), Il Sogno della Farfalla (qs08),
  La Promessa sulla Torre Antica (qs10), Il Segreto della Porpora (qs11), I Fili Sotto le Pietre (qs13).
- Copione `zz-secondarie`: cinque missioni giocate per intero (qs01, qs03, qs04, qs06, qs10).

## Conseguenze
- **Sei missioni restano da fare** perché chiedono sistemi che non esistono: il duello d'allenamento e il
  nuoto di Guardia Dino (qs09), le quattro ronde sulle mura e il campanile (qs12), la prova dell'arco con i
  bersagli (qs14), il cantiere della nave di tutti (qs15), scasso e borseggio (qs16), la costruzione della
  casa (qs17).
- Rispetto alla trama le undici sono semplificate: niente scelte finali (lo scudo si tiene, i tre nomi si
  pronunciano), gli oggetti di trama (disco, campanaccio, bussola, registro, gemma) sono raccontati nei
  pensieri e non stanno nell'inventario, la mucca non si segue davvero, i premi sono monete e oggetti esistenti.
- Le battute non hanno voce registrata. Sei missioni (qs02, qs05, qs07, qs08, qs11, qs13) non sono coperte
  da un copione: le controlla solo il test sul formato del file.
