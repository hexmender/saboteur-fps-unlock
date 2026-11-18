<#
  SaboteurFix installer (v2, plugin-based).
  Installs: dinput8.dll (SaboteurFix plugin, patches the game in memory - Saboteur.exe stays untouched)
            SaboteurFix.ini (all options), DXVK d3d9.dll + dxvk.conf (low latency, FPS cap, 16x AF),
            camera + foliage data patches in France\loosefiles_BinPC.pack (with backup),
            config.ini: Windowed 1 (borderless fullscreen via the plugin), VSync 0.
  Install.bat / Uninstall.bat, or:
    powershell -ExecutionPolicy Bypass -File SaboteurFpsUnlock.ps1 [-GameDir <path>] [-Uninstall] [-NoDxvk] [-NoConfig] [-NoCamera] [-NoFoliage] [-ResetIni]
#>
param(
    [string]$GameDir,
    [switch]$Uninstall,
    [switch]$NoDxvk,
    [switch]$NoConfig,
    [switch]$NoCamera,
    [switch]$NoFoliage,
    [switch]$ResetIni,
    [switch]$NoVehicleLod,
    [switch]$NoCrowds
)
$ErrorActionPreference = 'Stop'
$ModDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$GogHash = '628CDBB64A5317FEF66D3C2FE87AB063B0C659DFC92F5882B85828F90568E4EF'

function Write-Step($m) { Write-Host "  > $m" -ForegroundColor Cyan }
function Write-Ok($m)   { Write-Host "  + $m" -ForegroundColor Green }
function Write-Warn2($m){ Write-Host "  ! $m" -ForegroundColor Yellow }

function Find-GameDir {
    if ($GameDir) { return $GameDir }
    if (Test-Path (Join-Path $ModDir 'Saboteur.exe')) { return $ModDir }
    $parent = Split-Path -Parent $ModDir
    if ($parent -and (Test-Path (Join-Path $parent 'Saboteur.exe'))) { return $parent }
    foreach ($k in 'HKLM:\SOFTWARE\WOW6432Node\Electronic Arts\The Saboteur', 'HKLM:\SOFTWARE\Electronic Arts\The Saboteur') {
        try { $d = (Get-ItemProperty $k -ErrorAction Stop).'Install Dir'; if ($d -and (Test-Path (Join-Path $d 'Saboteur.exe'))) { return $d.TrimEnd('\') } } catch {}
    }
    foreach ($d in 'C:\GOG Games\The Saboteur', 'C:\Program Files (x86)\Steam\steamapps\common\The Saboteur',
                   'C:\Program Files (x86)\Origin Games\The Saboteur', 'C:\Program Files\EA Games\The Saboteur') {
        if (Test-Path (Join-Path $d 'Saboteur.exe')) { return $d }
    }
    throw "Could not find the game. Put this folder inside the game folder, or run with -GameDir 'C:\path\to\The Saboteur'."
}
function Find-ConfigIni {
    $docs = [Environment]::GetFolderPath('MyDocuments')
    Get-ChildItem (Join-Path $docs 'My Games') -Directory -Filter 'The Saboteur*' -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName 'SaveGames\config.ini' } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
function Set-IniValue([string]$t, [string]$k, [string]$v) {
    if ($t -match "(?m)^$k\s") { return ($t -replace "(?m)^$k\s+\S+", "$k`t$v") }
    return ($t.TrimEnd("`r", "`n") + "`r`n$k`t$v`r`n")
}
function ConvertFrom-Hex([string]$h) { $r = New-Object byte[] ($h.Length / 2); for ($i = 0; $i -lt $r.Length; $i++) { $r[$i] = [Convert]::ToByte($h.Substring($i * 2, 2), 16) }; return ,$r }
function Test-Bytes([byte[]]$b, [int]$o, [byte[]]$e) { if ($o + $e.Length -gt $b.Length) { return $false }; for ($i = 0; $i -lt $e.Length; $i++) { if ($b[$o + $i] -ne $e[$i]) { return $false } }; return $true }

function Apply-PackPatch([string]$game, [string]$json, [string]$label) {
    $pj = Join-Path $ModDir $json
    $pack = Join-Path $game 'France\loosefiles_BinPC.pack'
    if (-not (Test-Path -LiteralPath $pj) -or -not (Test-Path -LiteralPath $pack)) { Write-Warn2 "$label skipped (file missing)"; return }
    $spec = Get-Content -LiteralPath $pj -Raw | ConvertFrom-Json
    $pb = [IO.File]::ReadAllBytes($pack)
    if ($pb.Length -ne [int64]$spec.size) { Write-Warn2 "$label skipped: pack size differs from the GOG build"; return }
    foreach ($r in $spec.patches) {
        $o = [int]$r[0]
        if (-not ((Test-Bytes $pb $o (ConvertFrom-Hex $r[1])) -or (Test-Bytes $pb $o (ConvertFrom-Hex $r[2])))) { Write-Warn2 "$label skipped: pack contents differ (other mod?)"; return }
    }
    if (-not (Test-Path -LiteralPath "$pack.orig")) { Copy-Item -LiteralPath $pack "$pack.orig"; Write-Ok 'Backup: loosefiles_BinPC.pack.orig' }
    foreach ($r in $spec.patches) { (ConvertFrom-Hex $r[2]).CopyTo($pb, [int]$r[0]) }
    [IO.File]::WriteAllBytes($pack, $pb)
    Write-Ok "$label ($($spec.patches.Count) edits)"
}

Write-Host ''; Write-Host '  The Saboteur - SaboteurFix' -ForegroundColor White; Write-Host ''
$game = Find-GameDir
$exe = Join-Path $game 'Saboteur.exe'; $bak = Join-Path $game 'Saboteur.exe.orig'
Write-Step "Game folder: $game"

# v1 of this mod patched Saboteur.exe on disk; the plugin needs the original exe.
if (Test-Path -LiteralPath $bak) {
    if ((Get-FileHash -LiteralPath $exe).Hash -ne (Get-FileHash -LiteralPath $bak).Hash) { Copy-Item -LiteralPath $bak $exe -Force; Write-Ok 'Restored original Saboteur.exe (v1 exe patch removed)' }
}
foreach ($bk in 'saboteur.bik', 'saboteur_de.bik') {   # v1 intro hack -> plugin skips the intro in code
    $p = Join-Path $game "Bink\$bk"
    if (Test-Path -LiteralPath "$p.orig") { Copy-Item -LiteralPath "$p.orig" $p -Force; Remove-Item -LiteralPath "$p.orig" -Force; Write-Ok "Restored $bk" }
}

if ($Uninstall) {
    $ours = (Test-Path -LiteralPath (Join-Path $game 'dxvk.conf')) -and (Select-String -LiteralPath (Join-Path $game 'dxvk.conf') -Pattern 'The Saboteur - FPS Unlock' -Quiet)
    $list = @('dinput8.dll', 'SaboteurFix.log'); if ($ours) { $list += 'd3d9.dll', 'dxvk.conf' }
    foreach ($f in $list) { $p = Join-Path $game $f; if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force; Write-Ok "Removed $f" } }
    $ini = Join-Path $game 'SaboteurFix.ini'; if (Test-Path -LiteralPath $ini) { Remove-Item -LiteralPath $ini -Force; Write-Ok 'Removed SaboteurFix.ini' }
    $pk = Join-Path $game 'France\loosefiles_BinPC.pack'
    if (Test-Path -LiteralPath "$pk.orig") { Copy-Item -LiteralPath "$pk.orig" $pk -Force; Remove-Item -LiteralPath "$pk.orig" -Force; Write-Ok 'Restored loosefiles_BinPC.pack' }
    $ai = Join-Path $game 'ai.ini'
    if (Test-Path -LiteralPath "$ai.orig") { Copy-Item -LiteralPath "$ai.orig" $ai -Force; Remove-Item -LiteralPath "$ai.orig" -Force; Write-Ok 'Restored ai.ini' }
    $mi = Join-Path $game 'ModelInfo.txt'
    if (Test-Path -LiteralPath "$mi.orig") { Copy-Item -LiteralPath "$mi.orig" $mi -Force; Remove-Item -LiteralPath "$mi.orig" -Force; Write-Ok 'Restored ModelInfo.txt' }
    if (Test-Path -LiteralPath $bak) { Remove-Item -LiteralPath $bak -Force }
    $c = if ($NoConfig) { $null } else { Find-ConfigIni }
    if ($c) { $t = [IO.File]::ReadAllText($c) -replace "(?m)^Windowed\s+\S+\r?\n?", ''; [IO.File]::WriteAllText($c, $t, [Text.Encoding]::ASCII); Write-Ok 'Removed Windowed from config.ini' }
    Write-Host ''; Write-Ok 'Uninstalled.'; return
}

$h = (Get-FileHash -LiteralPath $exe).Hash
if ($h -ne $GogHash) { Write-Warn2 'Saboteur.exe is not the GOG 1.0 build. The plugin locates most fixes by signature and skips what it cannot verify (see SaboteurFix.log).' }

Copy-Item -LiteralPath (Join-Path $ModDir 'dinput8.dll') (Join-Path $game 'dinput8.dll') -Force
Write-Ok 'SaboteurFix plugin installed (dinput8.dll)'
$iniDst = Join-Path $game 'SaboteurFix.ini'
if ($ResetIni -or -not (Test-Path -LiteralPath $iniDst)) { Copy-Item -LiteralPath (Join-Path $ModDir 'SaboteurFix.ini') $iniDst -Force; Write-Ok 'SaboteurFix.ini installed (edit it to tune every fix)' }
else { Write-Ok 'Kept your existing SaboteurFix.ini (use -ResetIni to overwrite)' }

if (-not $NoDxvk) {
    $dll = Join-Path $ModDir 'dxvk\d3d9.dll'
    if (Test-Path -LiteralPath $dll) {
        Copy-Item -LiteralPath $dll (Join-Path $game 'd3d9.dll') -Force
        Copy-Item -LiteralPath (Join-Path $ModDir 'dxvk.conf') (Join-Path $game 'dxvk.conf') -Force
        Write-Ok 'DXVK installed (low input lag, 240 FPS cap, 16x AF) - edit dxvk.conf for your refresh rate'
    } else { Write-Warn2 'dxvk\d3d9.dll missing - DXVK skipped (the game will run uncapped)' }
}
if (-not $NoVehicleLod) {
    $mi = Join-Path $game 'ModelInfo.txt'
    if (Test-Path -LiteralPath $mi) {
        if (-not (Test-Path -LiteralPath "$mi.orig")) { Copy-Item -LiteralPath $mi "$mi.orig" }
        $lines = Get-Content -LiteralPath "$mi.orig"; $n = 0
        $out = foreach ($l in $lines) {
            if ($l -match '^VH_' -and $l -match 'LODDIST(25|30)\b') { $n++; ($l -replace 'LODDIST(25|30)\b', 'LODDIST90') -replace 'SHADOWSLICE3\b', 'SHADOWSLICE4' } else { $l }
        }
        Set-Content -LiteralPath $mi $out
        Write-Ok "Vehicles keep full detail to 90 m (was 25-30 m) and cast farther shadows ($n models)"
    }
}
if (-not $NoCrowds) {
    $ai = Join-Path $game 'ai.ini'
    if (Test-Path -LiteralPath $ai) {
        if (-not (Test-Path -LiteralPath "$ai.orig")) { Copy-Item -LiteralPath $ai "$ai.orig" }
        $t = [IO.File]::ReadAllText("$ai.orig")
        foreach ($kv in @(@('Disposables_MaxMale','20','30'),@('Disposables_MaxFemale','20','30'),@('Disposables_MaxVendor','10','15'),@('Disposables_NaziPadding','30','40'))) { $t = [regex]::Replace($t, "(?m)^($($kv[0])\s+)$($kv[1])\b", "`${1}$($kv[2])") }
        [IO.File]::WriteAllText($ai, $t)
        Write-Ok 'Pedestrians x1.5 (ai.ini, backup ai.ini.orig)'
    }
}
if (-not $NoCamera)  { Apply-PackPatch $game 'camera_pack_patch.json'  'Camera smoothing removed' }
if (-not $NoFoliage) { Apply-PackPatch $game 'foliage_pack_patch.json' 'Foliage draw distance x2' }
if (-not $NoCamera)  { Apply-PackPatch $game 'fov_pack_patch.json' 'Camera FOV changes settle 5x faster' }

if (-not $NoConfig) {
    $c = Find-ConfigIni
    if ($c) {
        Copy-Item -LiteralPath $c "$c.bak" -Force
        $t = [IO.File]::ReadAllText($c)
        $t = Set-IniValue $t 'VSync' '0'; $t = Set-IniValue $t 'RefreshRate' '60'; $t = Set-IniValue $t 'Windowed' '1'
        [IO.File]::WriteAllText($c, $t, [Text.Encoding]::ASCII)
        Write-Ok 'config.ini: Windowed 1 (borderless fullscreen), VSync 0 (backup: config.ini.bak)'
    } else { Write-Warn2 'config.ini not found: start the game once, save, quit, then run Install again.' }
}
Write-Host ''; Write-Ok "Done. Set Windows to your monitor's max refresh rate and play. Options: SaboteurFix.ini"; Write-Host ''
