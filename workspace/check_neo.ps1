cd C:\Users\nickf\Source\flaukowski
$commits = git log --all --oneline
foreach ($line in $commits) {
    $hash = $line.Split(' ')[0]
    $content = git show "${hash}:README.md" 2>$null
    if ($content -match "NRTAqoqs") {
        Write-Host "FOUND in ${hash}"
    } else {
        Write-Host "CLEAN: ${hash}"
    }
}
