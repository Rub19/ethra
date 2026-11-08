param([string]$Script = "start;go:tierlist;go:champions;go:champion/103/mid;go:overlays;go:reglages;game:valo;game:tft", [int]$Wait = 7000, [string]$Exe = ".\target\debug\ethra.exe")
$s = "$env:TEMP\kairo_snap"
Remove-Item -Recurse -Force $s -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$s\data" -Force | Out-Null
if (Get-Process | Where-Object { $_.ProcessName -match '^(League of Legends|VALORANT-Win64)$' }) { "JEU OUVERT"; exit 1 }
$env:KAIRO_DATA_DIR = "$s\data"
$env:KAIRO_READONLY = "1"
$env:KAIRO_OFFSCREEN = "1"
$env:KAIRO_SNAP = $s
$env:KAIRO_SNAP_WAIT = "$Wait"
$env:KAIRO_SCRIPT = $Script
$p = Start-Process -FilePath $Exe -PassThru -RedirectStandardError "$s\err.txt" -RedirectStandardOutput "$s\out.txt"
if (!$p) { "exe introuvable"; exit 1 }
$peak = 0
while (!$p.HasExited) {
    Start-Sleep -Milliseconds 500
    try { $p.Refresh(); if ($p.WorkingSet64 -gt $peak) { $peak = $p.WorkingSet64 } } catch {}
    
}
"pic memoire : {0:N0} Mo" -f ($peak / 1MB)
Get-ChildItem $s -Filter *.png | ForEach-Object { $_.Name }
Get-Content "$s\err.txt" -Tail 10

