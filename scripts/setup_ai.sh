#!/usr/bin/env bash
# Prepara i dialoghi con IA di Planet One (SOLO IN LOCALE, nessun servizio esterno).
#
# Fase 1: controlla Ollama e, se richiesto, scarica il modello.
#
# Uso:
#   ./scripts/setup_ai.sh                 # controlla soltanto e mostra cosa fare
#   ./scripts/setup_ai.sh --pull          # in più scarica il modello (download una tantum, 5-10 GB)
#   MODEL=gemma4:e2b ./scripts/setup_ai.sh --pull    # modello più leggero
#
# Il download del modello è l'unico momento in cui serve internet; dopo, tutto funziona offline.
# Non vengono installati programmi né aperte porte: se Ollama manca, lo script spiega come installarlo.
set -euo pipefail

MODEL="${MODEL:-gemma4:e4b}"
HOST="${OLLAMA_HOST:-127.0.0.1:11434}"

echo "== Planet One: dialoghi con IA locale =="

if ! command -v ollama >/dev/null 2>&1; then
  echo "Ollama non è installato."
  echo "Installalo dal sito ufficiale (https://ollama.com/download), poi rilancia questo script."
  exit 1
fi
echo "Ollama trovato: $(ollama --version 2>&1 | head -1)"

# Il servizio deve ascoltare solo su questo PC (loopback).
case "$HOST" in
  127.0.0.1*|localhost*|\[::1\]*) ;;
  *) echo "ATTENZIONE: OLLAMA_HOST=$HOST non è locale. Planet One rifiuta host non locali:"
     echo "            imposta OLLAMA_HOST=127.0.0.1:11434 (o toglilo) per non esporre il servizio in rete."
     exit 1 ;;
esac

if ollama list 2>/dev/null | awk 'NR>1{print $1}' | grep -qx "$MODEL"; then
  echo "Modello $MODEL già installato."
elif [ "${1:-}" = "--pull" ]; then
  echo "Scarico $MODEL (una tantum)..."
  ollama pull "$MODEL"
else
  echo "Modello $MODEL non ancora installato. Per scaricarlo:"
  echo "  ollama pull $MODEL        (oppure: ./scripts/setup_ai.sh --pull)"
  exit 0
fi
echo "Pronto. Avvia il gioco: i dialoghi IA si attivano da soli (menu Esc -> Dialoghi con IA)."
