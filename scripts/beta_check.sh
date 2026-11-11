#!/usr/bin/env bash
# Esegue i copioni del pilota automatico (prove/*.txt) e scrive prove/out/RAPPORTO.md.
#
# Uso: scripts/beta_check.sh [--aggiorna] [--senza-schermo] [copione ...]   (senza copioni: tutti)
# Le foto di ogni copione si confrontano con prove/riferimento/<copione>/<foto>.png
# (ridotte a 480x270, ImageMagick `compare`, RMSE normalizzato; soglia SOGLIA=0.12). --aggiorna copia le foto
# di questa corsa come nuovi riferimenti.
# Ogni copione gira in una partita di prova isolata (PLANET_ONE_DATA_DIR in prove/out/dati),
# a finestra, alle 11 con il sereno; le variabili PLANET_ONE_* già impostate valgono.
# Un copione può chiedere variabili sue con righe «# env: CHIAVE=valore».
# Uscita: 0 se tutti i copioni passano, 1 altrimenti.
set -u
cd "$(dirname "$0")/.."
OUT=prove/out
mkdir -p "$OUT"
cargo build --release --offline >/dev/null 2>&1 || { echo "build fallita"; exit 2; }
update=0
while [ $# -gt 0 ]; do
  case "$1" in
    --aggiorna) update=1; shift ;;
    # Senza finestra: la camera disegna in un'immagine (serve quando lo schermo non c'è,
    # per esempio se è attiva un'altra sessione del PC). Foto senza HUD né pannelli.
    --senza-schermo) export PLANET_ONE_HEADLESS=1; shift ;;
    *) break ;;
  esac
done
SOGLIA="${SOGLIA:-0.12}"
scripts=("$@")
[ ${#scripts[@]} -eq 0 ] && scripts=(prove/*.txt)
report="$OUT/RAPPORTO.md"
{
  echo "# Rapporto del pilota automatico"
  echo
  echo "Data: $(date '+%Y-%m-%d %H:%M') · versione: $(grep -o 'v[0-9][0-9.]*' src/version.rs | head -1)"
  echo
  echo "| Copione | Esito | Verifiche ok | Fallimenti | WARN | ERROR | panic |"
  echo "|---|---|---|---|---|---|---|"
} > "$report"
failed=0
for s in "${scripts[@]}"; do
  name=$(basename "$s" .txt)
  log="$OUT/$name.log"
  rm -rf "$OUT/$name"
  # Variabili proprie del copione: righe «# env: CHIAVE=valore» (es. PLANET_ONE_SPEED=6).
  extra=$(sed -n 's/^# env: //p' "$s" | tr '\n' ' ')
  env $extra \
  PLANET_ONE_DATA_DIR="${PLANET_ONE_DATA_DIR:-$PWD/$OUT/dati}" \
  PLANET_ONE_SCRIPT="$s" \
  PLANET_ONE_SIZE="${PLANET_ONE_SIZE:-1280x720}" \
  PLANET_ONE_HOUR="${PLANET_ONE_HOUR:-11}" \
  PLANET_ONE_WEATHER="${PLANET_ONE_WEATHER:-sereno}" \
    timeout "${TIMEOUT:-600}" cargo run --release --offline > "$log" 2>&1
  code=$?
  clean=$(sed 's/\x1b\[[0-9;]*m//g' "$log")
  ok=$(grep -c "\[VERIFICA\] ok" <<< "$clean")
  ko=$(grep -c "\[VERIFICA\] FALLITA\|\[PILOTA\] vai .*bloccato" <<< "$clean")
  warn=$(grep -c " WARN " <<< "$clean")
  err=$(grep -c " ERROR " <<< "$clean")
  pan=$(grep -c "panicked" <<< "$clean")
  case $code in
    0) esito="✅ passato" ;;
    124) esito="⏱️ tempo scaduto"; failed=1 ;;
    *) esito="❌ fallito ($code)"; failed=1 ;;
  esac
  echo "| \`$name\` | $esito | $ok | $ko | $warn | $err | $pan |" >> "$report"
  echo "$name: $esito (ok $ok, fallimenti $ko, panic $pan)"
done
{
  echo
  echo "## Fallimenti"
  for s in "${scripts[@]}"; do
    name=$(basename "$s" .txt)
    f="$OUT/$name/RAPPORTO.txt"
    [ -f "$f" ] && grep -q "^- " "$f" && { echo; echo "### $name"; grep "^- " "$f"; }
  done
  echo
  echo "## Avvisi ed errori (righe distinte)"
  for s in "${scripts[@]}"; do
    name=$(basename "$s" .txt)
    sed 's/\x1b\[[0-9;]*m//g' "$OUT/$name.log" | grep -E " WARN | ERROR |panicked" \
      | cut -c21- | sed 's/[0-9]\+v[0-9]\+/N/g' | sort | uniq -c | sed "s/^/    [$name] /"
  done
} >> "$report"
# Foto: confronto con i riferimenti.
{
  echo
  echo "## Foto rispetto ai riferimenti (soglia $SOGLIA)"
  echo
  echo "| Copione | Foto | Differenza | Esito |"
  echo "|---|---|---|---|"
} >> "$report"
for s in "${scripts[@]}"; do
  name=$(basename "$s" .txt)
  ref="prove/riferimento/$name"
  for shot in "$OUT/$name"/*.png; do
    [ -f "$shot" ] || continue
    base=$(basename "$shot")
    if [ $update -eq 1 ]; then
      mkdir -p "$ref"; convert "$shot" -resize 480x270! "$ref/$base"
      echo "| \`$name\` | $base | — | riferimento aggiornato |" >> "$report"
      continue
    fi
    if [ ! -f "$ref/$base" ]; then
      echo "| \`$name\` | $base | — | nessun riferimento |" >> "$report"
      continue
    fi
    small="$OUT/$name/small-$base"
    convert "$shot" -resize 480x270! "$small"
    diff=$(compare -metric RMSE "$small" "$ref/$base" "$OUT/$name/diff-$base" 2>&1 | sed -n 's/.*(\([0-9.e-]*\)).*/\1/p')
    if awk -v d="${diff:-1}" -v s="$SOGLIA" 'BEGIN{exit !(d<=s)}'; then
      esito="✅"
    else
      esito="❌ (diff-$base)"; failed=1
    fi
    echo "| \`$name\` | $base | ${diff:-?} | $esito |" >> "$report"
  done
done
echo "Rapporto: $report"
exit $failed
