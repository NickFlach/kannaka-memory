<#
.SYNOPSIS
  Cleanly remove the Kannaka seed-beacon Scheduled Task installed by
  install-beacon-task.ps1 (task + running daemon + copied launcher).

.DESCRIPTION
  `Unregister-ScheduledTask` alone is NOT a complete uninstall: the launcher
  starts the `swarm beacon --loop` daemon, and the copied kannaka-beacon-hidden.vbs
  is left behind. This script does all three:

    1. Stop + unregister the task.
    2. Kill the beacon daemon by its OWN command line.
    3. Remove the copied launcher.

  WHY (2) matches the daemon and not the launcher process tree: wscript launches
  kannaka via WshShell.Run, which is NOT a job-object child of the task. When
  Task Scheduler stops the task it terminates the wscript host, but the kannaka
  daemon is reparented and survives (orphaned). Keying the kill off the wscript
  parent therefore misses the very process we need to stop, so we match the
  daemon's own command line (`kannaka.exe ... swarm beacon --loop`) instead. We
  filter Name='kannaka.exe' so an unrelated shell whose command line merely
  contains the pattern never self-matches, and scope to THIS task's exe path
  (read from the task action before removal) so a beacon launched from a
  different kannaka.exe is not collateral-killed.

  Safe to run when the task is already gone (idempotent). If the task is already
  removed, the exe path is unknown, so every `swarm beacon --loop` daemon is
  stopped (a full teardown of the beacon on this host).

.PARAMETER TaskName
  Scheduled Task name to remove. Default: KannakaSeedBeacon

.PARAMETER InstallDir
  Directory the launcher was copied to (matches the installer default).
  Default: %USERPROFILE%\.local\bin

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File ops\windows\uninstall-beacon-task.ps1
#>
[CmdletBinding()]
param(
    [string]$TaskName   = 'KannakaSeedBeacon',
    [string]$InstallDir = (Join-Path $env:USERPROFILE '.local\bin')
)

$ErrorActionPreference = 'Stop'
$vbs = Join-Path $InstallDir 'kannaka-beacon-hidden.vbs'

# 1. Capture the exe path from the task action (2nd quoted token of
#    'wscript.exe "<vbs>" "<exe>"') BEFORE removal, to scope the kill precisely.
#    Then stop + unregister.
$exePath = $null
$task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($task) {
    $arg = ($task.Actions | Select-Object -First 1).Arguments
    $quoted = [regex]::Matches([string]$arg, '"([^"]*)"')
    if ($quoted.Count -ge 2) { $exePath = $quoted[1].Groups[1].Value }
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    Write-Host "removed task '$TaskName'"
} else {
    Write-Host "no task '$TaskName' (already removed)"
}

# 2. Kill the beacon daemon by its own command line (survives orphaning). Scope to
#    the task's exe path when known; otherwise stop every beacon daemon.
$killed = 0
Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $_.Name -eq 'kannaka.exe' -and $_.CommandLine -match 'swarm beacon --loop' -and
    ($null -eq $exePath -or ($_.CommandLine -and $_.CommandLine.Contains($exePath)))
} | ForEach-Object {
    try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; $killed++ } catch {}
}
Write-Host "stopped $killed beacon daemon(s)"

# 3. Remove the copied launcher.
if (Test-Path $vbs) {
    Remove-Item $vbs -Force
    Write-Host "removed launcher $vbs"
} else {
    Write-Host "no launcher at $vbs"
}

Write-Host "uninstall complete."
