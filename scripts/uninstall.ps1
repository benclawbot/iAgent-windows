<#
.SYNOPSIS
    Uninstall iAgent from Windows.
.DESCRIPTION
    Removes installed binaries, shortcuts, startup entries, and optional user data.
#>
param(
    [switch]$RemoveUserData,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Write-Info($msg) { Write-Host $msg -ForegroundColor Blue }
function Write-Warn($msg) { Write-Host "warning: $msg" -ForegroundColor Yellow }

$installDir = Join-Path $env:LOCALAPPDATA "iAgent\bin"
$appDir = Join-Path $env:LOCALAPPDATA "iAgent\app"
$buildsDir = Join-Path $env:LOCALAPPDATA "iAgent\builds"
$startupDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"
$desktopDir = [Environment]::GetFolderPath("Desktop")
$userHome = if ($env:IAGENT_HOME) { $env:IAGENT_HOME } else { Join-Path $env:USERPROFILE ".iagent" }

$targets = @(
    (Join-Path $installDir "iagent.exe"),
    (Join-Path $installDir "launch-iagent-dock.ps1"),
    (Join-Path $installDir "launch-iagent-dock.vbs"),
    (Join-Path $startupDir "iagent-hotkey.lnk"),
    (Join-Path $startupDir "iagent-personal-daemon.lnk"),
    (Join-Path $desktopDir "iAgent.lnk")
)

if (-not $Force) {
    Write-Host "This will uninstall iAgent binaries and launchers from this machine."
    $confirm = Read-Host "Type YES to continue"
    if ($confirm -ne "YES") {
        Write-Info "Aborted."
        exit 0
    }
}

foreach ($target in $targets) {
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue
        Write-Info "Removed: $target"
    }
}

foreach ($dir in @($appDir, $installDir, $buildsDir)) {
    if (Test-Path -LiteralPath $dir) {
        Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Info "Removed: $dir"
    }
}

if ($RemoveUserData) {
    if (Test-Path -LiteralPath $userHome) {
        Remove-Item -LiteralPath $userHome -Recurse -Force -ErrorAction SilentlyContinue
        Write-Info "Removed user data: $userHome"
    }
} else {
    Write-Warn "User data retained at: $userHome"
    Write-Warn "Re-run with -RemoveUserData to delete config/logs/memory."
}

Write-Info "iAgent uninstall complete."
