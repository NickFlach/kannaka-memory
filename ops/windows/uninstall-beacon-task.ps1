<#
.SYNOPSIS
  Cleanly remove the Kannaka seed-beacon Scheduled Task installed by
  install-beacon-task.ps1 (task + running daemon + copied launcher).

.DESCRIPTION
  `Unregister-ScheduledTask` alone is NOT a complete uninstall: the launcher
  starts the `swarm beacon --loop` daemon, and depending on how it was launched
  that process can outlive the task, and the copied kannaka-beacon-hidden.vbs is
  left behind. This script does all three, precisely:

    1. Stop + unregister the task.
    2. Kill exactly THIS launcher's process tree (the wscript running this
       task's own .vbs, plus its beacon child) - matched by the launcher path so
       a second, unrelated beacon on the machine is never touched.
    3. Remove the copied launcher.

  Safe to run when the task is already gone (idempotent).

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

# 1. Stop + unregister the task. Stopping first asks Task Scheduler to end the
#    task's process tree; the precise kill below is a fallback for any survivor.
if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    Write-Host "removed task '$TaskName'"
} else {
    Write-Host "no task '$TaskName' (already removed)"
}

# 2. Kill exactly this launcher's tree: the wscript.exe running THIS .vbs and its
#    children. Matching on the launcher path (not a blanket 'swarm beacon --loop'
#    sweep) means another beacon on the same host is never collateral-killed.
$all    = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
$hosts  = @($all | Where-Object { $_.Name -eq 'wscript.exe' -and $_.CommandLine -and $_.CommandLine.Contains($vbs) })
$killed = 0
foreach ($h in $hosts) {
    foreach ($kid in @($all | Where-Object { $_.ParentProcessId -eq $h.ProcessId })) {
        try { Stop-Process -Id $kid.ProcessId -Force -ErrorAction Stop; $killed++ } catch {}
    }
    try { Stop-Process -Id $h.ProcessId -Force -ErrorAction Stop; $killed++ } catch {}
}
Write-Host "stopped $killed leftover launcher process(es)"

# 3. Remove the copied launcher.
if (Test-Path $vbs) {
    Remove-Item $vbs -Force
    Write-Host "removed launcher $vbs"
} else {
    Write-Host "no launcher at $vbs"
}

Write-Host "uninstall complete."
