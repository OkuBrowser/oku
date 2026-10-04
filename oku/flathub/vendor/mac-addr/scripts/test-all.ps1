#!/usr/bin/env pwsh
# PowerShell 7+ recommended

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Run-Pair {
    param(
        [string[]]$CargoArgs = @()
    )

    Write-Host ""
    Write-Host "=== cargo build $($CargoArgs -join ' ') ===" -ForegroundColor Cyan
    cargo build @CargoArgs

    Write-Host "=== cargo test  $($CargoArgs -join ' ') ===" -ForegroundColor Yellow
    cargo test  @CargoArgs
}

# 1) default (std)
Run-Pair @()

# 2) no_std
Run-Pair @('--no-default-features')

# 3) alloc only
Run-Pair @('--no-default-features', '--features', 'alloc')

# 4) serde only
Run-Pair @('--no-default-features', '--features', 'serde')

# 5) alloc + serde
Run-Pair @('--no-default-features', '--features', 'alloc,serde')

# 6) all features
Run-Pair @('--all-features')
