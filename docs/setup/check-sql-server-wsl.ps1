# SQL Server WSL Connection Check Script
# Run this in PowerShell as Administrator on Windows

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "SQL Server WSL Connection Checker" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check if SQL Server service is running
Write-Host "1. Checking SQL Server service status..." -ForegroundColor Yellow
$sqlService = Get-Service -Name "MSSQLSERVER" -ErrorAction SilentlyContinue
if ($sqlService) {
    if ($sqlService.Status -eq "Running") {
        Write-Host "   ✓ SQL Server service is running" -ForegroundColor Green
    } else {
        Write-Host "   ✗ SQL Server service is NOT running (Status: $($sqlService.Status))" -ForegroundColor Red
        Write-Host "   → Start it with: Start-Service MSSQLSERVER" -ForegroundColor Yellow
    }
} else {
    Write-Host "   ✗ SQL Server service not found" -ForegroundColor Red
    Write-Host "   → Make sure SQL Server is installed" -ForegroundColor Yellow
}
Write-Host ""

# 2. Check if SQL Server is listening on port 1433
Write-Host "2. Checking if SQL Server is listening on port 1433..." -ForegroundColor Yellow
$listening = netstat -an | Select-String "1433" | Select-String "LISTENING"
if ($listening) {
    Write-Host "   ✓ SQL Server is listening on port 1433" -ForegroundColor Green
    $listening | ForEach-Object { Write-Host "     $_" -ForegroundColor Gray }
} else {
    Write-Host "   ✗ SQL Server is NOT listening on port 1433" -ForegroundColor Red
    Write-Host "   → TCP/IP protocol may be disabled" -ForegroundColor Yellow
    Write-Host "   → Check SQL Server Configuration Manager" -ForegroundColor Yellow
}
Write-Host ""

# 3. Check Windows Firewall rule for port 1433
Write-Host "3. Checking Windows Firewall rules for port 1433..." -ForegroundColor Yellow
$firewallRule = Get-NetFirewallRule | Where-Object {
    $_.Enabled -eq 'True' -and
    $_.Direction -eq 'Inbound'
} | Get-NetFirewallPortFilter | Where-Object {
    $_.LocalPort -eq 1433
}

if ($firewallRule) {
    Write-Host "   ✓ Firewall rule exists for port 1433" -ForegroundColor Green
} else {
    Write-Host "   ✗ No firewall rule found for port 1433" -ForegroundColor Red
    Write-Host "   → Create firewall rule? (y/n): " -ForegroundColor Yellow -NoNewline
    $response = Read-Host
    if ($response -eq 'y' -or $response -eq 'Y') {
        try {
            New-NetFirewallRule -DisplayName "SQL Server (SquealMate)" `
                               -Direction Inbound `
                               -LocalPort 1433 `
                               -Protocol TCP `
                               -Action Allow `
                               -ErrorAction Stop
            Write-Host "   ✓ Firewall rule created successfully" -ForegroundColor Green
        } catch {
            Write-Host "   ✗ Failed to create firewall rule: $_" -ForegroundColor Red
        }
    }
}
Write-Host ""

# 4. Check SQL Server Authentication mode
Write-Host "4. Checking SQL Server authentication mode..." -ForegroundColor Yellow
Write-Host "   (This requires SQL Server PowerShell module)" -ForegroundColor Gray

try {
    Import-Module SqlServer -ErrorAction Stop
    $server = New-Object Microsoft.SqlServer.Management.Smo.Server("localhost")
    $authMode = $server.Settings.LoginMode

    if ($authMode -eq "Mixed") {
        Write-Host "   ✓ SQL Server uses Mixed authentication (SQL + Windows)" -ForegroundColor Green
    } elseif ($authMode -eq "Integrated") {
        Write-Host "   ⚠ SQL Server uses Windows authentication only" -ForegroundColor Yellow
        Write-Host "   → Change to Mixed mode in SSMS if you need SQL authentication" -ForegroundColor Yellow
    } else {
        Write-Host "   ? Authentication mode: $authMode" -ForegroundColor Yellow
    }
} catch {
    Write-Host "   ⚠ Could not check authentication mode (SqlServer module not available)" -ForegroundColor Yellow
    Write-Host "   → Manually verify in SSMS: Server Properties > Security" -ForegroundColor Yellow
}
Write-Host ""

# 5. Get Windows IP addresses
Write-Host "5. Your Windows IP addresses (for WSL config):" -ForegroundColor Yellow
Get-NetIPAddress -AddressFamily IPv4 | Where-Object {
    $_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.254.*"
} | Select-Object IPAddress, InterfaceAlias | Format-Table -AutoSize
Write-Host ""

# Summary
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Next Steps:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "If any checks failed:" -ForegroundColor Yellow
Write-Host "1. Enable TCP/IP protocol:" -ForegroundColor White
Write-Host "   → Open SQL Server Configuration Manager" -ForegroundColor Gray
Write-Host "   → SQL Server Network Configuration > Protocols for MSSQLSERVER" -ForegroundColor Gray
Write-Host "   → Right-click TCP/IP > Enable" -ForegroundColor Gray
Write-Host "   → Restart SQL Server service" -ForegroundColor Gray
Write-Host ""
Write-Host "2. From WSL, test connection:" -ForegroundColor White
Write-Host "   → nc -zv YOUR_WINDOWS_IP 1433" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Configure SquealMate:" -ForegroundColor White
Write-Host "   → squealmate --server YOUR_WINDOWS_IP" -ForegroundColor Gray
Write-Host ""
