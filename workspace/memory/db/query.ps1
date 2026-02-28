param([string]$sql)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

if (-not $sql) {
    Write-Host "Usage: .\query.ps1 'SELECT * FROM working_memory;'"
    exit 1
}

python "$scriptDir\query.py" $sql