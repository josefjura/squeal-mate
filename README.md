# SquealMate

![GitHub Release](https://img.shields.io/github/v/release/josefjura/squeal-mate?include_prereleases)

## Purpose
Squeal Mate is designed to streamline the management of incremental SQL migration scripts. It allows developers to easily track available scripts, monitor execution history, and review execution outcomes. While it may not be universally essential, it is a powerful tool for developers managing databases with incremental migration scripts.

## Features

✨ **Interactive TUI**: Terminal-based interface for browsing and executing SQL scripts
📊 **Progress Tracking**: Visual spinner and "X/Y completed" counter during execution
🔍 **Smart Error Messages**: SQL errors show line numbers with code context
📝 **Script History**: Tracks execution status with CRC32 checksums to detect changes
⚡ **Async Operations**: Non-blocking file loading and checksum calculations
❓ **Context-Sensitive Help**: Press `?` for mode-specific keyboard shortcuts
🔒 **SQL Server 2022 Support**: Full encryption support with self-signed certificates
🎯 **Dual Screen Mode**: Switch between file browser and execution view with Tab

### Status Icons

Scripts are marked with visual indicators showing their execution status:

- `✓` (Green) - Script ran successfully
- `✗` (Red) - Script failed during execution
- `⚠` (Yellow) - Script was modified since last run (re-run recommended)
- `•` (Cyan) - New script, never run before
- `?` (Gray) - Status unknown (checking...)

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

## Usage

`squealmate.exe` provides a set of commands and options for managing database configurations and migrations. Below is a detailed description of each command and option available.

### First Run & Configuration

It's recommended to start with `squealmate init` which will guide you through the initial setup:

```powershell
squealmate init
```

This interactive wizard will help you configure:
- **Repository path**: Location of your SQL migration scripts
- **Database connection**: Server, port, database name
- **Authentication**: SQL Server authentication or Windows integrated auth
- **Encryption settings**: Required for SQL Server 2022+ (supports self-signed certificates)

### Commands

- **`config`**  
  Displays application information and configuration details for the current system, including paths and environment settings.

- **`migrations`**  
  Launches the migrations explorer, allowing you to view and manage database migrations interactively.

- **`init`**
  Assists in setting up the initial configuration file. This command guides you through the setup process and stores configuration settings locally.

- **`setup-db`**
  Interactive wizard that generates a SQL script to create a database user for SquealMate with proper permissions. It also generates a secure password and optionally saves credentials to your config file.

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

## Troubleshooting

### WSL (Windows Subsystem for Linux) Setup

If you're running SquealMate from WSL and SQL Server is installed on your Windows host, you'll need special configuration to connect.

#### Problem
Using `localhost` or `127.0.0.1` from WSL will try to connect to WSL's localhost, not Windows host.

#### Solution

**Option 1: Use Windows Network IP (Recommended)**

This is the most reliable method. Use your Windows machine's actual network IP address:

1. From PowerShell on Windows, find your network IP:
   ```powershell
   ipconfig | findstr IPv4
   ```
   Look for the IPv4 address on your main network adapter (e.g., `192.168.0.178` or `192.168.1.100`)
   - **DO NOT use** `172.x.x.1` (WSL gateway)
   - **DO NOT use** `127.0.0.1` (localhost)

2. Test connectivity from WSL:
   ```bash
   # Replace with your actual Windows IP
   timeout 2 bash -c "cat < /dev/null > /dev/tcp/192.168.0.178/1433" && echo "✓ Connected" || echo "✗ Failed"
   ```

3. Configure SquealMate to use this IP:
   ```bash
   squealmate init
   # When asked for "Database url", enter your Windows network IP (e.g., 192.168.0.178)
   ```

**Option 2: Use WSL2 Host Gateway (Less Reliable)**

The WSL gateway IP (`172.x.x.1`) *should* work but often has connectivity issues:

```bash
# Find the gateway IP
ip route show | grep -i default | awk '{ print $3}'

# Try connecting with it
squealmate --server $(ip route show | grep -i default | awk '{ print $3}')
```

**If the gateway IP doesn't work**, use Option 1 instead (your Windows network IP)

#### Firewall Configuration

Make sure Windows Firewall allows SQL Server connections from all network profiles:

```powershell
# Run in PowerShell as Administrator on Windows
New-NetFirewallRule -DisplayName "SQL Server (All Networks)" `
                   -Direction Inbound `
                   -LocalPort 1433 `
                   -Protocol TCP `
                   -Action Allow `
                   -Profile Domain,Private,Public
```

#### Quick Setup Script (Windows)

We provide a PowerShell script to automatically check and fix common issues:

```powershell
# Run in PowerShell as Administrator on Windows
.\docs\setup\check-sql-server-wsl.ps1
```

This script will:
- ✓ Check if SQL Server is running
- ✓ Verify TCP/IP is enabled and listening on port 1433
- ✓ Check Windows Firewall rules
- ✓ Show your Windows IP addresses for WSL config
- ✓ Optionally create firewall rules for you

#### Testing the Connection

From WSL, test if SQL Server is reachable:

```bash
# Test if port is open (requires netcat)
nc -zv YOUR_WINDOWS_IP 1433

# If you get "Connection refused" or timeout:
# - Check SQL Server is running on Windows
# - Verify Windows Firewall allows port 1433
# - Ensure SQL Server is configured for TCP/IP connections
```

### SQL Server Authentication Issues

**Error: "Login failed for user"**

1. Ensure SQL Server authentication is enabled:
   - Open SQL Server Management Studio (SSMS)
   - Right-click server → **Properties** → **Security**
   - Select **SQL Server and Windows Authentication mode**
   - Restart SQL Server

2. Create a SQL Server user:
   ```bash
   squealmate setup-db
   ```
   This wizard will generate a SQL script to create the user with proper permissions.

3. Run the generated SQL script in SSMS or via sqlcmd

**Error: "A connection was successfully established with the server, but then an error occurred"**

This usually indicates an encryption mismatch. Try:

```bash
# For SQL Server 2019 and older (no encryption)
squealmate --server YOUR_SERVER # Add to config with encryption = "not_supported"

# For SQL Server 2022+ with self-signed certificate
# Make sure your config.toml has:
# encryption = "required"
# trust_server_certificate = true
```

### General Connection Issues

1. **Test basic connectivity:**
   ```bash
   # From WSL
   ping YOUR_WINDOWS_IP
   telnet YOUR_WINDOWS_IP 1433
   ```

2. **Check SQL Server is listening:**
   ```powershell
   # From Windows PowerShell
   netstat -an | findstr 1433
   # Should show: TCP    0.0.0.0:1433    ...    LISTENING
   ```

3. **Verify SQL Server Browser is running** (for named instances)

4. **Check your config file location:**
   ```bash
   squealmate config
   ```

## Contributing
TODO

## License
This project is licensed under the terms specified in the [LICENSE.txt](./LICENSE.txt) file. 
