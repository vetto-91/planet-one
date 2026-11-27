#!/usr/bin/env bash
# Prepara i dialoghi con IA di Planet One (SOLO IN LOCALE, nessun servizio esterno).
#
# Fase 1 (testo):  con --ollama scarica l'Ollama del gioco e il modello; con --pull solo il modello.
#                  Il gioco avvia da sé questo Ollama e lo chiude all'uscita (ADR 0063).
# Fase 2 (voce):   con --tts scarica Piper (programma locale) e le voci italiane.
# Fase 3 (microfono): con --stt compila whisper.cpp e scarica un modello multilingue.
#
# Uso:
#   ./scripts/setup_ai.sh                          # controlla soltanto e mostra cosa fare
#   ./scripts/setup_ai.sh --ollama                 # Ollama del gioco (~1,7 GB) + modello (~10 GB)
#   ./scripts/setup_ai.sh --ollama --amd           # in più il supporto alle schede AMD (ROCm, ~1 GB)
#   ./scripts/setup_ai.sh --pull                   # solo il modello, se Ollama c'è già
#   MODEL=gemma4:e2b ./scripts/setup_ai.sh --pull  # modello più leggero
#   ./scripts/setup_ai.sh --tts                    # Piper + voce it_IT-riccardo-x_low
#   ./scripts/setup_ai.sh --tts --voice it_IT-paola-medium
#   ./scripts/setup_ai.sh --tts --all-it           # tutte le voci italiane (da provare e scegliere)
#   ./scripts/setup_ai.sh --stt                    # whisper.cpp (compilato qui) + ggml-base.bin (148 MB)
#   aggiungi --yes per non chiedere conferma, --dry-run per vedere cosa verrebbe scaricato
#
# I download sono l'unico momento in cui serve internet (installazione una tantum, da fonti ufficiali);
# dopo, tutto funziona offline. Il gioco non ridistribuisce modelli né voci: le scarichi tu, con
# il tuo consenso. Controlla la licenza di ogni voce (link alla model card mostrato prima del download).
set -euo pipefail

MODEL="${MODEL:-gemma4:e4b}"
OLLAMA_TAG="v0.40.18"
OLLAMA_URL="https://github.com/ollama/ollama/releases/download/$OLLAMA_TAG"
PIPER_TAG="2023.11.14-2"
WHISPER_TAG="v1.9.5"
WHISPER_MODEL="ggml-base.bin"
WHISPER_MODEL_SHA256="60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
VOICES_URL="https://huggingface.co/rhasspy/piper-voices/resolve/main/it/it_IT"
IT_VOICES="riccardo/x_low paola/medium serena/medium serena/high"

PULL=0; OLLAMA=0; AMD=0; TTS=0; STT=0; ALL=0; YES=0; DRY=0; VOICE="it_IT-riccardo-x_low"
while [ $# -gt 0 ]; do
  case "$1" in
    --pull) PULL=1 ;;
    --ollama) OLLAMA=1; PULL=1 ;;
    --amd) AMD=1 ;;
    --tts) TTS=1 ;;
    --stt) STT=1 ;;
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

# ---------- Fase 1: Ollama del gioco + modello ----------
# Va nella cartella dati del gioco, non nel sistema: nessun servizio installato, nessuna porta aperta
# finché il gioco non lo avvia (solo su 127.0.0.1, su una porta a caso).
OL="$AI/ollama"
ollama_exe() { for c in "$OL/bin/ollama" "$OL/ollama"; do [ -x "$c" ] && { echo "$c"; return; }; done; }

if [ "$OLLAMA" = 1 ] && [ -z "$(ollama_exe)" ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) PKGS="ollama-linux-amd64.tar.zst"; [ "$AMD" = 1 ] && PKGS="$PKGS ollama-linux-amd64-rocm.tar.zst" ;;
    Linux-aarch64) PKGS="ollama-linux-arm64.tar.zst" ;;
    Darwin-*) PKGS="ollama-darwin.tgz" ;;
    *) echo "Ollama: piattaforma non supportata da questo script ($(uname -s)-$(uname -m))."; exit 1 ;;
  esac
  echo
  echo "Ollama del gioco: verranno scaricati, UNA TANTUM, dal sito ufficiale (licenza MIT):"
  for p in $PKGS; do echo "  - $OLLAMA_URL/$p"; done
  echo "  e poi il modello $MODEL (circa 10 GB per gemma4:e4b, 7 GB per gemma4:e2b)."
  echo "  Destinazione: $OL   (il gioco lo avvia e lo chiude da sé)"
  if [ "$DRY" = 1 ]; then echo "[dry-run] nessun download eseguito."; exit 0; fi
  case "$PKGS" in *.zst*) command -v zstd >/dev/null 2>&1 || { echo "Serve zstd per estrarre Ollama (es. sudo apt install zstd)."; exit 1; } ;; esac
  if [ "$YES" != 1 ]; then
    read -r -p "Scarico e installo? [s/N] " ans
    case "$ans" in s|S|y|Y) ;; *) echo "Annullato."; exit 0 ;; esac
  fi
  # Lo scaricamento sta accanto alla destinazione: /tmp spesso è in RAM e l'archivio pesa quasi 2 GB.
  mkdir -p "$OL"
  ol_tmp="$(mktemp -d "$OL/.scarico.XXXXXX")"; trap 'rm -rf "$ol_tmp"' EXIT
  curl -fL --progress-bar -o "$ol_tmp/sha256sum.txt" "$OLLAMA_URL/sha256sum.txt"
  for p in $PKGS; do
    curl -fL --progress-bar -o "$ol_tmp/$p" "$OLLAMA_URL/$p"
    want="$(awk -v f="$p" '$2==f || $2=="./"f || $2=="*"f {print $1}' "$ol_tmp/sha256sum.txt")"
    got="$(sha256sum "$ol_tmp/$p" 2>/dev/null || shasum -a 256 "$ol_tmp/$p")"
    [ -n "$want" ] && [ "${got%% *}" = "$want" ] || { echo "Hash di $p errato: annullato."; exit 1; }
    case "$p" in
      *.zst) zstd -dc "$ol_tmp/$p" | tar -x -C "$OL" ;;
      *) tar -xzf "$ol_tmp/$p" -C "$OL" ;;
    esac
    rm -f "$ol_tmp/$p"
  done
  [ -n "$(ollama_exe)" ] || { echo "Nell'archivio non c'è l'eseguibile di Ollama: annullato."; exit 1; }
  echo "Ollama $OLLAMA_TAG installato in $OL"
fi

EXE="$(ollama_exe)"
if [ -z "$EXE" ]; then
  OLLAMA_OK=0
  echo "Ollama del gioco non installato. Per installarlo:  ./scripts/setup_ai.sh --ollama"
  echo "(Senza, il gioco prova un Ollama già avviato su 127.0.0.1:11434.)"
else
  OLLAMA_OK=1
  echo "Ollama del gioco: $("$EXE" --version 2>&1 | tail -1)"
  # Un server temporaneo, come quello del gioco: solo loopback, porta a caso, modelli nella cartella
  # del gioco. Si chiude alla fine dello script.
  PORT=$((20000 + RANDOM % 20000))
  OLLAMA_HOST="127.0.0.1:$PORT" OLLAMA_MODELS="$OL/models" OLLAMA_NO_CLOUD=1 "$EXE" serve >"$OL/setup.log" 2>&1 &
  SERVER=$!
  trap 'kill "$SERVER" 2>/dev/null; rm -rf "${ol_tmp:-}"' EXIT
  for _ in $(seq 1 60); do curl -fs "http://127.0.0.1:$PORT/api/version" >/dev/null 2>&1 && break; sleep 0.5; done
  export OLLAMA_HOST="127.0.0.1:$PORT"
  if "$EXE" list 2>/dev/null | awk 'NR>1{print $1}' | grep -qx "$MODEL"; then
    echo "Modello $MODEL già installato."
  elif [ "$PULL" = 1 ]; then
    if [ "$DRY" = 1 ]; then echo "[dry-run] ollama pull $MODEL"; else echo "Scarico $MODEL (una tantum)..."; "$EXE" pull "$MODEL"; fi
  else
    echo "Modello $MODEL non ancora installato. Per scaricarlo:  ./scripts/setup_ai.sh --pull"
  fi
  kill "$SERVER" 2>/dev/null; wait "$SERVER" 2>/dev/null || true
  trap 'rm -rf "${ol_tmp:-}"' EXIT
  unset OLLAMA_HOST
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

# ---------- Fase 3: whisper.cpp + modello ----------
if [ "$STT" = 1 ]; then
  command -v cmake >/dev/null 2>&1 || { echo "Serve cmake per compilare whisper.cpp."; exit 1; }
  SRC_URL="https://github.com/ggml-org/whisper.cpp/archive/refs/tags/$WHISPER_TAG.tar.gz"
  MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$WHISPER_MODEL"
  echo
  echo "Microfono (STT locale): verranno scaricati, UNA TANTUM, da fonti ufficiali:"
  echo "  - whisper.cpp $WHISPER_TAG (MIT), sorgente da compilare:  $SRC_URL"
  echo "  - modello $WHISPER_MODEL (148 MB, multilingue):  $MODEL_URL"
  echo "  Destinazione: $AI/stt   (nessuna rete dopo l'installazione)"
  if [ "$DRY" = 1 ]; then echo "[dry-run] nessun download eseguito."; exit 0; fi
  if [ "$YES" != 1 ]; then
    read -r -p "Scarico, compilo e installo? [s/N] " ans
    case "$ans" in s|S|y|Y) ;; *) echo "Annullato."; exit 0 ;; esac
  fi
  mkdir -p "$AI/stt/models"
  stt_tmp="$(mktemp -d)"; trap 'rm -rf "$stt_tmp"' EXIT
  if [ ! -x "$AI/stt/whisper-cli" ]; then
    curl -fL --progress-bar -o "$stt_tmp/src.tar.gz" "$SRC_URL"
    tar -xzf "$stt_tmp/src.tar.gz" -C "$stt_tmp"
    # Su x86 Linux niente -march=native: gli assembler vecchi non conoscono le istruzioni dei processori nuovi.
    SIMD=""
    if [ "$(uname -s)-$(uname -m)" = "Linux-x86_64" ]; then
      SIMD="-DGGML_NATIVE=OFF"
      if grep -qw avx2 /proc/cpuinfo; then SIMD="$SIMD -DGGML_AVX=ON -DGGML_AVX2=ON -DGGML_FMA=ON -DGGML_F16C=ON"; fi
    fi
    # shellcheck disable=SC2086
    cmake -S "$stt_tmp/whisper.cpp-${WHISPER_TAG#v}" -B "$stt_tmp/build" -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF $SIMD
    cmake --build "$stt_tmp/build" --target whisper-cli -j "$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)"
    cp "$stt_tmp/build/bin/whisper-cli" "$AI/stt/whisper-cli"
  else
    echo "whisper-cli già installato."
  fi
  if [ ! -f "$AI/stt/models/$WHISPER_MODEL" ]; then
    curl -fL --progress-bar -o "$stt_tmp/$WHISPER_MODEL" "$MODEL_URL"
    got="$(sha256sum "$stt_tmp/$WHISPER_MODEL" 2>/dev/null || shasum -a 256 "$stt_tmp/$WHISPER_MODEL")"
    [ "${got%% *}" = "$WHISPER_MODEL_SHA256" ] || { echo "Hash del modello errato: annullato."; exit 1; }
    mv "$stt_tmp/$WHISPER_MODEL" "$AI/stt/models/$WHISPER_MODEL"
  else
    echo "Modello già installato."
  fi
  echo "Fatto. Attiva il microfono nel menu Esc o con il comando stt on nella shell."
fi
exit 0
