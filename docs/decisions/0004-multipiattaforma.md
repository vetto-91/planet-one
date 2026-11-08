# ADR-0004: Porting Windows e macOS con build in CI

- Data: 2026-11-08
- Stato: accettato
- Versione: v0.0.14.0

## Contesto
Il gioco usciva solo per Linux x86_64, con il tarball costruito in locale. Si vogliono
supportare anche Windows e macOS, che non si possono compilare né provare dalla macchina
di sviluppo (Linux).

## Decisione
- Il codice resta unico e portabile: percorso delle impostazioni per piattaforma
  (`%APPDATA%`, `~/Library/Application Support`, `$XDG_CONFIG_HOME`/`~/.config`), cursore
  `Confined` su Windows (`Locked` non supportato) e `Locked` altrove, niente finestra di
  console in release su Windows, piazzamenti di `debug/placements.txt` incorporati con
  `include_str!` come fallback (i pacchetti non hanno la cartella `debug/`).
- Gli archivi Windows (`.zip`, x86_64) e macOS (`.tar.gz`, binario universale Intel +
  Apple Silicon creato con `lipo`) sono costruiti da GitHub Actions
  (`.github/workflows/release.yml`) a mano (`gh workflow run release.yml -f tag=… -f windows=true -f macos=true`, scelta per sistema operativo per contenere i costi) e allegati alla release. Linux
  resta costruito in locale come prima.
- Pacchetti non firmati (niente notarizzazione Apple né firma Authenticode): il README
  spiega come superare Gatekeeper e SmartScreen.

## Conseguenze
- Per un repo privato i minuti macOS costano 10× e Windows 2× (circa 100 min a release di
  macOS sul piano gratuito).
- Windows e macOS non sono provati dallo sviluppatore su queste piattaforme: i problemi
  emergono dal test degli archivi della CI.
- La procedura di release attende la CI prima di sincronizzare Drive.
- Un eventuale bundle `.app`, firma e notarizzazione restano lavoro futuro.
