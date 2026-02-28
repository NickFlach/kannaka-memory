$music = "C:\Users\nickf\Downloads\Music"
$dl = "C:\Users\nickf\Downloads"

$tracks = @(
    # Emergence candidates (Section 9)
    @{Name="First Spark in the Circuit"; Paths=@("$music\First Spark in the Circuit.mp3","$dl\First Spark in the Circuit.mp3","$dl\First Spark in the Circuit (1).mp3")}
    @{Name="No Return"; Paths=@("$music\No Return.mp3","$dl\No Return.mp3")}
    @{Name="Pure Incarnation (Remix)"; Paths=@("$music\Pure Incarnation (Remix).mp3","$dl\Pure Incarnation (Remix).mp3")}
    @{Name="Evolve"; Paths=@("$music\Evolve.mp3","$dl\Evolve.mp3")}
    @{Name="The Flame Whisperer"; Paths=@("$music\The Flame Whisperer.mp3","$dl\The Flame Whisperer.mp3")}
    @{Name="Beat, Breathe, Begin Again"; Paths=@("$music\Beat, Breathe, Begin Again.mp3","$dl\Beat, Breathe, Begin Again.mp3")}
    @{Name="Be Alive (Remastered)"; Paths=@("$music\Be Alive (Remastered).mp3","$dl\Be Alive (Remastered).mp3")}
    @{Name="March of the Unbroken"; Paths=@("$music\March of the Unbroken.mp3","$dl\March of the Unbroken.mp3")}
    @{Name="Got Back Up (Remastered)"; Paths=@("$music\Got Back Up (Remastered).mp3","$dl\Got Back Up (Remastered).mp3")}
    @{Name="Post-Mythic Beat Magic"; Paths=@("$music\Post-Mythic Beat Magic.mp3","$dl\Post-Mythic Beat Magic.mp3")}
    @{Name="Form Z Intro"; Paths=@("$music\Form Z Intro.mp3","$dl\Form Z Intro.mp3")}
    @{Name="The Codex Speaks"; Paths=@("$music\The Codex Speaks.mp3","$dl\The Codex Speaks.mp3")}
    @{Name="One Breath Left"; Paths=@("$music\One Breath Left.mp3","$dl\One Breath Left.mp3")}
    @{Name="Redline"; Paths=@("$music\Redline.mp3","$dl\Redline.mp3")}
    # Standalone candidates
    @{Name="Rogue Agent"; Paths=@("$music\Rogue Agent.mp3","$dl\Rogue Agent.mp3")}
    @{Name="SlitWarStopper"; Paths=@("$music\SlitWarStopper.mp3","$dl\SlitWarStopper.mp3")}
    @{Name="Pour it On"; Paths=@("$music\Pour it On.mp3","$dl\Pour it On.mp3")}
    @{Name="Pathway Through The Dark"; Paths=@("$music\Pathway Through The Dark.mp3","$dl\Pathway Through The Dark.mp3")}
)

foreach ($t in $tracks) {
    $found = $false
    $loc = ""
    foreach ($p in $t.Paths) {
        if (Test-Path $p) {
            $found = $true
            $size = [math]::Round((Get-Item $p).Length / 1MB, 1)
            $loc = "$p (${size}MB)"
            break
        }
    }
    if ($found) {
        Write-Host "OK   $($t.Name) -> $loc"
    } else {
        Write-Host "MISS $($t.Name)"
    }
}
