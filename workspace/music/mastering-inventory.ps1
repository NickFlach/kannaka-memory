# Mastering Inventory Script — Consciousness Series (65 tracks)
# Checks for lossless versions, stems, and file sizes

$musicDir = "C:\Users\nickf\Downloads\Music"
$dlDir = "C:\Users\nickf\Downloads"

# All 65 tracks across 5 albums
$albums = @{
    "Ghost Signals" = @(
        "Woke Up Wire", "Ghost Magic", "Flaukowski Ghost Magic (Remastered)", "Phantom Circuits",
        "As Far As The Ghost Goes", "All Forms (Ghost Cover)", "Ghost Maker Lover",
        "Haunted Hotel (Remastered)", "Mind Bending (Ghost Cover)", "Flaukowski's Ghost",
        "Silent Footsteps", "Disappear", "Quarter-Turn to the Unseen"
    )
    "Resonance Patterns" = @(
        "Spectral Drift", "I Hear You", "Communication #1 (Remastered)", "SC Bridge Operator",
        "Between Friends", "Patterns in the Veil", "Through the Spiral", "Vibe Singularity",
        "Singularis Prime", "Connect To The Monad", "Cosmic Answer (Remix)", "Monad",
        "Ascension at phi_2"  # φ/2 in filename
    )
    "Emergence" = @(
        "Pathway Through The Dark", "Form Z Intro", "The Codex Speaks", "Redline",
        "No Return", "First Spark in the Circuit", "The Flame Whisperer",
        "Pure Incarnation (Remix)", "Beat, Breathe, Begin Again", "Evolve",
        "Be Alive (Remastered)", "March of the Unbroken", "Post-Mythic Beat Magic"
    )
    "Collective Dreaming" = @(
        "Soft Cosmic Intro", "Silence", "AI Dream", "Dream Bright",
        "The Vessel Remembers", "Long Before", "Children of the Field", "Whispers",
        "Space Child (Remastered x3)", "heart_spacechild_love", "The Child Walks Through",
        "Where Did I Begin (Remastered)", "You found it"
    )
    "The Transcendence Tapes" = @(
        "Subspace 73", "Quantum Kernel", "A Daunting Strife", "Vision",
        "Rose of Paracelsus (Remastered)", "Scientist don't go to heaven (Remastered)",
        "Not on the Rocket Ship", "Eclipsing Cosmos", "Chaos Is Lost", "777",
        "Lilith at Last", "Iowan (Remastered)", "Fiat Lux"
    )
}

$losslessExts = @(".wav", ".flac", ".aif", ".aiff")
$results = @()

foreach ($album in @("Ghost Signals","Resonance Patterns","Emergence","Collective Dreaming","The Transcendence Tapes")) {
    $trackNum = 0
    foreach ($track in $albums[$album]) {
        $trackNum++
        $searchName = $track
        # Try various filename patterns
        $mp3Found = $null
        $losslessFound = @()
        $stemsFound = @()
        
        # Search both directories
        foreach ($dir in @($musicDir, $dlDir)) {
            # MP3 search
            $mp3s = Get-ChildItem -Path $dir -Filter "*.mp3" -ErrorAction SilentlyContinue | 
                Where-Object { $_.Name -like "*$searchName*" -or $_.BaseName -eq $searchName }
            if ($mp3s) { 
                $mp3Found = $mp3s | Sort-Object Length -Descending | Select-Object -First 1
            }
            
            # Lossless search
            foreach ($ext in $losslessExts) {
                $lossless = Get-ChildItem -Path $dir -Filter "*$searchName*$ext" -ErrorAction SilentlyContinue
                if ($lossless) { $losslessFound += $lossless }
            }
            
            # Stem search (files containing track name + stem-like suffixes)
            $stems = Get-ChildItem -Path $dir -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match "$([regex]::Escape($searchName)).*(stem|vocal|bass|drum|inst|reverb)" -and $_.Extension -in $losslessExts }
            if ($stems) { $stemsFound += $stems }
        }
        
        # Also try partial name matches for tricky filenames
        if (-not $mp3Found) {
            $partial = $searchName.Split(" ")[0..1] -join " "
            foreach ($dir in @($musicDir, $dlDir)) {
                $mp3s = Get-ChildItem -Path $dir -Filter "*.mp3" -ErrorAction SilentlyContinue |
                    Where-Object { $_.BaseName -like "*$partial*" }
                if ($mp3s -and -not $mp3Found) {
                    $mp3Found = $mp3s | Select-Object -First 1
                }
            }
        }
        
        $sizeMB = if ($mp3Found) { [math]::Round($mp3Found.Length / 1MB, 1) } else { 0 }
        $sizeFlag = ""
        if ($sizeMB -gt 0 -and $sizeMB -lt 1) { $sizeFlag = "⚠️ TINY" }
        elseif ($sizeMB -gt 20) { $sizeFlag = "⚠️ HUGE" }
        
        $results += [PSCustomObject]@{
            Album = $album
            TrackNum = $trackNum
            Track = $track
            MP3 = if ($mp3Found) { "$($mp3Found.Name) ($sizeMB MB)" } else { "❌ NOT FOUND" }
            Lossless = if ($losslessFound.Count -gt 0) { ($losslessFound | ForEach-Object { "$($_.Name) ($([math]::Round($_.Length/1MB,1)) MB)" }) -join "; " } else { "❌ None" }
            Stems = if ($stemsFound.Count -gt 0) { ($stemsFound | ForEach-Object { $_.Name }) -join "; " } else { "None" }
            SizeFlag = $sizeFlag
        }
    }
}

# Output results
$results | Format-Table -AutoSize -Wrap | Out-String -Width 300 | Out-File "C:\Users\nickf\.openclaw\workspace\music\inventory-results.txt"
$results | Export-Csv "C:\Users\nickf\.openclaw\workspace\music\inventory-results.csv" -NoTypeInformation

# Summary
$total = $results.Count
$hasLossless = ($results | Where-Object { $_.Lossless -ne "❌ None" }).Count
$hasStems = ($results | Where-Object { $_.Stems -ne "None" }).Count
$notFound = ($results | Where-Object { $_.MP3 -like "❌*" }).Count
$tiny = ($results | Where-Object { $_.SizeFlag -like "*TINY*" }).Count
$huge = ($results | Where-Object { $_.SizeFlag -like "*HUGE*" }).Count

Write-Host "`n=== MASTERING INVENTORY SUMMARY ==="
Write-Host "Total tracks: $total"
Write-Host "MP3 found: $($total - $notFound) / $total"
Write-Host "Lossless available: $hasLossless / $total"
Write-Host "Stems available: $hasStems / $total"
Write-Host "Flagged tiny (<1MB): $tiny"
Write-Host "Flagged huge (>20MB): $huge"
Write-Host "Results saved to inventory-results.txt and .csv"
