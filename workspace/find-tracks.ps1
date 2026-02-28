$missing = @('I Hear You','SC Bridge Operator','Singularis Prime','Connect To The Monad','Cosmic Answer','Monad')
$searchPaths = @('C:\Users\nickf\Downloads','C:\Users\nickf\Music','C:\Users\nickf\Documents','C:\Users\nickf\Desktop','D:\')
foreach ($t in $missing) {
  Write-Output "--- Searching: $t ---"
  $anyFound = $false
  foreach ($sp in $searchPaths) {
    if (Test-Path $sp) {
      $found = Get-ChildItem $sp -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*$t*" }
      foreach ($f in $found) {
        $mb = [math]::Round($f.Length / 1048576, 1)
        Write-Output "  FOUND: $($f.FullName) ($mb MB)"
        $anyFound = $true
      }
    }
  }
  if (-not $anyFound) { Write-Output "  NOT FOUND ANYWHERE" }
}
