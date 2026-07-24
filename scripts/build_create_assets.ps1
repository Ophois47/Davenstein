$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot

Push-Location $RepoRoot

try {
    Write-Host "##=>> Building Davenstein ..."
    # --locked Forbids Cargo From Rewriting Cargo.lock During a Build, so Routine
    # Builds Are Reproducible and Never Dirty the Lockfile (the Cause of the
    # Cross-Machine "cannot pull" Collisions). To Change Dependency Versions, Run
    # `cargo update` Deliberately, Commit the New Cargo.lock, Then Pull Everywhere.
    # This Matches the --locked Builds CI and the Flatpak Manifest Already Use
    cargo build --release --locked

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    Write-Host
    Write-Host "##=>> Building assets.pak ..."
    cargo run --release --locked --bin pak_builder -- `
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
