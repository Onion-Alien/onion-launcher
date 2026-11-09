# Release build with no local paths inside the exe. Rust keeps the source path of every
# panic message, and the paths of downloaded crates contain the builder's user name; here
# they become neutral names. Then the exe is checked for the user name and home folder.
#   powershell -File scripts/build-release.ps1 [extra cargo args]
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
$rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $HOME '.rustup' }
# One flag per entry (separated by 0x1f), so paths with spaces stay whole.
$env:CARGO_ENCODED_RUSTFLAGS = @(
    "--remap-path-prefix=$cargoHome=cargo",
    "--remap-path-prefix=$rustupHome=rustup",
    "--remap-path-prefix=$root=onion-launcher"
) -join [char]0x1f
Push-Location $root
try {
    cargo build --release @args
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
} finally { Pop-Location }

$exe = Join-Path $root 'target\release\OnionLauncher.exe'
$bytes = [IO.File]::ReadAllBytes($exe)
$text = [Text.Encoding]::ASCII.GetString($bytes) + [Text.Encoding]::Unicode.GetString($bytes)
$leaks = @($env:USERNAME, $HOME, $cargoHome, $rustupHome, $root) | Where-Object { $_ -and $text.IndexOf($_, [StringComparison]::OrdinalIgnoreCase) -ge 0 }
if ($leaks) { Write-Error "OnionLauncher.exe still contains: $($leaks -join ', ')" }
$hash = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
"$exe`nSHA-256 $hash`nno local paths inside"
