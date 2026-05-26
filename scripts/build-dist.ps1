#!/usr/bin/env pwsh
# Build a distributable iAgent binary using the release-lto profile.
# Usage: ./scripts/build-dist.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$env:IAGENT_RELEASE_BUILD = "1"

Write-Host "Building iAgent distribution binary (release-lto profile)..."
cargo build --profile release-lto --bin iagent

$out = "target\release-lto\iagent.exe"
if (-not (Test-Path $out)) {
    Write-Error "Build output not found at $out"
    exit 1
}

$size = (Get-Item $out).Length / 1MB
Write-Host "Done. Binary: $out ($([math]::Round($size, 2)) MB)"
