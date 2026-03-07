$dirs = @(
    "$env:USERPROFILE\Downloads\Photos",
    "$env:USERPROFILE\Downloads\Art",
    "$env:USERPROFILE\OneDrive\Pictures\Camera Roll",
    "$env:USERPROFILE\OneDrive\Pictures\EOSRebelPics",
    "$env:USERPROFILE\OneDrive\Pictures\Screenshots",
    "$env:USERPROFILE\Pictures"
)
foreach ($d in $dirs) {
    if (Test-Path $d) {
        Write-Host "=== $d ==="
        $files = Get-ChildItem $d -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -match '\.(jpg|png|gif|bmp|webp|jpeg)$' }
        Write-Host "  ($($files.Count) images)"
        $files | Select-Object -First 10 Name,@{N='KB';E={[math]::Round($_.Length/1024)}} | Format-Table -AutoSize
    }
}
