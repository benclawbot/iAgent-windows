<#
.SYNOPSIS
    Install iAgent on Windows.
.DESCRIPTION
    Downloads the latest iAgent release and installs it to %LOCALAPPDATA%\iAgent\bin.

    One-liner install:
      irm "https://raw.githubusercontent.com/benclawbot/iAgent-windows/main/scripts/install.ps1?v=dock" | iex

    Or download and run (allows parameters):
      & ([scriptblock]::Create((irm "https://raw.githubusercontent.com/benclawbot/iAgent-windows/main/scripts/install.ps1?v=dock")))
.PARAMETER InstallDir
    Override the installation directory (default: $env:LOCALAPPDATA\iAgent\bin)
.PARAMETER Version
    Override the version tag to install. Required when using a local artifact path.
.PARAMETER ArtifactExePath
    Use a local iagent.exe artifact instead of downloading from GitHub.
.PARAMETER ArtifactTgzPath
    Use a local iagent .tar.gz artifact instead of downloading from GitHub.
.PARAMETER SkipAlacrittySetup
    Skip Alacritty install/setup helpers.
.PARAMETER SkipHotkeySetup
    Skip Alt+; hotkey setup helpers.
.PARAMETER SkipPersonalDaemonSetup
    Skip personal ambient daemon startup helper.
.PARAMETER SkipDesktopShortcut
    Skip creating the iAgent desktop shortcut.
.PARAMETER SkipDockSetup
    Skip downloading and setting up the desktop dock frontend.
#>
param(
    [string]$InstallDir,
    [string]$AppDir,
    [string]$Version,
    [string]$ArtifactExePath,
    [string]$ArtifactTgzPath,
    [switch]$SkipAlacrittySetup,
    [switch]$SkipHotkeySetup,
    [switch]$SkipPersonalDaemonSetup,
    [switch]$SkipDesktopShortcut,
    [switch]$SkipDockSetup
)

$ErrorActionPreference = 'Stop'

if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Host "error: PowerShell 5.1 or later is required" -ForegroundColor Red
    exit 1
}

$Repo = "benclawbot/iAgent-windows"

if (-not $InstallDir) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "iAgent\bin"
}

if (-not $AppDir) {
    $AppDir = Join-Path $env:LOCALAPPDATA "iAgent\app"
}

$IagentHome = if ($env:IAGENT_HOME) {
    $env:IAGENT_HOME
} elseif ($env:USERPROFILE) {
    Join-Path $env:USERPROFILE ".iagent"
} else {
    Join-Path ([Environment]::GetFolderPath("UserProfile")) ".iagent"
}

$HotkeyDir = Join-Path $IagentHome "hotkey"
$PersonalDaemonDir = Join-Path $IagentHome "personal-daemon"
$SetupHintsPath = Join-Path $IagentHome "setup_hints.json"

function Write-Info($msg) { Write-Host $msg -ForegroundColor Blue }
function Write-Err($msg) { Write-Host "error: $msg" -ForegroundColor Red; exit 1 }
function Write-Warn($msg) { Write-Host "warning: $msg" -ForegroundColor Yellow }

function Get-ExpectedChecksumFromText([string]$ChecksumsText, [string]$AssetName) {
    foreach ($line in ($ChecksumsText -split "`n")) {
        $trimmed = $line.Trim()
        if (-not $trimmed) { continue }
        if ($trimmed -match '^([A-Fa-f0-9]{64})\s+\*?(.+)$') {
            $hash = $matches[1].ToLowerInvariant()
            $name = $matches[2].Trim()
            if ($name -eq $AssetName) {
                return $hash
            }
        }
    }
    return $null
}

function Verify-DownloadedChecksum([string]$FilePath, [string]$AssetName, [string]$ChecksumsUrl) {
    Write-Info "Verifying SHA256 for $AssetName..."
    try {
        $checksumsText = Invoke-WebRequest -Uri $ChecksumsUrl -UseBasicParsing -TimeoutSec 30 | Select-Object -ExpandProperty Content
    } catch {
        Write-Err "Failed to download checksum file: $ChecksumsUrl"
    }

    $expected = Get-ExpectedChecksumFromText -ChecksumsText $checksumsText -AssetName $AssetName
    if (-not $expected) {
        Write-Err "No checksum entry found for $AssetName in checksums.txt"
    }

    $actual = (Get-FileHash -Algorithm SHA256 -Path $FilePath).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Write-Err "Checksum mismatch for $AssetName. Expected $expected, got $actual"
    }
    Write-Info "Checksum verified."
}

function Get-LatestReleaseWithRetry([string]$Repository, [int]$MaxAttempts = 4, [int]$BaseDelaySeconds = 2) {
    $lastError = $null
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            return Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" -TimeoutSec 20 -Headers @{
                "User-Agent" = "iAgent-Windows-Installer"
                "Accept" = "application/vnd.github+json"
            }
        } catch {
            $lastError = $_
            if ($attempt -lt $MaxAttempts) {
                $delay = [Math]::Min(30, [int]($BaseDelaySeconds * [Math]::Pow(2, $attempt - 1)))
                Write-Warn "Latest-release lookup attempt $attempt/$MaxAttempts failed: $($_.Exception.Message). Retrying in ${delay}s..."
                Start-Sleep -Seconds $delay
            }
        }
    }

    throw $lastError
}

# ── OfficeCLI ──────────────────────────────────────────────────────────────────
# Office document automation for Word (.docx), Excel (.xlsx), PowerPoint (.pptx).
# Installed to $InstallDir so it's alongside iagent.exe and already on PATH.
function Install-OfficeCLI {
    Write-Info "Installing OfficeCLI..."
    $cliDest = Join-Path $InstallDir "officecli.exe"

    if ($IsWindows -or (-not (Test-Path variable:IsWindows) -and $env:OS -eq "Windows_NT")) {
        # Windows
        try {
            $response = Invoke-RestMethod -Uri "https://api.github.com/repos/iOfficeAI/OfficeCLI/releases/latest" -TimeoutSec 15
            $asset = $response.assets | Where-Object { $_.name -eq "officecli-win-x64.exe" } | Select-Object -First 1
            if ($null -eq $asset) { throw "No win-x64 asset found in latest release" }
            Write-Info "  Downloading $($asset.name)..."
            Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $cliDest -TimeoutSec 120
        } catch {
            Write-Warn "OfficeCLI download failed: $_"
            Write-Info "  To install manually: irm https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.ps1 | iex"
        }
    } else {
        # macOS / Linux
        Write-Info "  OfficeCLI: run the install command for your platform"
        Write-Info "    curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash"
    }

    if (Test-Path $cliDest) {
        Write-Info "OfficeCLI installed: $cliDest"
        Write-Info "  Verify: $cliDest --version"
    }
}

function Test-OfficeCLIInstalled {
    $cliPath = Join-Path $InstallDir "officecli.exe"
    if (Test-Path $cliPath) { return $true }
    $found = Get-Command officecli -ErrorAction SilentlyContinue
    return ($null -ne $found)
}

function Resolve-OptionalPath([string]$PathValue) {
    if (-not $PathValue) {
        return $null
    }

    try {
        return (Resolve-Path -LiteralPath $PathValue -ErrorAction Stop).Path
    } catch {
        Write-Err "Provided path does not exist: $PathValue"
    }
}

function Stop-ProcessTree([int]$ProcessId) {
    try {
        Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object { $_.ParentProcessId -eq $ProcessId } |
            ForEach-Object { Stop-ProcessTree -ProcessId $_.ProcessId }
    } catch {}

    try {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    } catch {}
}

function Invoke-ProcessWithTimeout {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [Parameter(Mandatory = $true)][string]$FriendlyName,
        [switch]$CaptureOutput
    )

    $startParams = @{
        FilePath = $FilePath
        ArgumentList = $ArgumentList
        PassThru = $true
        NoNewWindow = $true
    }

    $stdoutPath = $null
    $stderrPath = $null
    if ($CaptureOutput) {
        $stdoutPath = Join-Path $env:TEMP ("iagent-{0}-{1}-stdout.log" -f $FriendlyName, [guid]::NewGuid().ToString('N'))
        $stderrPath = Join-Path $env:TEMP ("iagent-{0}-{1}-stderr.log" -f $FriendlyName, [guid]::NewGuid().ToString('N'))
        $startParams.RedirectStandardOutput = $stdoutPath
        $startParams.RedirectStandardError = $stderrPath
    }

    $process = Start-Process @startParams
    $null = Wait-Process -Id $process.Id -Timeout $TimeoutSeconds -ErrorAction SilentlyContinue
    $process.Refresh()
    $timedOut = -not $process.HasExited
    if ($timedOut) {
        Stop-ProcessTree -ProcessId $process.Id
        return [pscustomobject]@{
            TimedOut = $true
            ExitCode = $null
            StdoutPath = $stdoutPath
            StderrPath = $stderrPath
        }
    }

    $process.Refresh()
    $exitCode = $process.ExitCode
    if ($null -eq $exitCode -or "$exitCode" -eq "") {
        $exitCode = 0
    }

    return [pscustomobject]@{
        TimedOut = $false
        ExitCode = $exitCode
        StdoutPath = $stdoutPath
        StderrPath = $stderrPath
    }
}

function Write-LogTail([string]$Path, [string]$Label) {
    if (-not $Path -or -not (Test-Path $Path)) {
        return
    }

    $lines = Get-Content -Path $Path -Tail 40 -ErrorAction SilentlyContinue
    if ($lines -and $lines.Count -gt 0) {
        Write-Warn "$Label (last 40 lines):"
        $lines | ForEach-Object { Write-Host $_ }
    }
}

function Test-CommandExists([string]$CommandName) {
    return [bool](Get-Command $CommandName -ErrorAction SilentlyContinue)
}

function Test-AlacrittyInstalled {
    return [bool](Find-AlacrittyPath)
}

function Find-AlacrittyPath {
    $candidates = @(
        "C:\Program Files\Alacritty\alacritty.exe",
        "C:\Program Files (x86)\Alacritty\alacritty.exe"
    )

    if ($env:LOCALAPPDATA) {
        $candidates += (Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links\alacritty.exe")
    }

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }

    try {
        $command = Get-Command alacritty -ErrorAction Stop
        if ($command -and $command.Source) {
            return $command.Source
        }
    } catch {}

    return $null
}

function Install-Alacritty {
    if (Test-AlacrittyInstalled) {
        Write-Info "Alacritty is already installed"
        return $true
    }

    if (-not (Test-CommandExists "winget")) {
        Write-Warn "winget was not found, so Alacritty could not be installed automatically"
        Write-Warn "Install App Installer / winget from Microsoft, then run: winget install -e --id Alacritty.Alacritty"
        return $false
    }

    Write-Info "Installing Alacritty..."
    $wingetArgs = @(
        "install",
        "-e",
        "--id", "Alacritty.Alacritty",
        "--accept-source-agreements",
        "--accept-package-agreements",
        "--disable-interactivity"
    )

    $wingetResult = Invoke-ProcessWithTimeout -FilePath "winget" -ArgumentList $wingetArgs -TimeoutSeconds 180 -FriendlyName "winget-install"
    if ($wingetResult.TimedOut) {
        Write-Warn "Alacritty install timed out after 180 seconds; skipping automatic setup"
        return $false
    }

    if ($wingetResult.ExitCode -ne 0) {
        Write-Warn "Alacritty install failed (winget exit code: $($wingetResult.ExitCode))"
        return $false
    }

    $alacrittyPath = Find-AlacrittyPath
    if (-not $alacrittyPath) {
        Write-Warn "Alacritty install finished, but alacritty.exe was not found on PATH yet"
        return $false
    }

    Write-Info "Alacritty installed: $alacrittyPath"
    return $true
}

function Stop-IagentHotkeyListeners {
    try {
        Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe' OR Name = 'pwsh.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.CommandLine -like '*iagent-hotkey*' } |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    } catch {}
}

function Set-SetupHintsState([bool]$AlacrittyConfigured, [bool]$HotkeyConfigured) {
    New-Item -ItemType Directory -Path $IagentHome -Force | Out-Null

    $state = @{
        launch_count = 0
        hotkey_configured = $HotkeyConfigured
        hotkey_dismissed = $HotkeyConfigured
        alacritty_configured = $AlacrittyConfigured
        alacritty_dismissed = $AlacrittyConfigured
        desktop_shortcut_created = $false
        mac_ghostty_guided = $false
        mac_ghostty_dismissed = $false
    }

    if (Test-Path $SetupHintsPath) {
        try {
            $existing = Get-Content $SetupHintsPath -Raw | ConvertFrom-Json -ErrorAction Stop
            foreach ($property in $existing.PSObject.Properties) {
                $state[$property.Name] = $property.Value
            }
        } catch {
            Write-Warn "Could not read existing setup hints state; overwriting it"
        }
    }

    if ($AlacrittyConfigured) {
        $state.alacritty_configured = $true
        $state.alacritty_dismissed = $true
    }

    if ($HotkeyConfigured) {
        $state.hotkey_configured = $true
        $state.hotkey_dismissed = $true
    }

    $state | ConvertTo-Json | Set-Content -Path $SetupHintsPath -Encoding UTF8
}

function Install-IagentHotkey([string]$DockLauncherVbsPath) {
    if (-not (Test-Path -LiteralPath $DockLauncherVbsPath)) {
        Write-Warn "Skipping Alt+; hotkey because dock launcher was not found"
        return $false
    }

    New-Item -ItemType Directory -Path $HotkeyDir -Force | Out-Null
    Stop-IagentHotkeyListeners

    $escapedDockLauncher = $DockLauncherVbsPath.Replace("'", "''")

    $ps1Path = Join-Path $HotkeyDir "iagent-hotkey.ps1"
    $ps1Lines = @(
        '# iAgent Alt+; global hotkey listener',
        '# Auto-generated by scripts/install.ps1. Runs at login via startup shortcut.',
        '',
        'Add-Type @"',
        'using System;',
        'using System.Runtime.InteropServices;',
        'public class HotKeyHelper {',
        '    [DllImport("user32.dll")]',
        '    public static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);',
        '    [DllImport("user32.dll")]',
        '    public static extern bool UnregisterHotKey(IntPtr hWnd, int id);',
        '    [DllImport("user32.dll")]',
        '    public static extern int GetMessage(out MSG lpMsg, IntPtr hWnd, uint wMsgFilterMin, uint wMsgFilterMax);',
        '    [StructLayout(LayoutKind.Sequential)]',
        '    public struct MSG {',
        '        public IntPtr hwnd;',
        '        public uint message;',
        '        public IntPtr wParam;',
        '        public IntPtr lParam;',
        '        public uint time;',
        '        public int pt_x;',
        '        public int pt_y;',
        '    }',
        '}',
        '"@',
        '',
        '$MOD_ALT = 0x0001',
        '$MOD_NOREPEAT = 0x4000',
        '$VK_OEM_1 = 0xBA',
        '$WM_HOTKEY = 0x0312',
        '$HOTKEY_ID = 0x4A43',
        '',
        'if (-not [HotKeyHelper]::RegisterHotKey([IntPtr]::Zero, $HOTKEY_ID, $MOD_ALT -bor $MOD_NOREPEAT, $VK_OEM_1)) {',
        '    Write-Error "Failed to register Alt+; hotkey (another program may have claimed it)"',
        '    exit 1',
        '}',
        '',
        'try {',
        '    $msg = New-Object HotKeyHelper+MSG',
        '    while ([HotKeyHelper]::GetMessage([ref]$msg, [IntPtr]::Zero, $WM_HOTKEY, $WM_HOTKEY) -ne 0) {',
        '        if ($msg.message -eq $WM_HOTKEY -and $msg.wParam.ToInt32() -eq $HOTKEY_ID) {',
        "            Start-Process 'wscript.exe' -ArgumentList '`"$escapedDockLauncher`"'",
        '        }',
        '    }',
        '} finally {',
        '    [HotKeyHelper]::UnregisterHotKey([IntPtr]::Zero, $HOTKEY_ID)',
        '}'
    )
    $ps1Content = $ps1Lines -join "`r`n"
    Set-Content -Path $ps1Path -Value $ps1Content -Encoding UTF8

    $vbsPath = Join-Path $HotkeyDir "iagent-hotkey-launcher.vbs"
    $vbsContent = @(
        'Set objShell = CreateObject("WScript.Shell")',
        ('objShell.Run "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{0}""", 0, False' -f $ps1Path)
    ) -join "`r`n"
    Set-Content -Path $vbsPath -Value $vbsContent -Encoding ASCII

    $startupDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"
    New-Item -ItemType Directory -Path $startupDir -Force | Out-Null
    $startupShortcutPath = (Join-Path $startupDir "iagent-hotkey.lnk").Replace("'", "''")
    $escapedVbsPath = $vbsPath.Replace("'", "''")

    $shortcutLines = @(
        '$shell = New-Object -ComObject WScript.Shell',
        "`$shortcut = `$shell.CreateShortcut('$startupShortcutPath')",
        "`$shortcut.TargetPath = 'wscript.exe'",
        ("`$shortcut.Arguments = '""{0}""'" -f $escapedVbsPath),
        "`$shortcut.Description = 'iAgent Alt+; hotkey listener'",
        '$shortcut.WindowStyle = 7',
        '$shortcut.Save()',
        "Write-Output 'OK'"
    )
    $shortcutScript = $shortcutLines -join "`r`n"

    $shortcutOutput = & powershell -NoProfile -Command $shortcutScript
    if ($LASTEXITCODE -ne 0 -or -not ($shortcutOutput -match 'OK')) {
        Write-Warn "Created hotkey files, but could not create the Startup shortcut"
        return $false
    }

    $launchHotkeyCommand = "Start-Process wscript.exe -ArgumentList '""{0}""' -WindowStyle Hidden" -f $vbsPath
    & powershell -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -Command $launchHotkeyCommand | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Hotkey will start on next login, but could not be launched immediately"
    }

    Write-Info "Configured Alt+; to launch the iAgent dock"
    return $true
}

function Install-IagentPersonalDaemon([string]$IagentExePath) {
    if (-not (Test-Path -LiteralPath $IagentExePath)) {
        Write-Warn "Skipping personal daemon because iagent.exe was not found"
        return $false
    }

    New-Item -ItemType Directory -Path $PersonalDaemonDir -Force | Out-Null

    $ps1Path = Join-Path $PersonalDaemonDir "iagent-personal-daemon.ps1"
    $logDir = Join-Path $env:LOCALAPPDATA "iAgent\logs"
    $escapedExe = $IagentExePath.Replace("'", "''")
    $escapedLogDir = $logDir.Replace("'", "''")
    $ps1Content = @(
        '$ErrorActionPreference = "Continue"',
        ('$iagent = ''{0}''' -f $escapedExe),
        ('$logDir = ''{0}''' -f $escapedLogDir),
        'New-Item -ItemType Directory -Path $logDir -Force | Out-Null',
        '$logPath = Join-Path $logDir "personal-daemon.log"',
        'while ($true) {',
        '    try {',
        '        & $iagent personal-daemon --headless *> $logPath',
        '    } catch {',
        '        Add-Content -Path $logPath -Value "[$(Get-Date -Format o)] $($_.Exception.Message)"',
        '    }',
        '    Start-Sleep -Seconds 10',
        '}'
    ) -join "`r`n"
    Set-Content -Path $ps1Path -Value $ps1Content -Encoding UTF8

    $vbsPath = Join-Path $PersonalDaemonDir "iagent-personal-daemon-launcher.vbs"
    $escapedPs1Path = $ps1Path.Replace('"', '""')
    $vbsContent = @(
        'Set objShell = CreateObject("WScript.Shell")',
        ('objShell.Run "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{0}""", 0, False' -f $escapedPs1Path)
    ) -join "`r`n"
    Set-Content -Path $vbsPath -Value $vbsContent -Encoding ASCII

    $startupDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"
    New-Item -ItemType Directory -Path $startupDir -Force | Out-Null
    $startupShortcutPath = (Join-Path $startupDir "iagent-personal-daemon.lnk").Replace("'", "''")
    $escapedVbsPath = $vbsPath.Replace("'", "''")
    $shortcutLines = @(
        '$shell = New-Object -ComObject WScript.Shell',
        "`$shortcut = `$shell.CreateShortcut('$startupShortcutPath')",
        "`$shortcut.TargetPath = 'wscript.exe'",
        ("`$shortcut.Arguments = '""{0}""'" -f $escapedVbsPath),
        "`$shortcut.Description = 'iAgent personal ambient daemon'",
        '$shortcut.WindowStyle = 7',
        '$shortcut.Save()',
        "Write-Output 'OK'"
    )
    $shortcutOutput = & powershell -NoProfile -Command ($shortcutLines -join "`r`n")
    if ($LASTEXITCODE -ne 0 -or -not ($shortcutOutput -match 'OK')) {
        Write-Warn "Created personal daemon files, but could not create the Startup shortcut"
        return $false
    }

    Write-Info "Configured personal ambient daemon startup"
    return $true
}

function New-IagentRobotIcon([string]$IconPath) {
    try {
        Add-Type -AssemblyName System.Drawing -ErrorAction Stop

        $bitmap = New-Object System.Drawing.Bitmap 64, 64
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.Clear([System.Drawing.Color]::Transparent)

        $bodyBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 34, 197, 94))
        $faceBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 15, 23, 42))
        $eyeBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
        $antennaPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 34, 197, 94)), 4

        $graphics.DrawLine($antennaPen, 32, 14, 32, 8)
        $graphics.FillEllipse($bodyBrush, 28, 3, 8, 8)
        $graphics.FillRectangle($bodyBrush, 14, 16, 36, 38)
        $graphics.FillEllipse($bodyBrush, 10, 16, 12, 12)
        $graphics.FillEllipse($bodyBrush, 42, 16, 12, 12)
        $graphics.FillEllipse($bodyBrush, 10, 42, 12, 12)
        $graphics.FillEllipse($bodyBrush, 42, 42, 12, 12)
        $graphics.FillRectangle($bodyBrush, 10, 22, 44, 26)
        $graphics.FillRectangle($faceBrush, 16, 24, 32, 18)
        $graphics.FillEllipse($faceBrush, 16, 24, 8, 8)
        $graphics.FillEllipse($faceBrush, 40, 24, 8, 8)
        $graphics.FillEllipse($faceBrush, 16, 34, 8, 8)
        $graphics.FillEllipse($faceBrush, 40, 34, 8, 8)
        $graphics.FillEllipse($eyeBrush, 23, 30, 6, 6)
        $graphics.FillEllipse($eyeBrush, 35, 30, 6, 6)
        $graphics.FillRectangle($bodyBrush, 20, 54, 8, 6)
        $graphics.FillRectangle($bodyBrush, 36, 54, 8, 6)

        $pngStream = New-Object System.IO.MemoryStream
        $bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngBytes = $pngStream.ToArray()

        $iconDir = [System.IO.Path]::GetDirectoryName($IconPath)
        if (-not (Test-Path $iconDir)) {
            New-Item -ItemType Directory -Path $iconDir -Force | Out-Null
        }

        $fs = [System.IO.File]::Open($IconPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
        $writer = New-Object System.IO.BinaryWriter($fs)
        $writer.Write([UInt16]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]1)
        $writer.Write([Byte]64)
        $writer.Write([Byte]64)
        $writer.Write([Byte]0)
        $writer.Write([Byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$pngBytes.Length)
        $writer.Write([UInt32]22)
        $writer.Write($pngBytes)
        $writer.Close()
        $fs.Close()

        $graphics.Dispose()
        $bitmap.Dispose()
        $pngStream.Dispose()
        return $true
    } catch {
        Write-Warn "Could not create robot icon: $($_.Exception.Message)"
        return $false
    }
}

function Install-IagentDesktopShortcut([string]$DockLauncherVbsPath, [string]$IconPath, [string]$WorkingDirectory) {
    try {
        $desktop = [Environment]::GetFolderPath("DesktopDirectory")
        if (-not $desktop) {
            Write-Warn "Desktop folder was not found; skipping desktop shortcut"
            return $false
        }

        $shortcutPath = Join-Path $desktop "iAgent.lnk"
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = "wscript.exe"
        $shortcut.Arguments = ('"{0}"' -f $DockLauncherVbsPath)
        $shortcut.WorkingDirectory = $WorkingDirectory
        $shortcut.Description = "Start the iAgent desktop dock"
        if (Test-Path $IconPath) {
            $shortcut.IconLocation = $IconPath
        }
        $shortcut.Save()

        Write-Info "Desktop shortcut ready: $shortcutPath"
        return $true
    } catch {
        Write-Warn "Could not create desktop shortcut: $($_.Exception.Message)"
        return $false
    }
}

function Install-UvIfMissing {
    $uv = Get-Command uv -ErrorAction SilentlyContinue
    if ($uv) {
        return $uv.Source
    }

    Write-Info "Installing uv Python runtime manager..."
    $uvTempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("iagent-uv-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $uvTempDir -Force | Out-Null
    $uvInstaller = Join-Path $uvTempDir "install-uv.ps1"

    try {
        Invoke-WebRequest -Uri "https://astral.sh/uv/install.ps1" -OutFile $uvInstaller -UseBasicParsing
        $uvInstallResult = Invoke-ProcessWithTimeout -FilePath "powershell.exe" -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            $uvInstaller
        ) -TimeoutSeconds 300 -FriendlyName "uv install" -CaptureOutput
        Write-LogTail -Path $uvInstallResult.StdoutPath -Label "uv install stdout"
        Write-LogTail -Path $uvInstallResult.StderrPath -Label "uv install stderr"
    } finally {
        Remove-Item -Path $uvTempDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    $uvCandidateDirs = @(
        (Join-Path $env:USERPROFILE ".local\bin"),
        (Join-Path $env:USERPROFILE ".cargo\bin")
    )
    foreach ($candidateDir in $uvCandidateDirs) {
        $candidate = Join-Path $candidateDir "uv.exe"
        if (Test-Path -LiteralPath $candidate) {
            $env:Path = "$candidateDir;$env:Path"
            return $candidate
        }
    }

    $uv = Get-Command uv -ErrorAction SilentlyContinue
    if ($uv) {
        return $uv.Source
    }

    Write-Err "uv installation completed but uv.exe was not found on PATH"
}

function Stop-IagentDockProcesses([string]$TargetAppDir) {
    $patterns = @(
        "*$TargetAppDir*",
        "*launch-iagent-dock*",
        "*launch-iagent.ps1*"
    )

    try {
        $processes = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object {
                $commandLine = $_.CommandLine
                if (-not $commandLine) {
                    $false
                } else {
                    $matched = $false
                    foreach ($pattern in $patterns) {
                        if ($commandLine -like $pattern) {
                            $matched = $true
                            break
                        }
                    }
                    $matched
                }
            }

        if ($processes) {
            $processes | ForEach-Object {
                Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            }
            Start-Sleep -Seconds 1
        }
    } catch {
        Write-Warn "Could not stop existing iAgent dock processes: $($_.Exception.Message)"
    }
}

function Install-IagentDockApp([string]$TargetAppDir) {
    Write-Info "Installing iAgent desktop dock..."

    $frontendZipUrl = "https://github.com/$Repo/archive/refs/heads/main.zip"
    $dockTempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("iagent-dock-" + [Guid]::NewGuid().ToString("N"))
    $dockZipPath = Join-Path $dockTempDir "dock-main.zip"
    $extractDir = Join-Path $dockTempDir "extract"

    New-Item -ItemType Directory -Path $dockTempDir -Force | Out-Null
    try {
        Invoke-WebRequest -Uri $frontendZipUrl -OutFile $dockZipPath -UseBasicParsing
        Expand-Archive -Path $dockZipPath -DestinationPath $extractDir -Force

        $zipRoot = Get-ChildItem -LiteralPath $extractDir -Directory | Select-Object -First 1
        if (-not $zipRoot) {
            Write-Err "Downloaded dock archive could not be extracted"
        }
        $extractedRoot = $zipRoot.FullName

        # New repo layout: app/iagent-py + app/launch-iagent.ps1
        # Legacy layout fallback: iagent-py + launch-iagent.ps1 at root.
        $dockSourceDir = Join-Path $extractedRoot "app"
        $usesLegacyLayout = $false
        if (-not (Test-Path -LiteralPath (Join-Path $dockSourceDir "iagent-py\pyproject.toml"))) {
            if (Test-Path -LiteralPath (Join-Path $extractedRoot "iagent-py\pyproject.toml")) {
                $dockSourceDir = $extractedRoot
                $usesLegacyLayout = $true
            } else {
                Write-Err "Downloaded dock archive did not contain iagent-py"
            }
        }
        if (-not (Test-Path -LiteralPath (Join-Path $dockSourceDir "launch-iagent.ps1"))) {
            Write-Err "Downloaded dock archive did not contain launch-iagent.ps1"
        }

        $targetParent = Split-Path -Parent $TargetAppDir
        New-Item -ItemType Directory -Path $targetParent -Force | Out-Null
        if (Test-Path -LiteralPath $TargetAppDir) {
            Stop-IagentDockProcesses -TargetAppDir $TargetAppDir
            Remove-Item -LiteralPath $TargetAppDir -Recurse -Force
        }
        if ($usesLegacyLayout) {
            Move-Item -LiteralPath $extractedRoot -Destination $TargetAppDir -Force
        } else {
            Move-Item -LiteralPath $dockSourceDir -Destination $TargetAppDir -Force
        }
    } finally {
        Remove-Item -Path $dockTempDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    $uvPath = Install-UvIfMissing
    $runtimeDir = Join-Path $TargetAppDir "iagent-py"
    $uvSyncResult = Invoke-ProcessWithTimeout -FilePath $uvPath -ArgumentList @(
        "sync",
        "--project",
        $runtimeDir
    ) -TimeoutSeconds 900 -FriendlyName "iAgent dock Python dependency install" -CaptureOutput
    Write-LogTail -Path $uvSyncResult.StdoutPath -Label "uv sync stdout"
    Write-LogTail -Path $uvSyncResult.StderrPath -Label "uv sync stderr"

    $workerDir = Join-Path $TargetAppDir "worker"
    $npmCommand = Get-Command npm.cmd -ErrorAction SilentlyContinue
    if (-not $npmCommand) {
        $npmCommand = Get-Command npm.exe -ErrorAction SilentlyContinue
    }

    if ((Test-Path -LiteralPath (Join-Path $workerDir "package.json")) -and $npmCommand) {
        $npmInstallResult = Invoke-ProcessWithTimeout -FilePath $npmCommand.Source -ArgumentList @(
            "install",
            "--prefix",
            $workerDir
        ) -TimeoutSeconds 900 -FriendlyName "iAgent worker dependency install" -CaptureOutput
        Write-LogTail -Path $npmInstallResult.StdoutPath -Label "npm install stdout"
        Write-LogTail -Path $npmInstallResult.StderrPath -Label "npm install stderr"
    } elseif (Test-Path -LiteralPath (Join-Path $workerDir "package.json")) {
        Write-Warn "npm was not found; worker dependencies were not installed. Dock launch still works unless worker_url is configured."
    }

    return $TargetAppDir
}

function New-IagentDockLauncher([string]$IagentExePath, [string]$TargetAppDir, [string]$TargetInstallDir) {
    $ps1Path = Join-Path $TargetInstallDir "launch-iagent-dock.ps1"
    $vbsPath = Join-Path $TargetInstallDir "launch-iagent-dock.vbs"
    $logDir = Join-Path $env:LOCALAPPDATA "iAgent\logs"

    $escapedIagentExe = $IagentExePath.Replace("'", "''")
    $escapedAppDir = $TargetAppDir.Replace("'", "''")
    $escapedLogDir = $logDir.Replace("'", "''")

    $ps1Content = @(
        '$ErrorActionPreference = "Stop"',
        ('$env:IAGENT_BIN = ''{0}''' -f $escapedIagentExe),
        ('$appDir = ''{0}''' -f $escapedAppDir),
        ('$logDir = ''{0}''' -f $escapedLogDir),
        'New-Item -ItemType Directory -Path $logDir -Force | Out-Null',
        '$logPath = Join-Path $logDir "dock-launch.log"',
        'try {',
        '    $launcher = Join-Path $appDir "launch-iagent.ps1"',
        '    if (-not (Test-Path -LiteralPath $launcher)) {',
        '        $legacyLauncher = Join-Path $env:USERPROFILE "iAgent\launch-iagent.ps1"',
        '        if (Test-Path -LiteralPath $legacyLauncher) {',
        '            $launcher = $legacyLauncher',
        '        }',
        '    }',
        '    if (-not (Test-Path -LiteralPath $launcher)) {',
        '        throw "iAgent dock launcher not found at $launcher"',
        '    }',
        '    & $launcher',
        '} catch {',
        '    $message = "[$(Get-Date -Format o)] $($_.Exception.Message)"',
        '    Add-Content -Path $logPath -Value $message',
        '    throw',
        '}'
    ) -join "`r`n"
    Set-Content -Path $ps1Path -Value $ps1Content -Encoding UTF8

    $escapedPs1Path = $ps1Path.Replace('"', '""')
    $vbsContent = @(
        'Set objShell = CreateObject("WScript.Shell")',
        ('objShell.Run "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{0}""", 0, False' -f $escapedPs1Path)
    ) -join "`r`n"
    Set-Content -Path $vbsPath -Value $vbsContent -Encoding ASCII

    return [pscustomobject]@{
        Ps1Path = $ps1Path
        VbsPath = $vbsPath
    }
}

function Write-DockPostInstallSelfCheck([string]$TargetInstallDir, [string]$TargetAppDir, $DockLauncher, [bool]$DockWasSkipped) {
    Write-Host ""
    Write-Info "Post-install self-check (dock runtime):"

    if ($DockWasSkipped) {
        Write-Info "  SKIPPED: Dock setup was skipped by parameter."
        return
    }

    $checks = @(
        @{
            Label = "dock launcher VBS"
            Path  = if ($DockLauncher) { $DockLauncher.VbsPath } else { $null }
        },
        @{
            Label = "dock launcher PS1"
            Path  = if ($DockLauncher) { $DockLauncher.Ps1Path } else { $null }
        },
        @{
            Label = "dock app launcher"
            Path  = Join-Path $TargetAppDir "launch-iagent.ps1"
        },
        @{
            Label = "python runtime project"
            Path  = Join-Path $TargetAppDir "iagent-py\pyproject.toml"
        },
        @{
            Label = "worker package"
            Path  = Join-Path $TargetAppDir "worker\package.json"
        }
    )

    $allOk = $true
    foreach ($check in $checks) {
        $path = $check.Path
        $ok = $path -and (Test-Path -LiteralPath $path)
        $status = if ($ok) { "OK" } else { "MISSING" }
        if (-not $ok) { $allOk = $false }
        Write-Host ("  {0}: {1} ({2})" -f $status, $check.Label, $path)
    }

    $launchScript = Join-Path $TargetAppDir "launch-iagent.ps1"
    if (Test-Path -LiteralPath $launchScript) {
        $launchText = Get-Content -LiteralPath $launchScript -Raw
        $hasUvLaunch = $launchText -match "run\s+python(\.exe)?\s+-m\s+iagent"
        $hasWorkerRef = $launchText -match "worker"
        Write-Host ("  {0}: launch command uses uv/python iagent" -f ($(if ($hasUvLaunch) { "OK" } else { "WARN" })))
        Write-Host ("  {0}: launch script references worker runtime" -f ($(if ($hasWorkerRef) { "OK" } else { "WARN" })))
        if (-not $hasUvLaunch) { $allOk = $false }
    }

    if ($allOk) {
        Write-Info "  Result: dock runtime layout looks complete."
    } else {
        Write-Warn "  Result: dock runtime layout is incomplete; review missing entries above."
    }
}

function Get-IagentWindowsArtifact {
    $candidates = @()

    try {
        $runtimeArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
        if ($runtimeArch) { $candidates += [string]$runtimeArch }
    } catch {}

    foreach ($envArch in @($env:PROCESSOR_ARCHITECTURE, $env:PROCESSOR_ARCHITEW6432)) {
        if ($envArch) { $candidates += [string]$envArch }
    }

    foreach ($arch in $candidates) {
        switch -Regex ($arch.Trim()) {
            '^(X64|AMD64|x86_64)$' { return "iagent-windows-x86_64" }
            '^(Arm64|ARM64|AARCH64|aarch64)$' { return "iagent-windows-aarch64" }
        }
    }

    $displayArch = if ($candidates.Count -gt 0) { $candidates -join ", " } else { "<unknown>" }
    Write-Err "Unsupported architecture: $displayArch (supported: x86_64, ARM64)"
}

$Artifact = Get-IagentWindowsArtifact

$ResolvedArtifactExePath = Resolve-OptionalPath $ArtifactExePath
$ResolvedArtifactTgzPath = Resolve-OptionalPath $ArtifactTgzPath

if ($ResolvedArtifactExePath -and $ResolvedArtifactTgzPath) {
    Write-Err "Provide only one of -ArtifactExePath or -ArtifactTgzPath"
}

if (-not $Version) {
    if ($ResolvedArtifactExePath -or $ResolvedArtifactTgzPath) {
        Write-Err "-Version is required when using a local artifact path"
    }

    Write-Info "Fetching latest release..."
    try {
        $Release = Get-LatestReleaseWithRetry -Repository $Repo
        $Version = $Release.tag_name
    } catch {
        Write-Warn "Failed to determine latest release version ($($_.Exception.Message))."
        Write-Warn "No published release found; falling back to source install from branch 'main'."
        $Version = "main"
    }
}

if (-not $Version) { Write-Err "Failed to determine latest version" }

$VersionNum = $Version.TrimStart('v')
$TgzUrl = "https://github.com/$Repo/releases/download/$Version/$Artifact.tar.gz"
$ExeUrl = "https://github.com/$Repo/releases/download/$Version/$Artifact.exe"
$ChecksumsUrl = "https://github.com/$Repo/releases/download/$Version/checksums.txt"

$BuildsDir = Join-Path $env:LOCALAPPDATA "iAgent\builds"
$StableDir = Join-Path $BuildsDir "stable"
$VersionDir = Join-Path $BuildsDir "versions\$VersionNum"
$LauncherPath = Join-Path $InstallDir "iagent.exe"
$IconPath = Join-Path $InstallDir "iagent-robot.ico"

$Existing = ""
if (Test-Path $LauncherPath) {
    try { $Existing = & $LauncherPath --version 2>$null | Select-Object -First 1 } catch {}
}

if ($Existing) {
    if ($Existing -match [regex]::Escape($VersionNum)) {
        Write-Info "iAgent $Version is already installed - reinstalling"
    } else {
        Write-Info "Updating iAgent $Existing -> $Version"
    }
} else {
    Write-Info "Installing iAgent $Version"
}
Write-Info "  launcher: $LauncherPath"

foreach ($d in @($InstallDir, $StableDir, $VersionDir)) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}

$TempDir = Join-Path $env:TEMP "iagent-install-$(Get-Random)"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

$DownloadMode = ""
$DownloadPath = Join-Path $TempDir "iagent.download"

if ($ResolvedArtifactExePath) {
    Write-Info "Using local artifact exe: $ResolvedArtifactExePath"
    Copy-Item -Path $ResolvedArtifactExePath -Destination $DownloadPath -Force
    $DownloadMode = "bin"
} elseif ($ResolvedArtifactTgzPath) {
    Write-Info "Using local artifact archive: $ResolvedArtifactTgzPath"
    Copy-Item -Path $ResolvedArtifactTgzPath -Destination $DownloadPath -Force
    $DownloadMode = "tar"
} else {
    try {
        Write-Info "Downloading $Artifact.exe..."
        Invoke-WebRequest -Uri $ExeUrl -OutFile $DownloadPath
        $DownloadMode = "bin"
    } catch {
        try {
            Write-Info "Trying archive download..."
            Invoke-WebRequest -Uri $TgzUrl -OutFile $DownloadPath
            $DownloadMode = "tar"
        } catch {
            $DownloadMode = ""
        }
    }
}

$shouldVerifyChecksum = (-not $ResolvedArtifactExePath) -and (-not $ResolvedArtifactTgzPath) -and ($Version -ne "main")
if ($shouldVerifyChecksum -and $DownloadMode) {
    $assetName = if ($DownloadMode -eq "bin") { "$Artifact.exe" } else { "$Artifact.tar.gz" }
    Verify-DownloadedChecksum -FilePath $DownloadPath -AssetName $assetName -ChecksumsUrl $ChecksumsUrl
}

$DestBin = Join-Path $VersionDir "iagent.exe"

if ($DownloadMode -eq "tar") {
    Write-Info "Extracting..."
    tar xzf $DownloadPath -C $TempDir 2>$null
    $SrcBin = Join-Path $TempDir "$Artifact.exe"
    if (-not (Test-Path $SrcBin)) {
        Write-Err "Downloaded archive did not contain expected binary: $Artifact.exe"
    }
    Move-Item -Path $SrcBin -Destination $DestBin -Force
} elseif ($DownloadMode -eq "bin") {
    Move-Item -Path $DownloadPath -Destination $DestBin -Force
} else {
    Write-Info "No prebuilt asset found for $Artifact in $Version; building from source..."
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Write-Err "git is required to build from source" }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Write-Err "cargo is required to build from source" }

    $SrcDir = Join-Path $TempDir "iAgent-windows-src"
    Write-Info "Cloning $Repo at $Version..."
    $gitCloneResult = Invoke-ProcessWithTimeout -FilePath "git" -ArgumentList @(
        "clone",
        "--depth", "1",
        "--branch", $Version,
        "https://github.com/$Repo.git",
        $SrcDir
    ) -TimeoutSeconds 600 -FriendlyName "git-clone" -CaptureOutput
    if ($gitCloneResult.TimedOut) {
        Write-LogTail -Path $gitCloneResult.StdoutPath -Label "git stdout"
        Write-LogTail -Path $gitCloneResult.StderrPath -Label "git stderr"
        Write-Err "git clone timed out after 600 seconds"
    }
    if ($gitCloneResult.ExitCode -ne 0) {
        Write-LogTail -Path $gitCloneResult.StdoutPath -Label "git stdout"
        Write-LogTail -Path $gitCloneResult.StderrPath -Label "git stderr"
        Write-Err "Failed to clone $Repo at $Version (exit code: $($gitCloneResult.ExitCode))"
    }

    Write-Info "Building iAgent from source (this can take several minutes)..."
    $cargoResult = Invoke-ProcessWithTimeout -FilePath "cargo" -ArgumentList @("build", "--release", "--manifest-path", (Join-Path $SrcDir "Cargo.toml")) -TimeoutSeconds 1800 -FriendlyName "cargo-build" -CaptureOutput
    if ($cargoResult.TimedOut) {
        Write-LogTail -Path $cargoResult.StdoutPath -Label "cargo stdout"
        Write-LogTail -Path $cargoResult.StderrPath -Label "cargo stderr"
        Write-Err "cargo build timed out after 1800 seconds"
    }
    if ($cargoResult.ExitCode -ne 0) {
        Write-LogTail -Path $cargoResult.StdoutPath -Label "cargo stdout"
        Write-LogTail -Path $cargoResult.StderrPath -Label "cargo stderr"
        Write-Err "cargo build failed (exit code: $($cargoResult.ExitCode))"
    }

    $BuiltBin = Join-Path $SrcDir "target\release\iagent.exe"
    if (-not (Test-Path $BuiltBin)) { Write-Err "Built binary not found at $BuiltBin" }
    Copy-Item -Path $BuiltBin -Destination $DestBin -Force
}

Copy-Item -Path $DestBin -Destination (Join-Path $StableDir "iagent.exe") -Force
Set-Content -Path (Join-Path $BuildsDir "stable-version") -Value $VersionNum
Copy-Item -Path (Join-Path $StableDir "iagent.exe") -Destination $LauncherPath -Force

Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$UserPath", "User")
    Write-Info "Added $InstallDir to user PATH"
}

$env:Path = "$InstallDir;$env:Path"

$installedAlacritty = $false
$configuredHotkey = $false
$configuredPersonalDaemon = $false
$configuredDesktopShortcut = $false
$dockLauncher = $null

if ($SkipDockSetup) {
    Write-Info "Skipping iAgent dock setup"
} else {
    $AppDir = Install-IagentDockApp -TargetAppDir $AppDir
}

$dockLauncher = New-IagentDockLauncher -IagentExePath $LauncherPath -TargetAppDir $AppDir -TargetInstallDir $InstallDir

if ($SkipAlacrittySetup) {
    Write-Info "Skipping Alacritty setup"
    $installedAlacritty = Test-AlacrittyInstalled
} else {
    Write-Info "Skipping Alacritty setup; the iAgent dock launches without a terminal"
    $installedAlacritty = Test-AlacrittyInstalled
}

if ($SkipHotkeySetup) {
    Write-Info "Skipping Alt+; hotkey setup"
} elseif ($dockLauncher) {
    $configuredHotkey = Install-IagentHotkey -DockLauncherVbsPath $dockLauncher.VbsPath
}

if ($SkipPersonalDaemonSetup) {
    Write-Info "Skipping personal ambient daemon setup"
} else {
    $configuredPersonalDaemon = Install-IagentPersonalDaemon -IagentExePath $LauncherPath
}

if ($SkipDesktopShortcut) {
    Write-Info "Skipping desktop shortcut setup"
} else {
    if (New-IagentRobotIcon -IconPath $IconPath) {
        $configuredDesktopShortcut = Install-IagentDesktopShortcut -DockLauncherVbsPath $dockLauncher.VbsPath -IconPath $IconPath -WorkingDirectory $AppDir
    } else {
        $configuredDesktopShortcut = Install-IagentDesktopShortcut -DockLauncherVbsPath $dockLauncher.VbsPath -IconPath $LauncherPath -WorkingDirectory $AppDir
    }
}

Set-SetupHintsState -AlacrittyConfigured:(Test-AlacrittyInstalled) -HotkeyConfigured:$configuredHotkey
Write-DockPostInstallSelfCheck -TargetInstallDir $InstallDir -TargetAppDir $AppDir -DockLauncher $dockLauncher -DockWasSkipped:$SkipDockSetup.IsPresent

Write-Host ""
Write-Info "iAgent $Version installed successfully!"
Write-Host ""

if (Test-AlacrittyInstalled) {
    $alacrittyPath = Find-AlacrittyPath
    if ($alacrittyPath) {
        Write-Info "Alacritty ready: $alacrittyPath"
    }
}

if ($configuredHotkey) {
    Write-Info "Global hotkey ready: Alt+; opens the iAgent dock"
    Write-Host ""
}

if ($configuredPersonalDaemon) {
    Write-Info "Personal ambient daemon ready: clipboard, reminders, jobs, and proactive suggestions run at login"
    Write-Host ""
}

if ($configuredDesktopShortcut) {
    Write-Info "Desktop shortcut ready: double-click iAgent to open the dock."
    Write-Host ""
}

if (Get-Command iagent -ErrorAction SilentlyContinue) {
    Write-Info "Backend CLI available: iagent"
} else {
    Write-Host "  Open a new terminal window, then run:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "    iagent" -ForegroundColor Green
}

if (Test-OfficeCLIInstalled) {
    Write-Info "OfficeCLI ready: Word/Excel/PowerPoint automation available"
    Write-Host "  Run: $InstallDir\officecli.exe --version" -ForegroundColor DarkGray
} else {
    Write-Warn "OfficeCLI not installed - Office automation will not be available"
    Write-Host "  To install: irm https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.ps1 | iex" -ForegroundColor DarkGray
}
