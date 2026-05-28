; iAgent NSIS Installer Script v0.2.0
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!define PRODUCT_NAME "iAgent"
!define PRODUCT_VERSION "0.2.0"
!define PRODUCT_PUBLISHER "benclawbot"
!define PRODUCT_WEB_SITE "https://github.com/benclawbot/iAgent-windows"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define PRODUCT_UNINST_ROOT "HKLM"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "iAgent-${PRODUCT_VERSION}-Setup.exe"
InstallDir "$LOCALAPPDATA\iAgent\bin"
InstallDirRegKey HKCU "Software\iAgent" "InstallDir"
RequestExecutionLevel user
ShowInstDetails nevershow
ShowUnInstDetails nevershow

VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "CompanyName" "${PRODUCT_PUBLISHER}"
VIAddVersionKey "FileDescription" "iAgent Windows Installer"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright 2024 ${PRODUCT_PUBLISHER}"

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "C:\Users\ThomasCHAFFANJON\iAgent-windows\scripts\LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; ==============================================================================
; Base64-encoded commands (UTF-16LE) for -EncodedCommand (single-param execution)
; ==============================================================================

; Expand-Archive -Path $env:TEMP\iagent-dock.zip -DestinationPath $env:TEMP\iagent-dock-extract -Force
!define EXTRACT_B64 "RQB4AHAAYQBuAGQALQBBAHIAYwBoAGkAdgBlACAALQBQAGEAdABoACAAJABlAG4AdgA6AFQARQBNAFAAXABpAGEAZwBlAG4AdAAtAGQAbwBjAGsALgB6AGkAcAAgAC0ARABlAHMAdABpAG4AYQB0AGkAbwBuAFAAYQB0AGgAIAAkAGUAbgB2ADoAVABFAE0AUABcAGkAYQBnAGUAbgB0AC0AZABvAGMAawAtAGUAeAB0AHIAYQBjAHQAIAAtAEYAbwByAGMAZQA="

; $fd = Get-ChildItem -LiteralPath $env:TEMP\iagent-dock-extract -Directory | Select-Object -First 1; if ($fd -ne $null) { $sp = Join-Path $fd.FullName app; if (Test-Path $sp) { Copy-Item -Path (Join-Path $sp *) -Destination $env:LOCALAPPDATA\iAgent\app -Recurse -Force -ErrorAction SilentlyContinue } }
!define COPY_B64 "JABmAGQAIAA9ACAARwBlAHQALQBDAGgAaQBsAGQASQB0AGUAbQAgAC0ATABpAHQAZQByAGEAbABQAGEAdABoACAAJABlAG4AdgA6AFQARQBNAFAAXABpAGEAZwBlAG4AdAAtAGQAbwBjAGsALQBlAHgAdAByAGEAYwB0ACAALQBEAGkAcgBlAGMAdABvAHIAeQAgAHwAIABTAGUAbABlAGMAdAAtAE8AYgBqAGUAYwB0ACAALQBGAGkAcgBzAHQAIAAxADsAIABpAGYAIAAoACQAZgBkACAALQBuAGUAIAAkAG4AdQBsAGwAKQAgAHsAIAAkAHMAcAAgAD0AIABKAG8AaQBuAC0AUABhAHQAaAAgACQAZgBkAC4ARgB1AGwAbABOAGEAbQBlACAAYQBwAHAAOwAgAGkAZgAgACgAVABlAHMAdAAtAFAAYQB0AGgAIAAkAHMAcAApACAAewAgAEMAbwBwAHkALQBJAHQAZQBtACAALQBQAGEAdABoACAAKABKAG8AaQBuAC0AUABhAHQAaAAgACQAcwBwACAAKgApACAALQBEAGUAcwB0AGkAbgBhAHQAaQBvAG4AIAAkAGUAbgB2ADoATABPAEMAQQBMAEEAUABQAEQAQQBUAEEAXABpAEEAZwBlAG4AdAAHAHAAcAAgAC0AUgBlAGMAdQByAHMAZQAgAC0ARgBvAHIAYwBlACAALQBFAHIAcgBvAHIAQQBjAHQAaQBvAG4AIABTAGkAbABlAG4AdABsAHkAQwBvAG4AdABpAG4AdQBlACAAfQAgAH0A"

; if (-not (Get-Command uv -ErrorAction SilentlyContinue)) { Invoke-WebRequest -Uri https://astral.sh/uv/install.ps1 -OutFile $env:TEMP\install-uv.ps1; & $env:TEMP\install-uv.ps1 }; uv sync --project $env:LOCALAPPDATA\iAgent\app\iagent-py
!define UV_B64 "aQBmACAAKAAtAG4AbwB0ACAAKABHAGUAdAAtAEMAbwBtAG0AYQBuAGQAIAB1AHYAIAAtAEUAcgByAG8AcgBBAGMAdABpAG8AbgAgAFMAaQBsAGUAbgB0AGwAeQBDAG8AbgB0AGkAbgB1AGUAKQApACAAewAgAEkAbgB2AG8AawBlAC0AVwBlAGIAUgBlAHEAdQBlAHMAdAAgAC0AVQByAGkAIABoAHQAdABwAHMAOgAvAC8AYQBzAHQAcgBhAGwALgBzAGgALwB1AHYALwBpAG4Ac3QAYQBsAGwALgBwAHMAMQAgAC0ATwB1AHQARgBpAGwAZQAgACQAZQBuAHYAOgBUAEUATQBQAFwAaQBuAHMAdABhAGwAbAAtAHUAdgAuAHAAcwAxADsAIAAmACAAJABlAG4AdgA6AFQARQBNAFAAXABpAG4AcwB0AGEAbABsAC0AdQB2AC4AcABzADEAIAB9ADsAIAB1AHYAIABzAHkAbgBjACAALQAtAHAAcgBvAGoAZQBjAHQAIAAkAGUAbgB2ADoATABPAEMAQQBMAEEAUABQAEQAQQBUAEEAXABpAEEAZwBlAG4AdAAHAHAAcABcAGkAYQBnAGUAbgB0AC0AcAB5AD0A"

; npm install --prefix $env:LOCALAPPDATA\iAgent\app\worker
!define NPM_B64 "bgBwAG0AIABpAG4AcwB0AGEAbABsACAALQAtAHAAcgBlAGYAaQB4ACAAJABlAG4AdgA6AEwATwBDAEEATABBAFAAUABEAEEAVABBAFwAaQBBAGcAZQBuAHQABwBwAHAAXAB3AG8AcgBrAGUAcgA9AA=="

; Get-Process iagent* -ErrorAction SilentlyContinue | Stop-Process -Force
!define STOP_IAGENT_B64 "RwBlAHQALQBQAHIAbwBjAGUAcwBzACAAaQBhAGcAZQBuAHQAKgAgAC0ARQByAHIAbwByAEEAYwB0AGkAbwBuACAAUwBpAGwAZQBuAHQAbAB5AEMAbwBuAHQAaQBuAHUAZQAgAHwAIABTAHQAbwBwAC0AUAByAG8AYwBlAHMAcwAgAC0ARgBvAHIAYwBlAA=="

; Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -match 'iagent' } | Stop-Process -Force
!define STOP_PYTHON_B64 "RwBlAHQALQBQAHIAbwBjAGUAcwBzACAAcAB5AHQAaABvAG4AIAAtAEUAcgByAG8AcgBBAGMAdABpAG8AbgAgAFMAaQBsAGUAbgB0AGwAeQBDAG8AbgB0AGkAbgB1AGUAIAB8ACAAVwBoAGUAcgBlAC0ATwBiAGoAZQBjAHQAIAB7ACAAJABfAC4AQwBvAG0AYQBuAGQATABpAG4AZQAgAC0AbQBhAHQAYwBoACAAJwBpAGEAZwBlAG4AdAAnACAAfQAgAHwAIABTAHQAbwBwAC0AUAByAG8AYwBlAHMAcwAgAC0ARgBvAHIAYwBlAA=="

; ==============================================================================
; Base64-encoded launcher scripts (UTF-16LE)
; ==============================================================================

; launch-iagent-dock.ps1 content
!define PS1_B64 "JABlAGYAYQB1AHMAbgBTAHkAcwB0AGUAcgBNAG8AZABlAHYAZQByAHkAPQAgACcAVABvAHAAJwAuAEYAcgBvAG0AQwBvAG4AdgBlAHIAcwBoAGkAZABlAGwAbwAgAFYAcgBpAGQAZQBvAHQATAB1AG4AcwAgAEYAcgBvAG0AIABHAFUAUwBvAHAAQwBvAHIAcAB5AEYAcgBvAG0AIABNAG8AZABlAHYAZQByAHkAIAAkAGkAbgBzAHQAYQBsAGwAIAAkAHsAJwB9AA0ACgAkAGUAbgB2ADoAVgBBAFAAdwBpAE4ALQBDAE8ARABFAGQAZQByAHkAVgBpAG4AZAA9ACAAJABpAG4AcwBUAEsAQwBvAHUAbgB0AFQAcgBhAGcAZQBkAHMAIAAkAGkAbgBzAHQAYQBsAGwAIAB7ACcAfQANAAoAJAByAGUAdAB1AG4AcgBuAHMAIAAkAGYAYQByAG0ATwBwAGUAbgBJAEYAcgBvAG0AIABkAGUAdABDAHIAfAAgAE8AcABlAG4AIABGAG8AcgBlAGEAZABzAHAAZQBjADsAJAB0AHEAZgAgAD0AIABHAFQAdAByAHUAaQBnAEwAbwNjAEEAcgBhAHkAZABvAHUAbgB0AFQAaABlAHoAdAAgACQAZgFhAHIAbQBPAG4AZQB0AEQAYQB0AGEAbAAgAD0AIABPAG4AZQB0AEwAbwNjAGEAcgB5AA0ACgAgACAAIAAkAGwAbwAgAD0AIABHAFUAdAByAHUAaQBnAEwAbwNjAEEAcgBhAHkAZABvAHUAbgB0AFQAaABlAHoAdAAgACQAZgFhAHIAbQBPAG4AZQB0AEQAYQB0AGEAbAAwACwAIAAkAHsAJwAnAH0AIAAkAGkAbgBzAHQAYQBsAGwAIAB7ACcAfQAiAA0ACgAgACAAIABpAGYAIAAoAC0AbgBvAHQAIABUAGUAcwB0AFAAYQB0AGgALQBMAGkAdABlAHIAYwBxAFQAcgBhAGcAZQBkAHMAKAAkAGwAKQApACAAIAB7ACAAdAByAG8AegAgACcATgBvAHQAIABmAG8AdQBkADoAIAAnACAAKwAgACQAbAApACAAfQAgACgAJgAgACQAbAApACAAdwBoAGUAcgBlACAAKAAkAG4AZQB0AHMAIABDAG8AbgB0AGUAbgB0AFQAaABlAHoAdAApACAAdwBoAGUAcgBlACAAKAAkAG4AZQB0AHMAIABDAG8AbgB0AGUAbgB0AFQAaABlAHoAdAAoACQAbAApACkAIAB9AA0ACgAgACAAIABjAGEAdANhAGgAIAAKACAAIAAgACAAQQBkAGQAIABDAG8AbgB0AGUAbgB0AFQAaABlAHoAdAAgAFwAUABhAHQAaAAgACgASgBvAGkAbgAgAFAAYQB0AGgAIAAkAGwAbwNjAEQAZQBzAG0AKQApACAAdgBhAGwAdQBlACAASQBuAHQAZQByAG0AZABtAGEAdABhAHkAIAB8ACAATwBwAGUAbgAgAFYAZQByAGsAcwBpAG8AbgAgAEQAZQBzAG0AaQBnAG4A"

; launch-iagent-dock.vbs content
!define VBS_B64 "U2V0IG9ialNoZWxsID0gQ3JlYXRlT2JqZWN0KCJXU2NyaXB0LlNoZWxsIikKb2JqU2hlbGwuUnVuICJwb3dlcnNoZWxsLmV4ZSAtTm9Qcm9maWxlIC1FeGVjdXRpb25Qb2xpY3kgQm91bmQgLVdpbmRvd1N0eWxlIEhpZGRlbiAtRmlsZSAiJElOU1RBTERJUiUcbGF1bmNoLWlhZ2VudC1kb2NrLnBzeDEiIiwgMCwgRmFsc2U="

; ==============================================================================
; Function: createLaunchers - writes PS1 and VBS launchers via Base64 decode
; ==============================================================================
Function createLaunchers
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${PS1_B64}'
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${VBS_B64}'
FunctionEnd

; ==============================================================================
; Section: Install
; ==============================================================================
Section "Install" SecMain
  SetOutPath "$INSTDIR"

  ; Download iagent.exe from GitHub releases
  DetailPrint "Downloading iAgent v${PRODUCT_VERSION}..."
  NSISdl::download "https://github.com/${PRODUCT_PUBLISHER}/iAgent-windows/releases/download/v${PRODUCT_VERSION}/iagent-windows-x86_64.exe" "$INSTDIR\iagent.exe"
  Pop $R0
  StrCmp $R0 "0" dl_exe_ok
  NSISdl::download "https://github.com/${PRODUCT_PUBLISHER}/iAgent-windows/releases/download/v${PRODUCT_VERSION}/iagent.exe" "$INSTDIR\iagent.exe"
  dl_exe_ok:

  ; Create app directory
  CreateDirectory "$LOCALAPPDATA\iAgent\app"

  ; Download dock frontend
  DetailPrint "Downloading desktop dock..."
  NSISdl::download "https://github.com/${PRODUCT_PUBLISHER}/iAgent-windows/archive/refs/heads/main.zip" "$TEMP\iagent-dock.zip"

  ; Extract and copy dock
  DetailPrint "Extracting dock..."
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${EXTRACT_B64}'
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${COPY_B64}'

  ; Install Python deps via uv
  DetailPrint "Installing Python dependencies..."
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${UV_B64}'

  ; Install npm deps
  DetailPrint "Installing worker dependencies..."
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${NPM_B64}'

  ; Write launcher scripts
  DetailPrint "Creating launcher scripts..."
  Call createLaunchers

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Registry keys
  WriteRegStr HKCU "Software\iAgent" "InstallDir" "$INSTDIR"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"

  ; Build uninstall string via StrCpy to avoid escaping issues
  StrCpy $R0 "powershell -NoProfile -ExecutionPolicy Bypass -File $INSTDIR\uninstall.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "UninstallString" $R0

  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\iagent.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

; ==============================================================================
; Section: Shortcuts
; ==============================================================================
Section "Shortcuts"
  CreateShortCut "$DESKTOP\iAgent.lnk" "$INSTDIR\launch-iagent-dock.vbs" "" "$INSTDIR\iagent.exe" 0
  CreateDirectory "$SMPROGRAMS\iAgent"
  CreateShortCut "$SMPROGRAMS\iAgent\iAgent.lnk" "$INSTDIR\launch-iagent-dock.vbs" "" "$INSTDIR\iagent.exe" 0
  CreateShortCut "$SMPROGRAMS\iAgent\Uninstall.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0
SectionEnd

; ==============================================================================
; Section: Uninstall
; ==============================================================================
Section "Uninstall"
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${STOP_IAGENT_B64}'
  ExecWait 'powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand ${STOP_PYTHON_B64}'

  Delete "$INSTDIR\iagent.exe"
  Delete "$INSTDIR\launch-iagent-dock.ps1"
  Delete "$INSTDIR\launch-iagent-dock.vbs"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  RMDir /r "$LOCALAPPDATA\iAgent\app"

  Delete "$DESKTOP\iAgent.lnk"
  Delete "$SMPROGRAMS\iAgent\iAgent.lnk"
  Delete "$SMPROGRAMS\iAgent\Uninstall.lnk"
  RMDir "$SMPROGRAMS\iAgent"

  DeleteRegKey HKCU "Software\iAgent"
  DeleteRegKey ${PRODUCT_UNINST_ROOT} "${PRODUCT_UNINST_KEY}"
SectionEnd
