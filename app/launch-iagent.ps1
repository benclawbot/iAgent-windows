$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSCommandPath
$workerDir = Join-Path $repoRoot "worker"
$iagentDir = Join-Path $repoRoot "iagent-py"
$npmCmd = (Get-Command npm -ErrorAction SilentlyContinue).Source
$uvCmd = (Get-Command uv -ErrorAction SilentlyContinue).Source
$workerConfig = "wrangler.local.toml"
if (-not (Test-Path -LiteralPath (Join-Path $workerDir $workerConfig))) {
  $workerConfig = "wrangler.toml"
}
$runtimeConfig = Join-Path $env:APPDATA "iAgent\config.toml"

$workerUrl = ""
if (Test-Path -LiteralPath $runtimeConfig) {
  $line = Get-Content -LiteralPath $runtimeConfig | Where-Object { $_ -match "^\s*worker_url\s*=" } | Select-Object -First 1
  if ($line -match '^\s*worker_url\s*=\s*"(.*)"\s*$') {
    $workerUrl = $Matches[1].Trim()
  }
}

try {
  $iagentProcs = Get-CimInstance Win32_Process | Where-Object {
    ($_.Name -eq "uv.exe" -and $_.CommandLine -match "run\s+python(\.exe)?\s+-m\s+iagent") -or
    (
      ($_.Name -eq "python.exe" -or $_.Name -eq "pythonw.exe") -and (
        $_.CommandLine -match "uv(\.exe)?\s+run\s+python(\.exe)?\s+-m\s+iagent" -or
        $_.CommandLine -match "python(\.exe)?\s+-m\s+iagent"
      )
    )
  }
} catch {
  # If process introspection is blocked, default to launching.
  $iagentProcs = @()
}
if (-not $iagentProcs) {
  if ([string]::IsNullOrWhiteSpace($uvCmd)) {
    throw "uv command not found on PATH. Install uv to run iAgent."
  }
  Start-Process -FilePath $uvCmd -ArgumentList @("run","python","-m","iagent") -WorkingDirectory $iagentDir -WindowStyle Hidden | Out-Null
}

# Start worker in parallel after iAgent launch, without blocking dock startup.
if (-not [string]::IsNullOrWhiteSpace($workerUrl)) {
  if ([string]::IsNullOrWhiteSpace($npmCmd)) {
    throw "npm command not found on PATH. Install Node.js to run worker mode."
  }
  # Start worker only if not already running for this repo with the local config.
  $workerDirEscaped = ($workerDir -replace "\\", "\\")
  $wranglerTmpDir = Join-Path $workerDir ".wrangler\\tmp"
  try {
    $workerProcs = Get-CimInstance Win32_Process | Where-Object {
      $_.Name -eq "node.exe" -and
      $_.CommandLine -like "*wrangler dev*" -and
      $_.CommandLine -like "*$workerDirEscaped*"
    }
  } catch {
    # If process introspection is blocked by policy, proceed with a fresh launch.
    $workerProcs = @()
  }

  if ($workerProcs) {
    $wrongConfig = $workerProcs | Where-Object { $_.CommandLine -notlike "*$workerConfig*" }
    if ($wrongConfig) {
      $wrongConfig | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
      Start-Sleep -Milliseconds 200
      $workerProcs = @()
    }
  }

  if (-not $workerProcs) {
    $outLog = Join-Path $workerDir ".worker-dev.out.log"
    $errLog = Join-Path $workerDir ".worker-dev.err.log"
    if (Test-Path -LiteralPath $wranglerTmpDir) {
      Remove-Item -LiteralPath $wranglerTmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    # Force npm to resolve wrangler from this repo's dependency graph.
    Start-Process -FilePath $npmCmd -ArgumentList @("exec","--","wrangler","dev","--config",$workerConfig,"--port","8787") -WorkingDirectory $workerDir -WindowStyle Hidden -RedirectStandardOutput $outLog -RedirectStandardError $errLog | Out-Null
  }
}
