# Versioning — Planet One

Formato a 4 segmenti: **`vMAJOR.MINOR.FEATURE.BUGFIX`**

Esempio: `v0.0.1.2` = nessun major, nessun minor, 1 feature, 2 bugfix.

| Segmento | Quando incrementa | Reset segmenti inferiori |
|---|---|---|
| **MAJOR** | Breaking change del gameplay/engine o milestone di rilascio (es. demo completa) | sì, tutti |
| **MINOR** | Nuova fase macro della roadmap (es. Fase 1 Movimento completata) | FEATURE e BUGFIX |
| **FEATURE** | Nuova funzionalità aggiunta all'interno di una fase | BUGFIX |
| **BUGFIX** | Correzione di un bug senza nuove feature | — |

## Regole

1. Si parte da `v0.0.0.1` (prima finestra eseguibile).
2. I tag Git sono l'unica fonte di verità per la 4ª cifra.
3. `Cargo.toml` riporta solo i primi 3 segmenti (limite di semver di cargo); il 4° è tracciato nei tag e nel `CHANGELOG.md`.
4. Pre-release: `v0.0.1.0-rc1`; build metadata: `v0.0.1.0+build.42`.
5. Tag solo su `main`, dopo verifica manuale della build.

## Milestone previste

- `v0.0.0.1` — Fase 0: bootstrap, finestra + scena base
- `v0.0.1.0` — Fase 1: movimento WASD + camera follow
- `v0.0.2.0` — Fase 2: scena, terreno, cel-shading
- `v0.0.3.0` — Fase 3: NPC, dialoghi, nemico
- `v0.0.4.0` — Fase 4: UI, inventario, pausa
- `v0.0.5.0` — Fase 5: vertical slice "Denaro Rubato"
- `v0.0.6.0` — rifiniture: diario, giorno/notte, skin, UI condivisa
- `v0.1.0.0` — prima demo completa (game over/vittoria, salvataggio)

## Branch e commit

- Branch: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`
- Commit: `feat(area): descrizione`, `fix(area): descrizione`, `chore(area): descrizione`
- Un commit per feature/fix, messaggio in inglese (codice) con descrizione chiara.

## Checklist di rilascio

1. Aggiornare `version` in `Cargo.toml` (primi 3 segmenti).
2. Aggiornare `GAME_VERSION` in `src/version.rs` (mostrata nel menu Esc).
3. Spostare la sezione "Non rilasciato" di `CHANGELOG.md` sotto la nuova versione.
4. Commit, poi tag `vX.Y.Z.W` su `main` dopo la verifica manuale.
