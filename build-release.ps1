# Builds dist/SaboteurFix-<version>.zip: plugin (dinput8.dll + ini), installer, DXVK d3d9.dll + license, pack patches.
param([string]$Version = '2.0.0', [string]$DxvkVersion = '3.1.1')
$ErrorActionPreference = 'Stop'
$root  = Split-Path -Parent $MyInvocation.MyCommand.Path
$dist  = Join-Path $root 'dist'
$stage = Join-Path $dist 'SaboteurFix'
$cache = Join-Path $dist 'cache'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force $stage, $cache, (Join-Path $stage 'dxvk') | Out-Null

Push-Location (Join-Path $root 'plugin'); cargo build --release --target i686-pc-windows-msvc; Pop-Location
Copy-Item (Join-Path $root 'plugin\target\i686-pc-windows-msvc\release\dinput8.dll') $stage
Copy-Item (Join-Path $root 'plugin\SaboteurFix.ini') $stage

$tgz = Join-Path $cache "dxvk-$DxvkVersion.tar.gz"
if (-not (Test-Path $tgz)) { Invoke-WebRequest "https://github.com/doitsujin/dxvk/releases/download/v$DxvkVersion/dxvk-$DxvkVersion.tar.gz" -OutFile $tgz }
tar -xzf $tgz -C $cache
Copy-Item (Join-Path $cache "dxvk-$DxvkVersion\x32\d3d9.dll") (Join-Path $stage 'dxvk\d3d9.dll')
Invoke-WebRequest "https://raw.githubusercontent.com/doitsujin/dxvk/v$DxvkVersion/LICENSE" -OutFile (Join-Path $stage 'dxvk\LICENSE-DXVK.txt')

foreach ($f in 'Install.bat', 'Uninstall.bat', 'SaboteurFpsUnlock.ps1', 'dxvk.conf', 'camera_pack_patch.json', 'foliage_pack_patch.json', 'fov_pack_patch.json') { Copy-Item (Join-Path $root "mod\$f") $stage }
Copy-Item (Join-Path $root 'README.md') $stage
Copy-Item (Join-Path $root 'LICENSE') (Join-Path $stage 'LICENSE.txt')

$zip = Join-Path $dist "SaboteurFix-$Version.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Host "Built $zip"
