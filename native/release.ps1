$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cmd /c ".\low.cmd build --release"
if ($LASTEXITCODE -ne 0) { throw "compilation de l'app" }
Push-Location setup
cmd /c "..\low.cmd build --release"
if ($LASTEXITCODE -ne 0) { throw "compilation de l'installeur" }
Pop-Location
New-Item -ItemType Directory -Force "release" | Out-Null
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$name = "Kairo_${version}_x64-setup.exe"
$pack = Start-Process -FilePath "setup\target\release\kairo-setup.exe" -ArgumentList "--pack", "target\release\ethra.exe", "release\$name" -Wait -PassThru
if ($pack.ExitCode -ne 0) { throw "paquet de l'installeur" }
"{0} : {1:N1} Mo" -f "ethra.exe", ((Get-Item target\release\ethra.exe).Length / 1MB)
"{0} : {1:N1} Mo" -f $name, ((Get-Item "release\$name").Length / 1MB)
(Resolve-Path "release\$name").Path
