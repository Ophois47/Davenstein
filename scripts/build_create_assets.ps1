$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot

Push-Location $RepoRoot

try {
    Write-Host "##=>> Building Davenstein ..."
    cargo build --release

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    Write-Host
    Write-Host "##=>> Building assets.pak ..."
    cargo run --release --bin pak_builder -- `
        --root assets `
        --out target/release/assets.pak

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    Write-Host
    Write-Host "##=>> Davenstein Has Been Built Successfully!"
    Write-Host "##=>> Run: target\release\Davenstein.exe"
}
finally {
    Pop-Location
}
