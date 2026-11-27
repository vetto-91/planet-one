# Prepara i dialoghi con IA di Planet One (SOLO IN LOCALE, nessun servizio esterno). Windows.
#
# Fase 1 (testo): con -Ollama scarica l'Ollama del gioco (~1,7 GB) e il modello (~10 GB); con -Pull
#                 solo il modello. Il gioco avvia da sé questo Ollama e lo chiude all'uscita (ADR 0063).
# Fase 2 (voce):  con -Tts scarica Piper (programma locale) e le voci italiane.
# Fase 3 (microfono): con -Stt compila whisper.cpp (servono cmake e Visual Studio) e scarica ggml-base.bin (148 MB).
#   .\scripts\setup_ai.ps1                         # controlla soltanto
#   .\scripts\setup_ai.ps1 -Ollama                 # Ollama del gioco + modello di testo
#   .\scripts\setup_ai.ps1 -Ollama -Amd            # in più il supporto alle schede AMD (ROCm)
#   .\scripts\setup_ai.ps1 -Pull                   # solo il modello, se Ollama c'è già
#   .\scripts\setup_ai.ps1 -Tts                    # Piper + voce it_IT-riccardo-x_low
#   .\scripts\setup_ai.ps1 -Tts -Voice it_IT-paola-medium
#   .\scripts\setup_ai.ps1 -Tts -AllIt             # tutte le voci italiane (da provare)
#   aggiungi -Yes per non chiedere conferma, -DryRun per vedere cosa verrebbe scaricato
# I download sono l'unico momento in cui serve internet (una tantum); dopo, tutto offline.
param([switch]$Ollama, [switch]$Amd, [switch]$Pull, [switch]$Tts, [switch]$Stt, [switch]$AllIt, [switch]$Yes, [switch]$DryRun, [string]$Voice = "it_IT-riccardo-x_low")

$model = if ($env:MODEL) { $env:MODEL } else { "gemma4:e4b" }
$ollamaTag = "v0.40.18"
$ollamaUrl = "https://github.com/ollama/ollama/releases/download/$ollamaTag"
$piperTag = "2023.11.14-2"
$voicesUrl = "https://huggingface.co/rhasspy/piper-voices/resolve/main/it/it_IT"
$ai = Join-Path $env:LOCALAPPDATA "planet-one\ai"

Write-Host "== Planet One: dialoghi con IA locale =="

# ---------- Fase 1: Ollama del gioco + modello ----------
# Nella cartella dati del gioco, non nel sistema: nessun servizio installato, nessuna porta aperta
# finché il gioco non lo avvia (solo su 127.0.0.1, su una porta a caso).
$ol = Join-Path $ai "ollama"
function Find-OllamaExe { foreach ($c in @((Join-Path $ol "ollama.exe"), (Join-Path $ol "bin\ollama.exe"))) { if (Test-Path $c) { return $c } }; return $null }
if ($Ollama) { $Pull = $true }

if ($Ollama -and -not (Find-OllamaExe)) {
    $pkgs = @("ollama-windows-amd64.zip")
    if ($Amd) { $pkgs += "ollama-windows-amd64-rocm.zip" }
    Write-Host ""
    Write-Host "Ollama del gioco: verranno scaricati, UNA TANTUM, dal sito ufficiale (licenza MIT):"
    foreach ($p in $pkgs) { Write-Host "  - $ollamaUrl/$p" }
    Write-Host "  e poi il modello $model (circa 10 GB per gemma4:e4b, 7 GB per gemma4:e2b)."
    Write-Host "  Destinazione: $ol   (il gioco lo avvia e lo chiude da sé)"
    if ($DryRun) { Write-Host "[dry-run] nessun download eseguito."; exit 0 }
    if (-not $Yes) {
        $ans = Read-Host "Scarico e installo? [s/N]"
        if ($ans -notmatch '^[sSyY]') { Write-Host "Annullato."; exit 0 }
    }
    New-Item -ItemType Directory -Force -Path $ol | Out-Null
    $ProgressPreference = 'SilentlyContinue'  # Invoke-WebRequest è molto più veloce senza barra
    $sums = (Invoke-WebRequest -Uri "$ollamaUrl/sha256sum.txt" -UseBasicParsing).Content -split "`n"
    foreach ($p in $pkgs) {
        $zip = Join-Path $env:TEMP $p
        Write-Host "Scarico $p..."
        Invoke-WebRequest -Uri "$ollamaUrl/$p" -OutFile $zip -UseBasicParsing
        $want = ($sums | Where-Object { $_ -match ("\s(\./|\*)?" + [regex]::Escape($p) + "\s*$") } | ForEach-Object { ($_ -split '\s+')[0] }) | Select-Object -First 1
        if (-not $want -or (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower() -ne $want.ToLower()) {
            Remove-Item $zip; Write-Host "Hash di $p errato: annullato."; exit 1
        }
        Expand-Archive -Path $zip -DestinationPath $ol -Force
        Remove-Item $zip
    }
    if (-not (Find-OllamaExe)) { Write-Host "Nell'archivio non c'è ollama.exe: annullato."; exit 1 }
    Write-Host "Ollama $ollamaTag installato in $ol"
}

$exe = Find-OllamaExe
if (-not $exe) {
    Write-Host "Ollama del gioco non installato. Per installarlo:  .\scripts\setup_ai.ps1 -Ollama"
    Write-Host "(Senza, il gioco prova un Ollama già avviato su 127.0.0.1:11434.)"
} else {
    Write-Host ("Ollama del gioco: " + (& $exe --version 2>&1 | Select-Object -Last 1))
    # Un server temporaneo, come quello del gioco: solo loopback, porta a caso, modelli nella
    # cartella del gioco. Si chiude alla fine.
    $port = Get-Random -Minimum 20000 -Maximum 40000
    $env:OLLAMA_HOST = "127.0.0.1:$port"
    $env:OLLAMA_MODELS = Join-Path $ol "models"
    $env:OLLAMA_NO_CLOUD = "1"
    $server = Start-Process -FilePath $exe -ArgumentList "serve" -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput (Join-Path $ol "setup.log") -RedirectStandardError (Join-Path $ol "setup-err.log")
    try {
        for ($i = 0; $i -lt 60; $i++) {
            try { Invoke-WebRequest -Uri "http://127.0.0.1:$port/api/version" -UseBasicParsing -TimeoutSec 2 | Out-Null; break } catch { Start-Sleep -Milliseconds 500 }
        }
        $installed = (& $exe list 2>$null | Select-Object -Skip 1 | ForEach-Object { ($_ -split '\s+')[0] })
        if ($installed -contains $model) { Write-Host "Modello $model già installato." }
        elseif ($Pull) {
            if ($DryRun) { Write-Host "[dry-run] ollama pull $model" } else { Write-Host "Scarico $model (una tantum)..."; & $exe pull $model }
        } else { Write-Host "Modello $model non ancora installato. Per scaricarlo: .\scripts\setup_ai.ps1 -Pull" }
    } finally {
        Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
        Remove-Item Env:OLLAMA_HOST, Env:OLLAMA_MODELS, Env:OLLAMA_NO_CLOUD -ErrorAction SilentlyContinue
    }
}

if ($Tts) {
    $asset = "piper_windows_amd64.zip"
    $piperUrl = "https://github.com/rhasspy/piper/releases/download/$piperTag/$asset"
    if ($AllIt) { $list = @("riccardo/x_low", "paola/medium", "serena/medium", "serena/high") }
    else { $n = $Voice -replace '^it_IT-', ''; $i = $n.LastIndexOf('-'); $list = @($n.Substring(0, $i) + "/" + $n.Substring($i + 1)) }
    Write-Host ""
    Write-Host "Voce (TTS locale): verranno scaricati, UNA TANTUM, da fonti ufficiali:"
    Write-Host "  - Piper $piperTag (MIT): $piperUrl"
    foreach ($v in $list) {
        $name, $q = $v -split '/'
        Write-Host "  - voce it_IT-$name-${q}: $voicesUrl/$name/$q/it_IT-$name-$q.onnx"
        Write-Host "      model card/licenza: https://huggingface.co/rhasspy/piper-voices/blob/main/it/it_IT/$name/$q/MODEL_CARD"
    }
    Write-Host "  Destinazione: $ai   (nessuna rete dopo l'installazione)"
    if ($DryRun) { Write-Host "[dry-run] nessun download eseguito."; exit 0 }
    if (-not $Yes) {
        $ans = Read-Host "Scarico e installo? [s/N]"
        if ($ans -notmatch '^[sSyY]') { Write-Host "Annullato."; exit 0 }
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $ai "piper"), (Join-Path $ai "voices") | Out-Null
    if (-not (Test-Path (Join-Path $ai "piper\piper\piper.exe"))) {
        $zip = Join-Path $env:TEMP $asset
        Invoke-WebRequest -Uri $piperUrl -OutFile $zip
        Expand-Archive -Path $zip -DestinationPath (Join-Path $ai "piper") -Force
        Remove-Item $zip
    } else { Write-Host "Piper già installato." }
    foreach ($v in $list) {
        $name, $q = $v -split '/'
        $base = "it_IT-$name-$q"
        foreach ($ext in @("onnx", "onnx.json")) {
            $dest = Join-Path $ai "voices\$base.$ext"
            if (-not (Test-Path $dest)) { Invoke-WebRequest -Uri "$voicesUrl/$name/$q/$base.$ext" -OutFile $dest }
        }
        Write-Host "Voce installata: $base"
    }
    Write-Host "Fatto. Il gioco trova Piper e le voci da solo (menu Esc -> Dialoghi con IA -> Voce)."
}

if ($Stt) {
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) { Write-Host "Serve cmake per compilare whisper.cpp."; exit 1 }
    $whisperTag = "v1.9.5"
    $modelFile = "ggml-base.bin"
    $modelHash = "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe"
    $srcUrl = "https://github.com/ggml-org/whisper.cpp/archive/refs/tags/$whisperTag.zip"
    $modelUrl = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$modelFile"
    $stt = Join-Path $ai "stt"
    Write-Host ""
    Write-Host "Microfono (STT locale): verranno scaricati, UNA TANTUM, da fonti ufficiali:"
    Write-Host "  - whisper.cpp $whisperTag (MIT), sorgente da compilare: $srcUrl"
    Write-Host "  - modello $modelFile (148 MB, multilingue): $modelUrl"
    Write-Host "  Destinazione: $stt   (nessuna rete dopo l'installazione)"
    if ($DryRun) { Write-Host "[dry-run] nessun download eseguito."; exit 0 }
    if (-not $Yes) {
        $ans = Read-Host "Scarico, compilo e installo? [s/N]"
        if ($ans -notmatch '^[sSyY]') { Write-Host "Annullato."; exit 0 }
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $stt "models") | Out-Null
    $exe = Join-Path $stt "whisper-cli.exe"
    if (-not (Test-Path $exe)) {
        $work = Join-Path $env:TEMP ("planet-one-stt-" + [guid]::NewGuid())
        New-Item -ItemType Directory -Path $work | Out-Null
        Invoke-WebRequest -Uri $srcUrl -OutFile (Join-Path $work "src.zip")
        Expand-Archive -Path (Join-Path $work "src.zip") -DestinationPath $work
        $src = Join-Path $work ("whisper.cpp-" + $whisperTag.TrimStart('v'))
        cmake -S $src -B (Join-Path $work "build") -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF
        cmake --build (Join-Path $work "build") --config Release --target whisper-cli
        Copy-Item (Join-Path $work "build\bin\Release\whisper-cli.exe") $exe
        Remove-Item -Recurse -Force $work
    } else { Write-Host "whisper-cli già installato." }
    $dest = Join-Path $stt "models\$modelFile"
    if (-not (Test-Path $dest)) {
        $tmp = "$dest.part"
        Invoke-WebRequest -Uri $modelUrl -OutFile $tmp
        if ((Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower() -ne $modelHash) { Remove-Item $tmp; Write-Host "Hash del modello errato: annullato."; exit 1 }
        Move-Item $tmp $dest
    } else { Write-Host "Modello già installato." }
    Write-Host "Fatto. Attiva il microfono nel menu Esc o con il comando stt on nella shell."
}
