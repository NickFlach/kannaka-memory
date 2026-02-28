$repos = Get-ChildItem 'C:\Users\nickf\Source' -Directory | Where-Object { $_.Name -ne '0xSCADA' }
foreach ($r in $repos) {
    $files = Get-ChildItem $r.FullName -Recurse -File -ErrorAction SilentlyContinue | Where-Object {
        $_.Extension -match '\.(js|ts|py|sol|json|env|md|txt|yml|yaml|toml|cfg|ini|sh|bat|ps1|go|rs|tsx|jsx|html|css)$' -and
        $_.FullName -notmatch '[\\/](node_modules|\.git|vendor|dist|build|\.next)[\\/]'
    }
    foreach ($f in $files) {
        $matches = Select-String -Path $f.FullName -Pattern '0x[0-9a-fA-F]{40}' -ErrorAction SilentlyContinue
        foreach ($m in $matches) {
            $line = $m.Line.Trim()
            if ($line.Length -gt 120) { $line = $line.Substring(0,120) }
            Write-Output "$($r.Name)|$($f.FullName)|$($m.LineNumber)|$line"
        }
    }
}
Write-Host "SCAN_COMPLETE"
