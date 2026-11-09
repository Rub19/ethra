param([string]$Notes = "", [switch]$SkipBuild)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
if (-not $SkipBuild) {
    cmd /c ".\low.cmd build --release"
    if ($LASTEXITCODE -ne 0) { throw "compilation de l'app" }
    Push-Location setup
    cmd /c "..\low.cmd build --release"
    if ($LASTEXITCODE -ne 0) { throw "compilation de l'installeur" }
    Pop-Location
}
New-Item -ItemType Directory -Force "release" | Out-Null
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$name = "Kairo_${version}_x64-setup.exe"
$out = Join-Path $PSScriptRoot "release\$name"
$pack = Start-Process -FilePath "setup\target\release\kairo-setup.exe" -ArgumentList "--pack", "target\release\ethra.exe", "`"$out`"" -Wait -PassThru
if ($pack.ExitCode -ne 0) { throw "paquet de l'installeur" }
"{0} : {1:N1} Mo" -f "ethra.exe", ((Get-Item target\release\ethra.exe).Length / 1MB)
"{0} : {1:N1} Mo" -f $name, ((Get-Item $out).Length / 1MB)
$key = if ($env:KAIRO_SIGN_KEY) { $env:KAIRO_SIGN_KEY } else { Join-Path $HOME ".tauri\kairo-updater.key" }
if (-not (Test-Path $key)) {
    "Pas de clé de signature ($key) : installeur non signé, latest.json non écrit."
    $out
    exit 0
}
$root = Split-Path -Parent (git rev-parse --path-format=absolute --git-common-dir)
$tauri = @((Join-Path $PSScriptRoot "..\node_modules\.bin\tauri.cmd"), (Join-Path $root "node_modules\.bin\tauri.cmd")) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $tauri) { throw "outil de signature introuvable : lance npm install dans $root" }
Remove-Item "$out.sig" -ErrorAction SilentlyContinue
cmd /c "`"$tauri`" signer sign -f `"$key`" -p `"`" `"$out`" >nul 2>&1"
$signed = $LASTEXITCODE
if ($signed -ne 0 -or -not (Test-Path "$out.sig")) { throw "signature de l'installeur" }
$latest = [ordered]@{
    version = $version
    notes = $Notes
    pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = (Get-Content "$out.sig" -Raw).Trim()
            url = "https://github.com/Rub19/ethra/releases/download/v$version/$name"
        }
    }
}
$json = $latest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $PSScriptRoot "release\latest.json"), $json, (New-Object Text.UTF8Encoding $false))
"Signé : $name.sig et release\latest.json prêts. Rien n'est publié."
$out
