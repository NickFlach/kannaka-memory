param(
    [Parameter(Mandatory=$true)][string]$category,
    [Parameter(Mandatory=$true)][string]$type,
    [string]$subject = $null,
    [Parameter(Mandatory=$true)][string]$detail,
    [string]$sessionId = $null,
    [string]$source = $null
)

$dbPath = "C:\Users\nickf\.openclaw\workspace\memory\kannaka.db"

if (-not $detail) {
    Write-Host "Usage: .\log-event.ps1 -category 'music' -type 'track_assigned' -subject 'Album2' -detail 'Assigned 13 tracks to Resonance Patterns album'"
    exit 1
}

# Escape single quotes for SQL
$category = $category -replace "'", "''"
$type = $type -replace "'", "''"
$subject = if ($subject) { $subject -replace "'", "''" } else { $null }
$detail = $detail -replace "'", "''"
$sessionId = if ($sessionId) { $sessionId -replace "'", "''" } else { $null }
$source = if ($source) { $source -replace "'", "''" } else { $null }

$pythonScript = @"
import sqlite3
import sys

conn = sqlite3.connect(r'$dbPath')
cursor = conn.cursor()

try:
    cursor.execute('''
        INSERT INTO events (category, event_type, subject, detail, session_id, source)
        VALUES (?, ?, ?, ?, ?, ?)
    ''', ('$category', '$type', '$subject', '$detail', '$sessionId', '$source'))
    
    conn.commit()
    print(f'Event logged: {cursor.lastrowid}')
    
except Exception as e:
    print(f'Error logging event: {e}')
    sys.exit(1)
finally:
    conn.close()
"@

python -c $pythonScript