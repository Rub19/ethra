$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
cmd /c ".\low.cmd build --release"
if ($LASTEXITCODE -ne 0) { throw "compilation de l'app" }
New-Item -ItemType Directory -Force "setup\payload", "release" | Out-Null
$in = [IO.File]::OpenRead((Resolve-Path "target\release\ethra.exe"))
$out = [IO.File]::Create("$PSScriptRoot\setup\payload\ethra.exe.gz")
$gz = New-Object IO.Compression.GZipStream($out, [IO.Compression.CompressionLevel]::Optimal)
$in.CopyTo($gz)
$gz.Close(); $out.Close(); $in.Close()
Push-Location setup
cmd /c "..\low.cmd build --release"
if ($LASTEXITCODE -ne 0) { throw "compilation de l'installeur" }
Pop-Location
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
$name = "Kairo_${version}_x64-setup.exe"
Copy-Item "setup\target\release\kairo-setup.exe" "release\$name" -Force
"{0} : {1:N1} Mo" -f "ethra.exe", ((Get-Item target\release\ethra.exe).Length / 1MB)
"{0} : {1:N1} Mo" -f $name, ((Get-Item "release\$name").Length / 1MB)
(Resolve-Path "release\$name").Path
