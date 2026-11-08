# ADR-0011: Calendario, velocità del tempo e sole stagionale (Fase 1)

- Data: 2026-11-08
- Stato: accettato (Fase 1 di 3; stagioni visibili e meteo seguono)
- Versione: v0.0.21.0

## Contesto
Il gioco aveva solo l'ora (1 h = 60 s) e il sole sempre uguale. Servono data, stagioni e, in seguito, meteo; si procede a fasi.

## Decisione
- `GameDate` (anno, mese, giorno) dentro `GameClock`: parte dal **21 marzo anno 0**, mesi reali, 365 giorni, nessun anno bisestile. Stagioni astronomiche dell'Europa
  centrale: primavera 21/3, estate 21/6, autunno 23/9, inverno 21/12.
- Sole: declinazione `23,44° · sin(2π(N−80)/365)` e latitudine 48°; all'equinozio sorge alle 6 e tramonta alle 18, d'estate è più alto e i giorni più lunghi
  (alba ≈ 4:05), d'inverno più basso (alba ≈ 7:55). `sun_dir(giorno, ora)` e `sunrise_sunset(giorno)` in `daynight.rs`.
- `GameClock` espone alba/tramonto e le finestre derivate (`awake`, `lamps_on`, `lighthouse_dark`, `guard_lanterns_on`) usate da cancelli, Viandante,
  lampioni, faro e lanterne delle guardie al posto di ore fisse. Orari degli abitanti (`abitanti.txt`) e campane restano a ore fisse.
- Velocità del tempo ×1/×2/×4/×8 nel menu Esc, salvata in `settings.txt` (`time_speed`). "Aspetta" (T) fa avanzare anche i giorni.
- Debug: `PLANET_ONE_DATE=anno,mese,giorno`.

## Conseguenze
- Il sole è più basso che in precedenza (picco 42° all'equinozio): ombre più lunghe, da verificare a occhio.
- Le fasi successive (tinte stagionali, neve, foglie, meteo) useranno `GameDate::season()`.
