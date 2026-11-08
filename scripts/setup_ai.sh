#!/usr/bin/env bash
# Prepara i dialoghi con IA di Planet One (SOLO IN LOCALE, nessun servizio esterno).
#
# Fase 1 (testo):  controlla Ollama e, con --pull, scarica il modello.
# Fase 2 (voce):   con --tts scarica Piper (programma locale) e le voci italiane.
#
# Uso:
#   ./scripts/setup_ai.sh                          # controlla soltanto e mostra cosa fare
#   ./scripts/setup_ai.sh --pull                   # in più scarica il modello di testo (5-10 GB)
#   MODEL=gemma4:e2b ./scripts/setup_ai.sh --pull  # modello più leggero
#   ./scripts/setup_ai.sh --tts                    # Piper + voce it_IT-riccardo-x_low
#   ./scripts/setup_ai.sh --tts --voice it_IT-paola-medium
#   ./scripts/setup_ai.sh --tts --all-it           # tutte le voci italiane (da provare e scegliere)
#   aggiungi --yes per non chiedere conferma, --dry-run per vedere cosa verrebbe scaricato
#
# I download sono l'unico momento in cui serve internet (installazione una tantum, da fonti ufficiali);
# dopo, tutto funziona offline. Il gioco non ridistribuisce modelli né voci: le scarichi tu, con
# il tuo consenso. Controlla la licenza di ogni voce (link alla model card mostrato prima del download).
set -euo pipefail

MODEL="${MODEL:-gemma4:e4b}"
HOST="${OLLAMA_HOST:-127.0.0.1:11434}"
PIPER_TAG="2023.11.14-2"
VOICES_URL="https://huggingface.co/rhasspy/piper-voices/resolve/main/it/it_IT"
IT_VOICES="riccardo/x_low paola/medium serena/medium serena/high"

PULL=0; TTS=0; ALL=0; YES=0; DRY=0; VOICE="it_IT-riccardo-x_low"
while [ $# -gt 0 ]; do
  case "$1" in
    --pull) PULL=1 ;;
    --tts) TTS=1 ;;
    --all-it) ALL=1 ;;
    --voice) VOICE="$2"; shift ;;
    --yes) YES=1 ;;
    --dry-run) DRY=1 ;;
    *) echo "Opzione sconosciuta: $1"; exit 2 ;;
  esac
  shift
done

case "$(uname -s)" in
  Darwin) AI="$HOME/Library/Application Support/planet-one/ai" ;;
  *) AI="${XDG_DATA_HOME:-$HOME/.local/share}/planet-one/ai" ;;
esac

echo "== Planet One: dialoghi con IA locale =="

# ---------- Fase 1: Ollama ----------
if ! command -v ollama >/dev/null 2>&1; then
  echo "Ollama non è installato."
  echo "Installalo dal sito ufficiale (https://ollama.com/download), poi rilancia questo script."
  OLLAMA_OK=0
else
  OLLAMA_OK=1
  echo "Ollama trovato: $(ollama --version 2>&1 | head -1)"
  case "$HOST" in
    127.0.0.1*|localhost*|\[::1\]*) ;;
    *) echo "ATTENZIONE: OLLAMA_HOST=$HOST non è locale. Planet One rifiuta host non locali:"
       echo "            imposta OLLAMA_HOST=127.0.0.1:11434 (o toglilo) per non esporre il servizio in rete."
       exit 1 ;;
  esac
  if ollama list 2>/dev/null | awk 'NR>1{print $1}' | grep -qx "$MODEL"; then
    echo "Modello $MODEL già installato."
  elif [ "$PULL" = 1 ]; then
    if [ "$DRY" = 1 ]; then echo "[dry-run] ollama pull $MODEL"; else echo "Scarico $MODEL (una tantum)..."; ollama pull "$MODEL"; fi
  else
    echo "Modello $MODEL non ancora installato. Per scaricarlo:  ollama pull $MODEL   (oppure --pull)"
  fi
fi

# ---------- Fase 2: Piper + voci ----------
if [ "$TTS" = 1 ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) ASSET="piper_linux_x86_64.tar.gz" ;;
    Linux-aarch64) ASSET="piper_linux_aarch64.tar.gz" ;;
    Darwin-arm64) ASSET="piper_macos_aarch64.tar.gz" ;;
    Darwin-x86_64) ASSET="piper_macos_x64.tar.gz" ;;
    *) echo "Piper: piattaforma non supportata da questo script ($(uname -s)-$(uname -m))."; exit 1 ;;
  esac
  PIPER_URL="https://github.com/rhasspy/piper/releases/download/$PIPER_TAG/$ASSET"
  if [ "$ALL" = 1 ]; then LIST="$IT_VOICES"; else
    # it_IT-<nome>-<qualità> -> <nome>/<qualità>
    n="${VOICE#it_IT-}"; LIST="${n%-*}/${n##*-}"
  fi
  echo
  echo "Voce (TTS locale): verranno scaricati, UNA TANTUM, da fonti ufficiali:"
  echo "  - Piper $PIPER_TAG (MIT):  $PIPER_URL"
  for v in $LIST; do
    name="${v%/*}"; q="${v#*/}"
    echo "  - voce it_IT-$name-$q:  $VOICES_URL/$name/$q/it_IT-$name-$q.onnx"
    echo "      model card/licenza: https://huggingface.co/rhasspy/piper-voices/blob/main/it/it_IT/$name/$q/MODEL_CARD"
  done
  echo "  Destinazione: $AI   (nessuna rete dopo l'installazione)"
  if [ "$DRY" = 1 ]; then echo "[dry-run] nessun download eseguito."; exit 0; fi
  if [ "$YES" != 1 ]; then
    read -r -p "Scarico e installo? [s/N] " ans
    case "$ans" in s|S|y|Y) ;; *) echo "Annullato."; exit 0 ;; esac
  fi
  mkdir -p "$AI/piper" "$AI/voices"
  if [ ! -x "$AI/piper/piper/piper" ]; then
    tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
    curl -fL --progress-bar -o "$tmp/$ASSET" "$PIPER_URL"
    tar -xzf "$tmp/$ASSET" -C "$AI/piper"
  else
    echo "Piper già installato."
  fi
  for v in $LIST; do
    name="${v%/*}"; q="${v#*/}"; base="it_IT-$name-$q"
    for ext in onnx onnx.json; do
      [ -f "$AI/voices/$base.$ext" ] || curl -fL --progress-bar -o "$AI/voices/$base.$ext" "$VOICES_URL/$name/$q/$base.$ext"
    done
    echo "Voce installata: $base"
  done
  echo "Fatto. Il gioco trova Piper e le voci da solo (menu Esc -> Dialoghi con IA -> Voce)."
elif [ "$OLLAMA_OK" = 1 ]; then
  echo "Pronto. Per la voce: ./scripts/setup_ai.sh --tts"
fi
