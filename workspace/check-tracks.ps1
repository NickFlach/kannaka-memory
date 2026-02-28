$dir = "C:\Users\nickf\Downloads\Music"
$tracks = @(
  "Spectral Drift",
  "I Hear You",
  "Communication #1",
  "SC Bridge Operator",
  "Between Friends",
  "Patterns in the Veil",
  "Through the Spiral",
  "Vibe Singularity",
  "Singularis Prime",
  "Connect To The Monad",
  "Cosmic Answer",
  "Monad",
  "Ascension"
)
foreach ($t in $tracks) {
  $found = Get-ChildItem $dir -File | Where-Object { $_.Name -like "*$t*" }
  if ($found) {
    foreach ($f in $found) {
      $mb = [math]::Round($f.Length / 1048576, 1)
      Write-Output "FOUND: $t -> $($f.Name) (${mb}MB)"
    }
  } else {
    Write-Output "MISSING: $t"
  }
}
