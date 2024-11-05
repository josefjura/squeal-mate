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
   irm -useb https://github.com/josefjura/squeal-mate/raw/master/deployment/install_windows.ps1 | iex
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

`squealmate.exe` provides a set of commands and options for managing database configurations and migrations. Below is a detailed description of each command and option available.

### First run

It's recommended to start with `squealmate init` which will help you with first setup.

### Commands

- **`config`**  
  Displays application information and configuration details for the current system, including paths and environment settings.

- **`migrations`**  
  Launches the migrations explorer, allowing you to view and manage database migrations interactively.

- **`init`**  
  Assists in setting up the initial configuration file. This command guides you through the setup process and stores configuration settings locally.

- **`help`**  
  Provides help information. Use this command to view usage details for specific commands or options.

### Options

- **`-s`, `--server <SERVER>`**  
  Specify the database server URL. If not provided, it defaults to `localhost`.

- **`--port <PORT>`**  
  Set the port number for the database connection. Defaults to `1433` if omitted.

- **`-u`, `--username <USERNAME>`**  
  Define the username to log into the database. Required unless integrated authentication is used.

- **`-p`, `--password <PASSWORD>`**  
  Specify the password associated with the database username. This option is also skipped if integrated authentication is enabled.

- **`-n`, `--name <NAME>`**  
  The name of the database you wish to connect to.

- **`-i`, `--is-integrated <IS_INTEGRATED>`**  
  Enable integrated authentication by setting this option to `true`, which bypasses the need for a username and password. Accepts values `true` or `false`.

- **`-h`, `--help`**  
  Display help information for the main command or for a specific subcommand when combined with a command.

- **`-V`, `--version`**  
  Output the version information for `squealmate`.

## Contributing
TODO

## License
This project is licensed under the terms specified in the [LICENSE.txt](./LICENSE.txt) file. 
