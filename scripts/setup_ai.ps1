# Prepara i dialoghi con IA di Planet One (SOLO IN LOCALE, nessun servizio esterno). Windows.
#
# Fase 1: controlla Ollama e, con -Pull, scarica il modello (download una tantum, 5-10 GB).
#   .\scripts\setup_ai.ps1            # controlla soltanto
#   .\scripts\setup_ai.ps1 -Pull      # in più scarica il modello
#   $env:MODEL="gemma4:e2b"; .\scripts\setup_ai.ps1 -Pull    # modello più leggero
param([switch]$Pull)

$model = if ($env:MODEL) { $env:MODEL } else { "gemma4:e4b" }
$ollamaHost = if ($env:OLLAMA_HOST) { $env:OLLAMA_HOST } else { "127.0.0.1:11434" }

Write-Host "== Planet One: dialoghi con IA locale =="
if (-not (Get-Command ollama -ErrorAction SilentlyContinue)) {
    Write-Host "Ollama non è installato. Installalo da https://ollama.com/download e rilancia lo script."
    exit 1
}
Write-Host ("Ollama trovato: " + (ollama --version 2>&1 | Select-Object -First 1))

if ($ollamaHost -notmatch '^(127\.0\.0\.1|localhost|\[::1\])') {
    Write-Host "ATTENZIONE: OLLAMA_HOST=$ollamaHost non è locale. Planet One rifiuta host non locali."
    Write-Host "Imposta OLLAMA_HOST=127.0.0.1:11434 (o rimuovilo) per non esporre il servizio in rete."
    exit 1
}

$installed = (ollama list 2>$null | Select-Object -Skip 1 | ForEach-Object { ($_ -split '\s+')[0] })
if ($installed -contains $model) {
    Write-Host "Modello $model già installato."
} elseif ($Pull) {
    Write-Host "Scarico $model (una tantum)..."
    ollama pull $model
} else {
    Write-Host "Modello $model non ancora installato. Per scaricarlo: ollama pull $model  (oppure -Pull)"
    exit 0
}
Write-Host "Pronto. Avvia il gioco: i dialoghi IA si attivano da soli (menu Esc -> Dialoghi con IA)."
