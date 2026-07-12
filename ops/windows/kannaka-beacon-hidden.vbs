' kannaka-beacon-hidden.vbs - windowless launcher for the seed beacon (Windows).
'
' Windows analog of the ExecStart in ops/services/kannaka-beacon.service. It runs
' `kannaka swarm beacon --loop` (one signed heartbeat per corroboration epoch to
' KANNAKA.events.beacon) with NO visible console.
'
' WHY THIS EXISTS: kannaka.exe is a console-subsystem binary. When a Windows
' Scheduled Task launches it (or a `cmd` wrapper) directly in the interactive
' session, Windows shows a console window every time it fires - a black flash
' once per epoch. The task's "Hidden" setting does NOT suppress this (it only
' hides the task from the Task Scheduler UI). wscript.exe is a windowless Script
' Host, and WshShell.Run with window style 0 starts the console binary hidden, so
' the identical beacon publish runs with zero UI.
'
' Usage (as a Scheduled Task action - see install-beacon-task.ps1):
'   wscript.exe "<dir>\kannaka-beacon-hidden.vbs"
'   wscript.exe "<dir>\kannaka-beacon-hidden.vbs" "C:\alt\path\kannaka.exe"
'
' Arg 1 (optional): full path to kannaka.exe.
'   Default: %USERPROFILE%\.local\bin\kannaka.exe (the install.sh location).

Option Explicit

Dim sh, exePath, cmd
Set sh = CreateObject("WScript.Shell")

If WScript.Arguments.Count >= 1 Then
    exePath = WScript.Arguments(0)
Else
    exePath = sh.ExpandEnvironmentStrings("%USERPROFILE%") & "\.local\bin\kannaka.exe"
End If

' Publisher only - the beacon emitter never writes the HRM. Keep it read-only so
' it can never contend for the single-writer lock (mirrors the .service unit's
' KANNAKA_READONLY=1). NATS auth is taken from the user's kannaka config/env.
sh.Environment("PROCESS")("KANNAKA_READONLY") = "1"

' One beacon per epoch; refuses to loop on a non-seed. For a fixed cadence,
' append: --interval-secs 60
cmd = """" & exePath & """ swarm beacon --loop"

' style 0 = hidden window, False = fire-and-forget (do not block the host).
sh.Run cmd, 0, False
