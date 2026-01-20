# PowerShell development script for Windows
# This script starts the Vibe Kanban development environment

Write-Host "Starting Vibe Kanban development environment..." -ForegroundColor Cyan

# Get ports from setup script
Write-Host "Allocating ports..." -ForegroundColor Yellow
$portsOutput = node scripts/setup-dev-environment.js get 2>&1
$portsJsonLine = $portsOutput | Where-Object { $_ -match '^\s*\{' } | Select-Object -Last 1
$ports = $portsJsonLine | ConvertFrom-Json

$frontendPort = $ports.frontend
$backendPort = $ports.backend

Write-Host "Frontend port: $frontendPort" -ForegroundColor Green
Write-Host "Backend port: $backendPort" -ForegroundColor Green

# Set environment variables
$env:FRONTEND_PORT = $frontendPort
$env:BACKEND_PORT = $backendPort
$env:VK_ALLOWED_ORIGINS = "http://localhost:$frontendPort"
$env:VITE_VK_SHARED_API_BASE = if ($env:VK_SHARED_API_BASE) { $env:VK_SHARED_API_BASE } else { "" }
$env:VITE_OPEN = "false"
$env:DISABLE_WORKTREE_ORPHAN_CLEANUP = "1"
$env:RUST_LOG = "debug"
$env:HOST = "127.0.0.1"
$env:MCP_HOST = "127.0.0.1"
$env:MCP_PORT = $backendPort

Write-Host ""
Write-Host "Environment configured:" -ForegroundColor Cyan
Write-Host "   FRONTEND_PORT: $env:FRONTEND_PORT" -ForegroundColor Gray
Write-Host "   BACKEND_PORT: $env:BACKEND_PORT" -ForegroundColor Gray
Write-Host "   VK_ALLOWED_ORIGINS: $env:VK_ALLOWED_ORIGINS" -ForegroundColor Gray
Write-Host ""

# Start both backend and frontend using concurrently
Write-Host "Starting backend and frontend servers..." -ForegroundColor Cyan
Write-Host ""

# Run concurrently with the environment variables set
# We run the commands directly instead of through package.json scripts to avoid Unix-style env var syntax
npx concurrently --names "BACKEND,FRONTEND" --prefix-colors "blue,magenta" "cargo watch -w crates -x 'run --bin server'" "cd frontend && npx vite --port $frontendPort --host"
