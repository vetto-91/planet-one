#!/usr/bin/env bash
# Esegue i copioni del pilota automatico (prove/*.txt) e scrive prove/out/RAPPORTO.md.
#
# Uso: scripts/beta_check.sh [copione ...]      (senza argomenti: tutti i copioni)
# Ogni copione gira in una partita di prova isolata (PLANET_ONE_DATA_DIR in prove/out/dati),
# a finestra, alle 11 con il sereno; le variabili PLANET_ONE_* già impostate valgono.
# Uscita: 0 se tutti i copioni passano, 1 altrimenti.
set -u
cd "$(dirname "$0")/.."
OUT=prove/out
mkdir -p "$OUT"
cargo build --release --offline >/dev/null 2>&1 || { echo "build fallita"; exit 2; }
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
  PLANET_ONE_DATA_DIR="${PLANET_ONE_DATA_DIR:-$PWD/$OUT/dati}" \
  PLANET_ONE_SCRIPT="$s" \
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
echo "Rapporto: $report"
exit $failed
