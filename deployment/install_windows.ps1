# Set error action to stop on any error
$ErrorActionPreference = 'stop'

# GitHub Org and Repo for squealmate
$GitHubOrg = "josefjura"
$GitHubRepo = "squeal-mate"

# Installation path in a non-privileged location for testing
$InstallDir = "$env:LOCALAPPDATA\squealmate"
$ExecutableName = "squealmate.exe"
$ExecutablePath = "${InstallDir}\${ExecutableName}"

Write-Output "Installing squealmate..."
Write-Output "Creating $InstallDir directory"
New-Item -ErrorAction Ignore -Path $InstallDir -ItemType "directory"
if (!(Test-Path $InstallDir -PathType Container)) {
    throw "Could not create $InstallDir"
}

# Fetch the latest release from GitHub
Write-Output "Fetching the latest squealmate release..."
$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/${GitHubOrg}/${GitHubRepo}/releases" -Method Get
if ($releases.Count -eq 0) {
    throw "No releases found in github.com/$GitHubOrg/$GitHubRepo repository"
}

# Find the Windows asset
$windowsAsset = $releases[0].assets | Where-Object { $_.name -Like "*windows-gnu*.zip" }
if (!$windowsAsset) {
    throw "Cannot find the Windows squealmate archive"
}

# Log the URL to verify correctness
$zipFilePath = "${InstallDir}\${windowsAsset.name}\squealmate.zip"
Write-Output "Found the latest release: $($windowsAsset.browser_download_url)"
Write-Output "Downloading $($windowsAsset.name) to $zipFilePath..."

# Attempt download with Invoke-WebRequest or fallback to curl
try {
    # Attempt to download with Invoke-WebRequest
    Invoke-WebRequest -Uri $windowsAsset.browser_download_url -OutFile $zipFilePath -UseBasicParsing
} catch {
    Write-Output "Invoke-WebRequest failed. Attempting download with curl..."
    $curlCommand = "curl -L -o `"$zipFilePath`" `"$($windowsAsset.browser_download_url)`""
    Write-Output "Running curl command: $curlCommand"
    Invoke-Expression $curlCommand
}

# Confirm download success
if (!(Test-Path $zipFilePath -PathType Leaf)) {
    throw "Failed to download squealmate - $zipFilePath"
}

Write-Output "Successfully downloaded squealmate to $zipFilePath."

# Extract the zip file
Write-Output "Extracting $zipFilePath to $InstallDir..."
Expand-Archive -Force -Path $zipFilePath -DestinationPath $InstallDir
if (!(Test-Path $ExecutablePath -PathType Leaf)) {
    throw "Failed to extract squealmate executable - $ExecutablePath"
}

# Verify the installation by checking the version
Write-Output "Verifying installation..."
Invoke-Expression "$ExecutablePath --version"

# Clean up the downloaded zip file
Write-Output "Cleaning up $zipFilePath..."
Remove-Item $zipFilePath -Force

# Add the installation directory to the User Path environment variable
Write-Output "Adding $InstallDir to the User PATH..."
$UserPathEnvironmentVar = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPathEnvironmentVar -notlike "*$InstallDir*") {
    [System.Environment]::SetEnvironmentVariable("PATH", "$UserPathEnvironmentVar;$InstallDir", "User")
    Write-Output "Successfully added $InstallDir to the PATH."
} else {
    Write-Output "Path $InstallDir is already in the User PATH."
}

Write-Output "`r`nSquealmate has been installed successfully!"
Write-Output "Please restart your terminal to apply the PATH changes."
Write-Output "For more information, visit https://github.com/$GitHubOrg/$GitHubRepo"
