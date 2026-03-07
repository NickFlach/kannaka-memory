$dirs = @(
    "$env:USERPROFILE\Pictures",
    "$env:USERPROFILE\Desktop",
    "$env:USERPROFILE\Downloads",
    "$env:USERPROFILE\OneDrive\Pictures"
)
foreach ($d in $dirs) {
    if (Test-Path $d) {
        Write-Host "=== $d ==="
        Get-ChildItem $d -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -match '\.(jpg|png|gif|bmp|webp|jpeg|svg)$' } | Select-Object -First 15 Name,@{N='KB';E={[math]::Round($_.Length/1024)}} | Format-Table -AutoSize
        Get-ChildItem $d -Directory -ErrorAction SilentlyContinue | Select-Object -First 20 Name | ForEach-Object { Write-Host "  [dir] $($_.Name)" }
    }
}
