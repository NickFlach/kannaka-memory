param([Parameter(Mandatory=$true)][string]$name)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

if (-not $name) {
    Write-Host "Usage: .\get-entity.ps1 'Nick'"
    exit 1
}

python "$scriptDir\get-entity.py" $name