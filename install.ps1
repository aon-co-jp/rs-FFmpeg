# rs-ffmpeg installer for Windows
# rs-ffmpeg Windows向けインストーラー
#
# Builds the release binary and installs it to %LOCALAPPDATA%\rs-ffmpeg,
# adding that directory to the current user's PATH.
# リリースビルドを行い、%LOCALAPPDATA%\rs-ffmpeg へ配置してPATHに追加します。

$ErrorActionPreference = "Stop"

Write-Output "Building rs-ffmpeg (release)... / リリースビルド中..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build failed / ビルドに失敗しました"
    exit 1
}

$installDir = Join-Path $env:LOCALAPPDATA "rs-ffmpeg"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item -Force "target\release\rs-ffmpeg.exe" $installDir

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Output "Added $installDir to your user PATH. Restart your terminal to use 'rs-ffmpeg'."
    Write-Output "$installDir をユーザーPATHに追加しました。新しいターミナルで 'rs-ffmpeg' が使えます。"
} else {
    Write-Output "$installDir is already in PATH."
}

Write-Output "Installed: $installDir\rs-ffmpeg.exe"
