#!/usr/bin/env pwsh
# Context reboot script - outputs compact summary for fresh context injection

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
python "$scriptDir\reboot.py"