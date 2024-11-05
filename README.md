# SquealMate

![GitHub Release](https://img.shields.io/github/v/release/josefjura/squeal-mate?include_prereleases)

## Purpose
Squeal Mate is designed to streamline the management of incremental SQL migration scripts. It allows developers to easily track available scripts, monitor execution history, and review execution outcomes. While it may not be universally essential, it is a powerful tool for developers managing databases with incremental migration scripts.

Here’s a Markdown section for your GitHub README that provides installation instructions using `curl` and `iex` to run your PowerShell installation script directly:

---

## Installation

To install **squealmate** on Windows, you can use the following one-liner in PowerShell. This command downloads and runs the installation script automatically, so there’s no need to manually download or unzip files.

### Using PowerShell

1. Open PowerShell as Administrator.
2. Run the following command:

   ```powershell
   iex (New-Object Net.WebClient).DownloadString("https://github.com/josefjura/squeal-mate/raw/master/deployment/install_windows.ps1")
   ```

This will:
- Download the latest `squealmate` release from GitHub
- Extract it to `C:\Program Files\squealmate`
- Add `squealmate` to your PATH for easy access from any command line

3. **Restart your terminal** to ensure the `PATH` changes take effect.

Once installed, you can start using `squealmate` by simply typing:

```powershell
squealmate
```

### Manual Installation

If you prefer manual installation, you can download the latest release from the [GitHub Releases page](https://github.com/josefjura/squeal-mate/releases), extract the files to a folder of your choice, and add that folder to your PATH.

---

This section will guide users through the PowerShell installation process with minimal effort!

## Usage
TODO

## Contributing
TODO

## License
This project is licensed under the terms specified in the [LICENSE.txt](./LICENSE.txt) file. 
