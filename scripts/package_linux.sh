#!/usr/bin/env bash
# Costruisce il pacchetto Linux di una release (ADR 0004): target/dist/planet-one-<tag>-linux-x86_64.tar.gz.
# Windows e macOS li costruisce GitHub Actions (.github/workflows/release.yml) con lo stesso contenuto.
#
# Uso: ./scripts/package_linux.sh v0.0.69.0
set -euo pipefail
TAG="${1:?serve il tag, es. v0.0.69.0}"
cd "$(dirname "$0")/.."
cargo build --release --locked
N="planet-one-${TAG}-linux-x86_64"
rm -rf "target/dist/$N" "target/dist/$N.tar.gz"
mkdir -p "target/dist/$N"
cp target/release/planet-one "target/dist/$N/"
strip "target/dist/$N/planet-one"
cp -r assets "target/dist/$N/assets"
# setup_ai.sh: Ollama del gioco, modello, voci e microfono (ADR 0063).
cp README.md CREDITS.md LICENSE-MIT LICENSE-APACHE scripts/setup_ai.sh "target/dist/$N/"
tar -czf "target/dist/$N.tar.gz" -C target/dist "$N"
echo "Pacchetto: target/dist/$N.tar.gz ($(du -h "target/dist/$N.tar.gz" | cut -f1))"
