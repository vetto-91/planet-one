#!/usr/bin/env bash
# Sincronizza il progetto su Google Drive con rclone, escludendo i file pesanti
# e rigenerabili (target/ di cargo, worktree di Claude, export di sessione).
#
# Uso:
#   ./scripts/sync_drive.sh --dry-run          # mostra cosa verrebbe copiato/cancellato
#   ./scripts/sync_drive.sh                    # sync reale
#   ./scripts/sync_drive.sh --delete-excluded  # in più rimuove da Drive ciò che ora è escluso
#
# Variabili: REMOTE (default gdrive:dev/Project-One)
set -euo pipefail

REMOTE="${REMOTE:-gdrive:dev/Project-One}"
cd "$(dirname "$0")/.."

exec rclone sync . "$REMOTE" --progress \
  --exclude-if-present CACHEDIR.TAG \
  --exclude '/target/**' \
  --exclude '/.claude/worktrees/**' \
  --exclude '/context/**' \
  "$@"
