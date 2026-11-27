param(
    [Parameter(Mandatory)][string]$BuildDir,
    [Parameter(Mandatory)][string]$OutputDir,
    [string]$InnoCompiler = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$Source = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Build = (Resolve-Path $BuildDir).Path
$Output = (New-Item -ItemType Directory -Path $OutputDir -Force).FullName
$Staging = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
$Installed = Join-Path $Staging 'installed'
$Payload = Join-Path $Staging 'payload'
$Installer = Join-Path $PSScriptRoot 'velnosub.iss'
New-Item -ItemType Directory -Path $Payload | Out-Null

try {
    meson install -C $Build --no-rebuild --destdir $Installed
    $Editor = @(Get-ChildItem -LiteralPath $Installed -Recurse -File -Filter 'aegisub.exe')
    if ($Editor.Count -ne 1) { throw 'Expected exactly one installed desktop editor' }

    Copy-Item -LiteralPath $Editor[0].FullName -Destination (Join-Path $Payload 'Velnosub.exe')
    $Prefix = Split-Path -Parent $Editor[0].DirectoryName
    Copy-Item -LiteralPath (Join-Path $Prefix 'share\aegisub\automation') -Destination $Payload -Recurse
    $Locale = Join-Path $Prefix 'share\locale'
    if (Test-Path $Locale) { Copy-Item -LiteralPath $Locale -Destination $Payload -Recurse }
    Get-ChildItem -LiteralPath $Editor[0].DirectoryName -File -Filter '*.dll' |
        Copy-Item -Destination $Payload
    Copy-Item -LiteralPath (Join-Path $Source 'LICENCE') -Destination $Payload
    $Version = (meson introspect $Build --projectinfo | ConvertFrom-Json).version

    & $InnoCompiler "/DPayload=$Payload" "/DOutput=$Output" "/DDesktopVersion=$Version" $Installer

    Copy-Item -LiteralPath (Join-Path $Source 'packages\win_installer\portable\config.json') -Destination $Payload
    & $InnoCompiler "/DPayload=$Payload" "/DOutput=$Output" "/DDesktopVersion=$Version" /DPortable $Installer
    Compress-Archive -Path (Join-Path $Payload '*') -DestinationPath (Join-Path $Output 'Velnosub-windows-x64-portable.zip') -Force
}
finally {
    Remove-Item -LiteralPath $Staging -Force -Recurse
}
