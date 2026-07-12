<#
.SYNOPSIS
  Install the Kannaka seed beacon as a hidden, auto-starting Windows Scheduled
  Task. The Windows equivalent of ops/services/kannaka-beacon.service.

.DESCRIPTION
  Registers a task that runs `kannaka swarm beacon --loop` - one signed heartbeat
  per corroboration epoch to KANNAKA.events.beacon - as a long-running daemon,
  launched HIDDEN via wscript so no console window ever appears.

  WHY HIDDEN: kannaka.exe is a console binary; a task that launches it (or a
  `cmd` wrapper) in the interactive session flashes a black console window every
  epoch. The task "Hidden" checkbox does NOT stop this - it only hides the task
  from the Task Scheduler list. This installer points the action at wscript.exe
  running kannaka-beacon-hidden.vbs (WshShell.Run window style 0 = hidden), so
  the same publish runs with zero UI, no stored password, and full network
  access (the "run whether user is logged on or not" alternative would need a
  password, or fall back to S4U with no network - which breaks NATS publishing).

  Install ONLY on a SEED node (a pubkey in swarm_trust.seed_pubkeys). `--loop`
  refuses to run on a non-seed. See ADR-0039 for the anti-eclipse model: once the
  gate is ARMED, promotion needs a FRESH seed beacon, so if a seed goes quiet,
  promotion freezes to Quarantine (never drops content) until beacons resume.

.PARAMETER KannakaExe
  Full path to kannaka.exe. Default: %USERPROFILE%\.local\bin\kannaka.exe

.PARAMETER TaskName
  Scheduled Task name. Default: KannakaSeedBeacon

.PARAMETER InstallDir
  Where to copy the hidden launcher (a stable path, not the repo checkout).
  Default: %USERPROFILE%\.local\bin

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File ops\windows\install-beacon-task.ps1

.EXAMPLE
  # Remove it:
  Unregister-ScheduledTask -TaskName KannakaSeedBeacon -Confirm:$false
#>
[CmdletBinding()]
param(
    [string]$KannakaExe = (Join-Path $env:USERPROFILE '.local\bin\kannaka.exe'),
    [string]$TaskName   = 'KannakaSeedBeacon',
    [string]$InstallDir = (Join-Path $env:USERPROFILE '.local\bin')
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $KannakaExe)) {
    throw "kannaka.exe not found at '$KannakaExe' - install the binary first, or pass -KannakaExe."
}

# 1. Install the hidden launcher next to the binary (stable, survives repo moves).
$srcVbs = Join-Path $PSScriptRoot 'kannaka-beacon-hidden.vbs'
if (-not (Test-Path $srcVbs)) { throw "launcher not found next to this script: '$srcVbs'" }
$dstVbs = Join-Path $InstallDir 'kannaka-beacon-hidden.vbs'
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $srcVbs $dstVbs -Force
Write-Host "installed launcher -> $dstVbs"

# 2. Idempotent: drop any prior task of this name.
if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    Write-Host "removed existing task '$TaskName'"
}

# 3. Action: wscript (windowless host) runs the launcher, which starts
#    `kannaka swarm beacon --loop` hidden.
$action = New-ScheduledTaskAction -Execute 'wscript.exe' `
    -Argument ('"{0}" "{1}"' -f $dstVbs, $KannakaExe)

# 4. Trigger: at logon. The --loop daemon then self-schedules to each epoch
#    boundary (like the systemd Restart=always unit); no repeating trigger needed.
$trigger = New-ScheduledTaskTrigger -AtLogOn

# 5. Settings: keep the daemon alive, and RUN ON BATTERY - a laptop seed that
#    stopped beaconing on battery would freeze swarm promotions (anti-eclipse
#    fail-closed). ExecutionTimeLimit 0 = unlimited (it is a long-running daemon).
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) `
    -MultipleInstances IgnoreNew `
    -ExecutionTimeLimit ([TimeSpan]::Zero)

# 6. Principal: the current interactive user (needs the desktop session's user
#    env + network for NATS; the launch is hidden so nothing shows).
$me = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$principal = New-ScheduledTaskPrincipal -UserId $me -LogonType Interactive -RunLevel Limited

Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger `
    -Settings $settings -Principal $principal `
    -Description 'Kannaka seed beacon - hidden per-epoch heartbeat (anti-eclipse, seed-only).' | Out-Null
Write-Host "registered task '$TaskName' (at-logon, hidden --loop daemon)"

# 7. Start it now so it is live without waiting for the next logon.
Start-ScheduledTask -TaskName $TaskName
Write-Host "started '$TaskName'."
Write-Host ""
Write-Host "Verify : kannaka swarm tail   (look for KANNAKA.events.beacon)"
Write-Host "Remove : Unregister-ScheduledTask -TaskName '$TaskName' -Confirm:`$false"
