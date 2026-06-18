$destDir = Join-Path $PSScriptRoot "..\src-tauri\resources\7z"
$destExe = Join-Path $destDir "7z.exe"
$destDll = Join-Path $destDir "7z.dll"

if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
}

$sources = @(
    "C:\Program Files\7-Zip\7z.exe",
    "C:\Program Files (x86)\7-Zip\7z.exe"
)

$found = $false
foreach ($src in $sources) {
    if (Test-Path $src) {
        Copy-Item $src $destExe -Force
        $dll = Join-Path (Split-Path $src) "7z.dll"
        if (Test-Path $dll) {
            Copy-Item $dll $destDll -Force
        }
        Write-Host "Copied 7-Zip from $src"
        $found = $true
        break
    }
}

if (-not $found) {
    Write-Warning "7-Zip not found. Install 7-Zip or place 7z.exe in src-tauri/resources/7z/"
    exit 0
}
