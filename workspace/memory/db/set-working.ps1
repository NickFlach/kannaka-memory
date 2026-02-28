param(
    [Parameter(Mandatory=$true)][string]$key,
    [Parameter(Mandatory=$true)][string]$value,
    [string]$category = 'general',
    [int]$priority = 0,
    [string]$expiresAt = $null
)

$dbPath = "C:\Users\nickf\.openclaw\workspace\memory\kannaka.db"

if (-not $key -or -not $value) {
    Write-Host "Usage: .\set-working.ps1 -key 'active_task' -value 'Building memory system' -category 'task' -priority 10"
    exit 1
}

# Escape single quotes for SQL
$key = $key -replace "'", "''"
$value = $value -replace "'", "''"
$category = $category -replace "'", "''"
$expiresAt = if ($expiresAt) { $expiresAt -replace "'", "''" } else { $null }

$pythonScript = @"
import sqlite3
import sys

conn = sqlite3.connect(r'$dbPath')
cursor = conn.cursor()

try:
    # UPSERT (INSERT or UPDATE if exists)
    cursor.execute('''
        INSERT INTO working_memory (key, value, category, priority, expires_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            category = excluded.category,
            priority = excluded.priority,
            expires_at = excluded.expires_at,
            updated_at = datetime('now')
    ''', ('$key', '$value', '$category', $priority, '$expiresAt'))
    
    conn.commit()
    if cursor.rowcount == 1:
        print(f'Working memory set: $key = $value')
    else:
        print(f'Working memory updated: $key = $value')
    
except Exception as e:
    print(f'Error setting working memory: {e}')
    sys.exit(1)
finally:
    conn.close()
"@

python -c $pythonScript