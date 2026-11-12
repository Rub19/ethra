param([string]$Script = "kairo.txt", [string]$Phase = "None", [string]$ValoPhase = "MENUS", [string]$Remove = "", [string]$Variant = "", [switch]$PendantJeu)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot)
if (-not $PendantJeu -and (Get-Process "League of Legends","VALORANT-Win64-Shipping" -ErrorAction SilentlyContinue)) { "Un jeu est ouvert : rien n'est lancé."; exit 2 }
$build = Start-Process cargo -ArgumentList "build","--features","e2e","-j",$(if ($PendantJeu) { "2" } else { "3" }) -NoNewWindow -PassThru -RedirectStandardError "$env:TEMP\kairo-e2e-build.txt"
$build.PriorityClass = $(if ($PendantJeu) { "Idle" } else { "BelowNormal" })
$build.WaitForExit()
if (Select-String -Path "$env:TEMP\kairo-e2e-build.txt" -Pattern "^error" -Quiet) { Get-Content "$env:TEMP\kairo-e2e-build.txt" | Select-String "^error" -Context 0,10 | Select-Object -First 3; exit 1 }
$out = Join-Path $env:TEMP "kairo-e2e"
Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$out\donnees", "$out\captures" | Out-Null
python -I tools\fixture.py "$out\jeu-essai.json" $Phase $ValoPhase $Variant
if ($Remove) { python -I -c "import json,re,sys; p=sys.argv[1]; d=json.load(open(p,encoding='utf8')); json.dump({k:v for k,v in d.items() if not re.search(sys.argv[2],k)},open(p,'w',encoding='utf8'))" "$out\jeu-essai.json" $Remove }
$env:KAIRO_FAKE = "$out\jeu-essai.json"
$env:KAIRO_DATA_DIR = "$out\donnees"
$env:KAIRO_READONLY = "1"
$env:KAIRO_OFFSCREEN = "1"
$env:KAIRO_E2E = Join-Path $PSScriptRoot $Script
$env:KAIRO_E2E_LOG = "$out\journal.txt"
$env:KAIRO_E2E_SNAP = "$out\captures"
New-Item -ItemType Directory "$out\exports" | Out-Null
$env:KAIRO_E2E_EXPORT = "$out\exports"
$app = Start-Process "target\debug\ethra.exe" -PassThru -WindowStyle Hidden
$app.PriorityClass = "BelowNormal"
$app.WaitForExit()
Get-Content "$out\journal.txt" -Encoding utf8
Get-ChildItem "$out\exports" -ErrorAction SilentlyContinue | ForEach-Object { "export : {0} ({1} octets)" -f $_.Name, $_.Length }
"Captures : $out\captures"
