# Script to download and install squealmate from GitHub
$version = "v0.8.1-alpha"
$downloadUrl = "https://github.com/josefjura/squeal-mate/releases/download/$version/squealmate-x86_64-pc-windows-gnu-$version.zip"
$installPath = "$env:ProgramFiles\squealmate"

# Create the installation folder if it doesn't exist
if (!(Test-Path -Path $installPath)) {
    New-Item -ItemType Directory -Force -Path $installPath
}

# Download the zip file
$zipPath = "$installPath\squealmate.zip"
Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath

# Extract the zip file
Expand-Archive -Path $zipPath -DestinationPath $installPath -Force

# Remove the zip file after extraction
Remove-Item -Path $zipPath

# Add the application path to the PATH environment variable for easy access from any command line
$envPath = [System.Environment]::GetEnvironmentVariable("Path", [System.EnvironmentVariableTarget]::Machine)
if (-not ($envPath -contains $installPath)) {
    [System.Environment]::SetEnvironmentVariable("Path", "$envPath;$installPath", [System.EnvironmentVariableTarget]::Machine)
}

Write-Output "squealmate has been successfully installed to $installPath"
Write-Output "Please restart your terminal to apply the changes."
