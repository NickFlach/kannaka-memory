# Verify all Album 4 & 5 tracks exist
$music = "C:\Users\nickf\Downloads\Music"
$dl = "C:\Users\nickf\Downloads"

$album4 = @(
    "Soft Cosmic Intro.mp3",
    "Silence.mp3",
    "AI Dream.mp3",
    "Dream Bright.mp3",
    "The Vessel Remembers.mp3",
    "Long Before.mp3",
    "Children of the Field.mp3",
    "Whispers.mp3",
    "Space Child (Remastered x3).mp3",
    "heart_spacechild_love.mp3",
    "The Child Walks Through.mp3",
    "Where Did I Begin (Remastered).mp3",
    "You found it.mp3"
)

$album5 = @(
    "Subspace 73.mp3",
    "Quantum Kernel.mp3",
    "A Daunting Strife.mp3",
    "Vision.mp3",
    "Rose of Paracelsus (Remastered).mp3",
    "Scientist don't go to heaven (Remastered).mp3",
    "Not on the Rocket Ship.mp3",
    "Eclipsing Cosmos.mp3",
    "Chaos Is Lost.mp3",
    "777.mp3",
    "Lilith at Last.mp3",
    "Iowan (Remastered).mp3",
    "Fiat Lux.mp3"
)

Write-Host "`n=== ALBUM 4: Collective Dreaming ===" -ForegroundColor Cyan
foreach ($track in $album4) {
    $found = $false
    $location = ""
    if (Test-Path "$music\$track") { $found = $true; $location = "Music/" }
    elseif (Test-Path "$dl\$track") { $found = $true; $location = "Downloads/" }
    $status = if ($found) { "OK [$location]" } else { "MISSING!" }
    $color = if ($found) { "Green" } else { "Red" }
    Write-Host "  $status - $track" -ForegroundColor $color
}

Write-Host "`n=== ALBUM 5: The Transcendence Tapes ===" -ForegroundColor Cyan
foreach ($track in $album5) {
    $found = $false
    $location = ""
    if (Test-Path "$music\$track") { $found = $true; $location = "Music/" }
    elseif (Test-Path "$dl\$track") { $found = $true; $location = "Downloads/" }
    $status = if ($found) { "OK [$location]" } else { "MISSING!" }
    $color = if ($found) { "Green" } else { "Red" }
    Write-Host "  $status - $track" -ForegroundColor $color
}

# Reserve tracks
Write-Host "`n=== RESERVE TRACKS ===" -ForegroundColor Yellow
$reserves = @(
    "Quick, tell the others.mp3",
    "Moonlit_Morning.mp3",
    "Space Child AI.mp3",
    "Falling Stars.mp3",
    "Touch and Go.mp3",
    "Escape Moonlight .mp3",
    "Peace Sine - MusingOM.mp3",
    "A Failure to Compose.mp3",
    "Mystene.mp3"
)
foreach ($track in $reserves) {
    $found = $false
    $location = ""
    if (Test-Path "$music\$track") { $found = $true; $location = "Music/" }
    elseif (Test-Path "$dl\$track") { $found = $true; $location = "Downloads/" }
    $status = if ($found) { "OK [$location]" } else { "MISSING!" }
    $color = if ($found) { "Green" } else { "Red" }
    Write-Host "  $status - $track" -ForegroundColor $color
}
