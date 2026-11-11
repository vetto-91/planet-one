#!/usr/bin/env bash
# Esegue i copioni del pilota automatico (prove/*.txt) e scrive prove/out/RAPPORTO.md.
#
# Uso: scripts/beta_check.sh [--aggiorna] [--con-finestra] [--con-casse] [--risoluzioni R] [copione ...]
#      (senza copioni: tutti)
#
# Di base le prove non disturbano chi usa il PC: niente finestra (la camera disegna in
# un'immagine alla risoluzione esatta, foto senza HUD), quindi nessun input da tastiera, mouse
# o gamepad, e l'audio va in una scheda virtuale di PulseAudio (casse mute; `registra` la
# registra lo stesso). --con-finestra apre la finestra vera (foto con HUD e pannelli;
# le risoluzioni più grandi dello schermo restano senza finestra); --con-casse fa suonare le casse.
#
# Risoluzioni standard, come un giocatore vero: 720p (1280x720), 1080p (1920x1080),
# 1440p (2560x1440), 4k (3840x2160). --risoluzioni sceglie:
#   giro  (predefinito) ogni copione a una risoluzione diversa, a rotazione: in una corsa si
#         provano tutte e quattro senza moltiplicare i tempi;
#   tutte ogni copione a tutte e quattro (corsa lunga: quattro volte);
#   720p | 1080p | 1440p | 4k | LxA   tutti i copioni a una sola risoluzione.
# Un copione con «# env: PLANET_ONE_SIZE=...» usa sempre quella.
#
# Le foto di ogni copione si confrontano con prove/riferimento/<copione>/<foto>.png
# (ridotte a 480x270, ImageMagick `compare`, RMSE normalizzato; soglia SOGLIA=0.12). --aggiorna copia
# le foto di questa corsa come nuovi riferimenti.
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
mode=giro
window=0
speakers=0
while [ $# -gt 0 ]; do
  case "$1" in
    --aggiorna) update=1; shift ;;
    --con-finestra) window=1; shift ;;
    --senza-schermo) window=0; shift ;;   # vecchio nome, ora è il comportamento di base
    --con-casse) speakers=1; shift ;;
    --risoluzioni) mode="${2:-giro}"; shift 2 ;;
    *) break ;;
  esac
done
[ $window -eq 0 ] && export PLANET_ONE_HEADLESS=1
# Audio in una scheda virtuale (null sink), tolta all'uscita.
if [ $speakers -eq 0 ] && command -v pactl >/dev/null; then
  sink_module=$(pactl load-module module-null-sink sink_name=planet_one_prove \
    sink_properties=device.description=PlanetOneProve 2>/dev/null)
  if [ -n "$sink_module" ]; then
    export PULSE_SINK=planet_one_prove
    trap 'pactl unload-module "$sink_module" 2>/dev/null' EXIT
  else
    echo "attenzione: scheda audio virtuale non creata, le prove suonano nelle casse"
  fi
fi
STANDARD=(1280x720 1920x1080 2560x1440 3840x2160)
size_of() {
  case "$1" in
    720p) echo 1280x720 ;;
    1080p) echo 1920x1080 ;;
    1440p) echo 2560x1440 ;;
    4k|4K|2160p) echo 3840x2160 ;;
    *) echo "$1" ;;
  esac
}
SOGLIA="${SOGLIA:-0.12}"
# Schermo più grande collegato (xrandr): una finestra che non ci sta verrebbe ridotta dal
# gestore delle finestre, quindi quelle corse girano senza finestra, alla misura esatta.
screen_area=$(xrandr 2>/dev/null | sed -n 's/.* connected.* \([0-9]\+\)x\([0-9]\+\)+.*/\1 \2/p' \
  | awk '{a=$1*$2; if (a>m) m=a} END{print m+0}')
scripts=("$@")
[ ${#scripts[@]} -eq 0 ] && scripts=(prove/*.txt)

# Corse da fare: «copione|risoluzione|chiave» (la chiave nomina log e cartella d'uscita).
runs=()
i=0
for s in "${scripts[@]}"; do
  name=$(basename "$s" .txt)
  fixed=$(sed -n 's/^# env: PLANET_ONE_SIZE=//p' "$s" | head -1)
  if [ -n "$fixed" ]; then
    runs+=("$s|$fixed|$name")
  elif [ "$mode" = tutte ]; then
    for r in "${STANDARD[@]}"; do runs+=("$s|$r|$name@$r"); done
  elif [ "$mode" = giro ]; then
    runs+=("$s|${STANDARD[$((i % 4))]}|$name")
  else
    runs+=("$s|$(size_of "$mode")|$name")
  fi
  i=$((i + 1))
done

report="$OUT/RAPPORTO.md"
{
  echo "# Rapporto del pilota automatico"
  echo
  echo "Data: $(date '+%Y-%m-%d %H:%M') · versione: $(grep -o 'v[0-9][0-9.]*' src/version.rs | head -1)" \
    "· risoluzioni: $mode${PLANET_ONE_HEADLESS:+ · senza finestra}${PULSE_SINK:+ · casse mute}"
  echo
  echo "| Copione | Risoluzione | Esito | Verifiche ok | Fallimenti | FPS medio | 1% peggiore | WARN | ERROR | panic |"
  echo "|---|---|---|---|---|---|---|---|---|---|"
} > "$report"
failed=0
for run in "${runs[@]}"; do
  IFS='|' read -r s res key <<< "$run"
  log="$OUT/$key.log"
  rm -rf "${OUT:?}/$key"
  # Variabili proprie del copione: righe «# env: CHIAVE=valore» (es. PLANET_ONE_SPEED=6).
  extra=$(sed -n 's/^# env: //p' "$s" | grep -v '^PLANET_ONE_SIZE=' | tr '\n' ' ')
  w=${res%x*}; h=${res#*x}
  if [ -z "${PLANET_ONE_HEADLESS:-}" ] && [ "$screen_area" -gt 0 ] && [ $((w * h)) -ge "$screen_area" ]; then
    extra="$extra PLANET_ONE_HEADLESS=1"; shown="$res (senza finestra)"
  else
    shown="$res"
  fi
  env $extra \
  PLANET_ONE_DATA_DIR="${PLANET_ONE_DATA_DIR:-$PWD/$OUT/dati}" \
  PLANET_ONE_SCRIPT="$s" \
  PLANET_ONE_OUT="$PWD/$OUT/$key" \
  PLANET_ONE_SIZE="$res" \
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
  fps_line=$(grep -h "^FPS:" "$OUT/$key/RAPPORTO.txt" 2>/dev/null)
  avg=$(sed -n 's/^FPS: medio \([0-9]*\).*/\1/p' <<< "$fps_line")
  low=$(sed -n 's/.*1% peggiore \([0-9]*\).*/\1/p' <<< "$fps_line")
  case $code in
    0) esito="✅ passato" ;;
    124) esito="⏱️ tempo scaduto"; failed=1 ;;
    *) esito="❌ fallito ($code)"; failed=1 ;;
  esac
  real=$(sed -n 's/^Risoluzione: //p' "$OUT/$key/RAPPORTO.txt" 2>/dev/null)
  [ -n "$real" ] && [ "${real%% *}" != "$res" ] && shown="$shown, vera $real"
  echo "| \`$key\` | $shown | $esito | $ok | $ko | ${avg:-—} | ${low:-—} | $warn | $err | $pan |" >> "$report"
  echo "$key ($shown): $esito (ok $ok, fallimenti $ko, FPS ${avg:-?}/${low:-?}, panic $pan)"
done
{
  echo
  echo "## Fallimenti"
  for run in "${runs[@]}"; do
    IFS='|' read -r s res key <<< "$run"
    f="$OUT/$key/RAPPORTO.txt"
    [ -f "$f" ] && grep -q "^- " "$f" && { echo; echo "### $key ($res)"; grep "^- " "$f"; }
  done
  echo
  echo "## Avvisi ed errori (righe distinte)"
  for run in "${runs[@]}"; do
    IFS='|' read -r s res key <<< "$run"
    sed 's/\x1b\[[0-9;]*m//g' "$OUT/$key.log" | grep -E " WARN | ERROR |panicked" \
      | cut -c21- | sed 's/[0-9]\+v[0-9]\+/N/g' | sort | uniq -c | sed "s/^/    [$key] /"
  done
} >> "$report"
# Foto: confronto con i riferimenti (uno per copione, qualunque risoluzione: si confrontano ridotte).
{
  echo
  echo "## Foto rispetto ai riferimenti (soglia $SOGLIA)"
  echo
  echo "| Copione | Foto | Differenza | Esito |"
  echo "|---|---|---|---|"
} >> "$report"
for run in "${runs[@]}"; do
  IFS='|' read -r s res key <<< "$run"
  name=$(basename "$s" .txt)
  ref="prove/riferimento/$name"
  for shot in "$OUT/$key"/*.png; do
    [ -f "$shot" ] || continue
    base=$(basename "$shot")
    case "$base" in small-*|diff-*|*-spettro.png|*-volume.png) continue ;; esac
    if [ $update -eq 1 ]; then
      mkdir -p "$ref"; convert "$shot" -resize 480x270! "$ref/$base"
      echo "| \`$key\` | $base | — | riferimento aggiornato |" >> "$report"
      continue
    fi
    if [ ! -f "$ref/$base" ]; then
      echo "| \`$key\` | $base | — | nessun riferimento |" >> "$report"
      continue
    fi
    small="$OUT/$key/small-$base"
    convert "$shot" -resize 480x270! "$small"
    diff=$(compare -metric RMSE "$small" "$ref/$base" "$OUT/$key/diff-$base" 2>&1 | sed -n 's/.*(\([0-9.e-]*\)).*/\1/p')
    if awk -v d="${diff:-1}" -v s="$SOGLIA" 'BEGIN{exit !(d<=s)}'; then
      esito="✅"
    else
      esito="❌ (diff-$base)"; failed=1
    fi
    echo "| \`$key\` | $base | ${diff:-?} | $esito |" >> "$report"
  done
done
echo "Rapporto: $report"
exit $failed
