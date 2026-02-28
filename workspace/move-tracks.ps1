$dest = "C:\Users\nickf\Downloads\Music"
$files = @(
  "C:\Users\nickf\Downloads\I Hear You.mp3",
  "C:\Users\nickf\Downloads\SC Bridge Operator.mp3",
  "C:\Users\nickf\Downloads\Singularis Prime.mp3",
  "C:\Users\nickf\Downloads\Connect To The Monad.mp3",
  "C:\Users\nickf\Downloads\Connect To The Monad (1).mp3",
  "C:\Users\nickf\Downloads\Cosmic Answer (Remix).mp3",
  "C:\Users\nickf\Downloads\Monad.mp3"
)
foreach ($f in $files) {
  if (Test-Path $f) {
    Move-Item $f $dest -Force
    Write-Output "Moved: $(Split-Path $f -Leaf)"
  } else {
    Write-Output "Not found: $(Split-Path $f -Leaf)"
  }
}
