# Installation Guide

<p align="center">
  <a href="https://vibekanban.com">
    <picture>
      <source srcset="frontend/public/vibe-kanban-logo-dark.svg" media="(prefers-color-scheme: dark)">
      <source srcset="frontend/public/vibe-kanban-logo.svg" media="(prefers-color-scheme: light)">
      <img src="frontend/public/vibe-kanban-logo.svg" alt="Vibe Kanban Logo" width="400">
    </picture>
  </a>
</p>

<p align="center">
  <strong>AI Coding Agent Orchestration Platform</strong><br>
  Get 10X more out of Claude Code, Gemini CLI, Codex, Amp and other coding agents
</p>

---

## Table of Contents

- [Quick Start](#quick-start)
- [System Requirements](#system-requirements)
- [Installation Methods](#installation-methods)
  - [Method 1: NPX (Recommended)](#method-1-npx-recommended)
  - [Method 2: Global NPM Installation](#method-2-global-npm-installation)
  - [Method 3: Build from Source](#method-3-build-from-source)
  - [Method 4: Docker Container](#method-4-docker-container)
- [Platform-Specific Instructions](#platform-specific-instructions)
  - [Windows](#windows)
  - [macOS](#macos)
  - [Linux](#linux)
- [Post-Installation Setup](#post-installation-setup)
- [Configuration](#configuration)
- [Self-Hosted Deployment](#self-hosted-deployment)
- [Remote Deployment](#remote-deployment)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

---

## Quick Start

The fastest way to get started with Vibe Kanban is using `npx`:

```bash
npx vibe-kanban
```

This command will:
1. Download and run Vibe Kanban automatically
2. Start the application locally
3. Open it in your default browser
4. No installation required!

**Prerequisites**: Make sure you have authenticated with your favourite coding agent first. See the [list of supported agents](https://vibekanban.com/docs).

---

## System Requirements

### Minimum Requirements

- **Node.js**: >= 18.0.0
- **Git**: >= 2.0.0 (for repository operations)
- **Operating System**:
  - Windows 10/11 (x64)
  - macOS 10.15+ (Intel x64 or Apple Silicon ARM64)
  - Linux (x64)

### Recommended Requirements

- **RAM**: 4GB minimum, 8GB recommended
- **Disk Space**: 500MB free space
- **Network**: Internet connection for downloading binaries and dependencies

### For Development (Building from Source)

- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Node.js**: >= 18.0.0
- **pnpm**: >= 8.0.0
- **Cargo tools** (optional):
  - `cargo-watch` - for development auto-reload
  - `sqlx-cli` - for database migrations

---

## Installation Methods

### Method 1: NPX (Recommended)

**Best for**: End users who want to run Vibe Kanban without permanent installation.

```bash
npx vibe-kanban
```

**Advantages**:
- No installation required
- Always runs the latest version
- Easy to update (just run the command again)
- Minimal disk space usage

**How it works**:
- Downloads platform-specific binaries on first run
- Caches them locally for faster subsequent runs
- Automatically handles updates

---

### Method 2: Global NPM Installation

**Best for**: Users who want Vibe Kanban permanently installed and available from any directory.

```bash
npm install -g vibe-kanban
```

Then run from anywhere:

```bash
vibe-kanban
```

**To update**:
```bash
npm update -g vibe-kanban
```

**To check version**:
```bash
npm list -g vibe-kanban
```

---

### Method 3: Build from Source

**Best for**: Developers who want to contribute, customize, or run the latest unreleased features.

#### Step 1: Install Prerequisites

**Install Rust** (via rustup):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Install Node.js** (>= 18):
- Download from [nodejs.org](https://nodejs.org/)
- Or use a version manager like [nvm](https://github.com/nvm-sh/nvm)

**Install pnpm** (>= 8):
```bash
npm install -g pnpm
```

#### Step 2: Clone the Repository

```bash
git clone https://github.com/BloopAI/vibe-kanban.git
cd vibe-kanban
```

#### Step 3: Install Dependencies

```bash
pnpm install
```

#### Step 4: Install Development Tools (Optional)

```bash
cargo install cargo-watch
cargo install sqlx-cli
```

#### Step 5: Build the Project

**For development** (with hot-reload):
```bash
pnpm run dev
```

This will:
- Start the backend server with auto-reload
- Start the frontend dev server
- Copy a blank database from `dev_assets_seed` folder
- Open the application in your browser

**For production build**:

On macOS/Linux:
```bash
./local-build.sh
```

On Windows:
```bash
bash local-build.sh
```

#### Step 6: Test the Build

```bash
cd npx-cli
node bin/cli.js
```

---

### Method 4: Docker Container

**Best for**: Users who prefer containerized deployments or need isolated environments.

#### Using Docker Compose (with Remote Service)

```bash
cd crates/remote
docker compose --env-file .env.remote up --build
```

**Prerequisites**:
1. Create `.env.remote` file with required environment variables:
   ```env
   JWT_SECRET=your-secret-key
   GITHUB_CLIENT_ID=your-github-oauth-client-id
   GITHUB_CLIENT_SECRET=your-github-oauth-secret
   ```

2. See [crates/remote/README.md](crates/remote/README.md) for detailed remote deployment setup.

#### Custom Docker Setup

You can also build a custom Docker image using the included `Dockerfile`:

```bash
docker build -t vibe-kanban .
docker run -p 3000:3000 vibe-kanban
```

---

## Platform-Specific Instructions

### Windows

#### Prerequisites
1. Install **Git for Windows**: [git-scm.com](https://git-scm.com/download/win)
2. Install **Node.js**: [nodejs.org](https://nodejs.org/)

#### Installation

**Option 1: NPX (Recommended)**
```bash
npx vibe-kanban
```

No additional setup needed for NPX users!

**Option 2: Build from Source**

**IMPORTANT**: Building from source on Windows requires the Microsoft C++ Build Tools.

**Step 1: Install Microsoft C++ Build Tools**

Choose one option:

**Option A: Build Tools for Visual Studio (Recommended - Smaller)**
1. Download from: https://visualstudio.microsoft.com/downloads/
2. Scroll to "Tools for Visual Studio"
3. Download "Build Tools for Visual Studio 2022"
4. Run installer and select: **"Desktop development with C++"** workload
5. Restart your terminal after installation

**Option B: Visual Studio Community (Full IDE)**
1. Download from: https://visualstudio.microsoft.com/vs/community/
2. During installation, select: **"Desktop development with C++"** workload
3. Restart your terminal after installation

**Verify Build Tools**:
```powershell
# Check if link.exe is available
where link.exe
# Should show path to Microsoft Visual Studio linker
```

**Step 2: Install Rust**
```powershell
# Download and run rustup-init.exe from:
# https://rustup.rs/

# Or use winget:
winget install Rustlang.Rustup

# Restart terminal, then verify:
rustc --version
cargo --version
```

**Step 3: Install pnpm**
```powershell
npm install -g pnpm
```

**Step 4: Clone and build**
```powershell
git clone https://github.com/BloopAI/vibe-kanban.git
cd vibe-kanban
pnpm install
bash local-build.sh  # Use Git Bash or PowerShell
```

#### Windows-Specific Notes
- **Build Tools are required** for Rust compilation on Windows (see Step 1 above)
- If using `HOST=0.0.0.0`, set `MCP_HOST=127.0.0.1` to avoid connection issues
- Use PowerShell or Git Bash for running bash scripts
- Ensure Windows Defender doesn't block the executable
- If you see `error: linker 'link.exe' not found`, you need to install the C++ Build Tools (Step 1)

---

### macOS

#### Prerequisites
1. Install **Homebrew** (optional but recommended):
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

2. Install **Git** (usually pre-installed):
   ```bash
   git --version
   ```

3. Install **Node.js**:
   ```bash
   brew install node
   ```

#### Installation

**Option 1: NPX (Recommended)**
```bash
npx vibe-kanban
```

**Option 2: Build from Source**

For **Intel Macs** (x64):
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install pnpm
npm install -g pnpm

# Clone and build
git clone https://github.com/BloopAI/vibe-kanban.git
cd vibe-kanban
pnpm install
./local-build.sh
```

For **Apple Silicon** (ARM64/M1/M2/M3):
```bash
# Same as above - the build script auto-detects architecture
./local-build.sh
```

#### macOS-Specific Notes
- Binaries are built for both Intel (x64) and Apple Silicon (arm64)
- If you get "unidentified developer" warning, go to System Preferences → Security & Privacy and allow the app
- On first run, macOS may ask for permissions to access files

---

### Linux

#### Prerequisites

**Debian/Ubuntu**:
```bash
sudo apt update
sudo apt install -y git curl build-essential
```

**Fedora/RHEL**:
```bash
sudo dnf install git curl gcc
```

**Arch Linux**:
```bash
sudo pacman -S git curl base-devel
```

**Install Node.js**:
```bash
# Using Node Version Manager (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18

# Or using package manager
# Debian/Ubuntu:
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install -y nodejs
```

#### Installation

**Option 1: NPX (Recommended)**
```bash
npx vibe-kanban
```

**Option 2: Build from Source**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install pnpm
npm install -g pnpm

# Clone and build
git clone https://github.com/BloopAI/vibe-kanban.git
cd vibe-kanban
pnpm install
./local-build.sh
```

#### Linux-Specific Notes
- Ensure you have `libssl-dev` installed for HTTPS support
- Some distributions may require additional dependencies for WebSocket support
- If running as a service, consider using systemd (see [Self-Hosted Deployment](#self-hosted-deployment))

---

## Post-Installation Setup

### 1. First Run

After installation, run Vibe Kanban:

```bash
npx vibe-kanban
# or
vibe-kanban  # if installed globally
```

The application will:
1. Start the backend server
2. Launch the frontend in your default browser
3. Create a local SQLite database
4. Display the welcome screen

### 2. Add Your First Project

1. Click **"Add Project"** or **"Browse Filesystem"**
2. Select an existing git repository or create a new one
3. Configure project settings (optional):
   - Setup scripts
   - Development scripts
   - Default agent preferences

### 3. Configure AI Agents

**Authenticate with your preferred coding agents** before using Vibe Kanban:

**Claude Code**:
```bash
# Authentication handled by Claude CLI
claude-code auth
```

**Other Agents**: Refer to each agent's documentation for authentication steps.

**Supported agents**: Claude Code, Gemini CLI, OpenAI Codex, Cursor Agent CLI, Amp, OpenCode, and more. See the [full list](https://vibekanban.com/docs).

---

## Configuration

### Environment Variables

Configure Vibe Kanban using environment variables:

#### Build-Time Variables
Set these when building from source:

| Variable | Default | Description |
|----------|---------|-------------|
| `POSTHOG_API_KEY` | Empty | PostHog analytics API key (disables analytics if empty) |
| `POSTHOG_API_ENDPOINT` | Empty | PostHog analytics endpoint |

#### Runtime Variables
Set these when running the application:

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | Auto-assign | Server port (**Production**). In **Dev**: Frontend port (backend uses PORT+1) |
| `BACKEND_PORT` | `0` (auto) | Backend server port (dev mode only, overrides PORT+1) |
| `FRONTEND_PORT` | `3000` | Frontend dev server port (dev mode only, overrides PORT) |
| `HOST` | `127.0.0.1` | Backend server host |
| `MCP_HOST` | Value of `HOST` | MCP server connection host (use `127.0.0.1` when `HOST=0.0.0.0` on Windows) |
| `MCP_PORT` | Value of `BACKEND_PORT` | MCP server connection port |
| `DISABLE_WORKTREE_ORPHAN_CLEANUP` | Not set | Disable git worktree cleanup (for debugging) |
| `VK_ALLOWED_ORIGINS` | Not set | Comma-separated allowed origins for CORS (required for reverse proxy/custom domain) |

#### Example: Custom Port

```bash
PORT=8080 npx vibe-kanban
```

#### Example: Development with Custom Ports

```bash
FRONTEND_PORT=3001 BACKEND_PORT=3002 pnpm run dev
```

### Global Settings

Access settings in the Vibe Kanban UI:

1. Click the **Settings** icon (top-right)
2. Configure:
   - **Editor Integration**: Choose your preferred code editor (VSCode, Cursor, Windsurf, IntelliJ, Zed)
   - **Sound Notifications**: Enable/disable completion sounds
   - **Default Agent**: Set your preferred AI coding agent
   - **GitHub CLI**: Configure for creating pull requests
   - **Remote SSH**: Configure remote server access (see [Remote Deployment](#remote-deployment))

### MCP Configuration

Centralize Model Context Protocol (MCP) configuration for your coding agents:

1. Navigate to **Settings → MCP Configuration**
2. Define global MCP servers and tools
3. Agents will automatically use these configurations

---

## Self-Hosted Deployment

### Running Behind a Reverse Proxy

When deploying Vibe Kanban behind a reverse proxy (nginx, Caddy, Traefik) or on a custom domain, you **must** set the `VK_ALLOWED_ORIGINS` environment variable.

#### Example: Nginx

```nginx
server {
    listen 443 ssl;
    server_name vk.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

**Environment configuration**:
```bash
VK_ALLOWED_ORIGINS=https://vk.example.com npx vibe-kanban
```

#### Example: Caddy

```caddy
vk.example.com {
    reverse_proxy localhost:3000
}
```

**Environment configuration**:
```bash
VK_ALLOWED_ORIGINS=https://vk.example.com npx vibe-kanban
```

#### Multiple Origins

```bash
VK_ALLOWED_ORIGINS=https://vk.example.com,https://vk-staging.example.com npx vibe-kanban
```

### Running as a System Service (Linux)

Create a systemd service file:

```bash
sudo nano /etc/systemd/system/vibe-kanban.service
```

**Service configuration**:
```ini
[Unit]
Description=Vibe Kanban AI Orchestration Platform
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/home/your-username/vibe-kanban
ExecStart=/usr/bin/npx vibe-kanban
Restart=on-failure
Environment="PORT=3000"
Environment="VK_ALLOWED_ORIGINS=https://vk.example.com"

[Install]
WantedBy=multi-user.target
```

**Enable and start**:
```bash
sudo systemctl daemon-reload
sudo systemctl enable vibe-kanban
sudo systemctl start vibe-kanban
sudo systemctl status vibe-kanban
```

---

## Remote Deployment

Deploy Vibe Kanban on a remote server and access projects via SSH from your local editor.

### Setup Steps

#### 1. Deploy on Remote Server

Install and run Vibe Kanban on your remote server:

```bash
# On remote server
npx vibe-kanban
```

Or use systemd service (see [Self-Hosted Deployment](#self-hosted-deployment)).

#### 2. Expose Web UI

Use a tunnel service to access the web interface:

**Option A: Cloudflare Tunnel**
```bash
cloudflared tunnel --url http://localhost:3000
```

**Option B: ngrok**
```bash
ngrok http 3000
```

**Option C: Self-hosted reverse proxy** (nginx, Caddy, etc.)

#### 3. Configure Remote SSH

In the Vibe Kanban UI:

1. Go to **Settings → Editor Integration**
2. Set **Remote SSH Host**: `your-server.com` (or IP address)
3. Set **Remote SSH User**: `your-username` (optional)

#### 4. Prerequisites

On your **local machine**:

1. **SSH access** to the remote server:
   ```bash
   ssh your-username@your-server.com
   ```

2. **SSH keys configured** (passwordless authentication):
   ```bash
   ssh-copy-id your-username@your-server.com
   ```

3. **VSCode Remote-SSH extension** installed:
   - Install from [marketplace](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-ssh)

#### 5. Usage

When configured, the **"Open in VSCode"** buttons will generate URLs like:

```
vscode://vscode-remote/ssh-remote+user@host/path/to/project
```

This opens your local VSCode and connects to the remote server automatically.

### Detailed Documentation

See the [Remote SSH Configuration docs](https://vibekanban.com/docs/configuration-customisation/global-settings#remote-ssh-configuration) for advanced setup.

---

## Verification

### Verify Installation

After installation, verify everything is working:

#### 1. Check Version
```bash
npx vibe-kanban --version
```

#### 2. Test Launch
```bash
npx vibe-kanban
```

Expected output:
```
🚀 Starting Vibe Kanban...
✅ Backend server started on http://127.0.0.1:XXXX
✅ Opening browser...
```

#### 3. Check Browser Access

The application should automatically open in your browser at:
```
http://localhost:XXXX
```

If not, manually navigate to the URL shown in the terminal.

#### 4. Verify Database

Check that the SQLite database was created:

```bash
# Location varies by platform
# macOS/Linux: ~/.vibe-kanban/vibe-kanban.db
# Windows: %APPDATA%\vibe-kanban\vibe-kanban.db
```

#### 5. Test Project Creation

1. Click **"Add Project"**
2. Browse to a git repository
3. Verify it appears in the project list

#### 6. Test Agent Execution

1. Create a task
2. Run with your configured AI agent
3. Check the execution logs

### Health Check Endpoint

When running, you can check the health of the backend:

```bash
curl http://localhost:XXXX/health
```

Expected response:
```json
{"status": "ok"}
```

---

## Troubleshooting

### Common Issues

#### Issue: "Command not found: npx"

**Solution**: Install Node.js (includes npx):
```bash
# Download from nodejs.org or use nvm
nvm install 18
```

#### Issue: "Port already in use"

**Solution**: Specify a different port:
```bash
PORT=8080 npx vibe-kanban
```

#### Issue: "Failed to download binary"

**Causes**:
- Network connectivity issues
- Firewall blocking downloads
- Unsupported platform

**Solution**:
1. Check your internet connection
2. Verify platform compatibility (see [System Requirements](#system-requirements))
3. Try building from source (see [Method 3](#method-3-build-from-source))

#### Issue: "Git repository not detected"

**Solution**:
1. Ensure the directory is a valid git repository:
   ```bash
   git status
   ```
2. If not, initialize it:
   ```bash
   git init
   ```

#### Issue: "Agent not authenticated"

**Solution**: Authenticate with your coding agent before using Vibe Kanban. For Claude Code:
```bash
claude-code auth
```

#### Issue: "403 Forbidden" when using reverse proxy

**Solution**: Set `VK_ALLOWED_ORIGINS`:
```bash
VK_ALLOWED_ORIGINS=https://your-domain.com npx vibe-kanban
```

#### Issue: Windows - "MCP connection failed"

**Solution**: When using `HOST=0.0.0.0`, explicitly set:
```bash
MCP_HOST=127.0.0.1 npx vibe-kanban
```

#### Issue: macOS - "Unidentified developer" warning

**Solution**:
1. Go to **System Preferences → Security & Privacy**
2. Click **"Allow Anyway"** next to the blocked app
3. Re-run the application

#### Issue: Build fails with "cargo not found"

**Solution**: Install Rust toolchain:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Issue: Frontend build fails

**Solution**:
1. Clear node_modules and reinstall:
   ```bash
   rm -rf node_modules frontend/node_modules
   pnpm install
   ```
2. Verify pnpm version:
   ```bash
   pnpm --version  # Should be >= 8
   ```

### Getting Help

If you encounter issues not covered here:

1. **Search existing issues**: [GitHub Issues](https://github.com/BloopAI/vibe-kanban/issues)
2. **Ask a question**: [GitHub Discussions](https://github.com/BloopAI/vibe-kanban/discussions)
3. **Join Discord**: [Community Discord](https://discord.gg/AC4nwVtJM3)
4. **Report a bug**: [Open an Issue](https://github.com/BloopAI/vibe-kanban/issues/new)

### Debug Mode

Enable debug logging for troubleshooting:

```bash
RUST_LOG=debug npx vibe-kanban
```

This will output detailed logs to help diagnose issues.

---

## Uninstallation

### NPX Method

No uninstallation needed - NPX downloads are cached and can be cleared:

```bash
# Clear npm cache
npm cache clean --force
```

### Global Installation

```bash
npm uninstall -g vibe-kanban
```

### Remove Data

To remove all Vibe Kanban data:

**macOS/Linux**:
```bash
rm -rf ~/.vibe-kanban
```

**Windows**:
```bash
rd /s /q %APPDATA%\vibe-kanban
```

### Built from Source

Simply delete the cloned repository:

```bash
rm -rf /path/to/vibe-kanban
```

---

## Next Steps

After installation:

1. **Read the documentation**: [vibekanban.com/docs](https://vibekanban.com/docs)
2. **Watch the overview video**: [YouTube](https://youtu.be/TFT3KnZOOAk)
3. **Join the community**: [Discord](https://discord.gg/AC4nwVtJM3)
4. **Explore features**: Create your first project and task!

---

## Support

- **Documentation**: [vibekanban.com/docs](https://vibekanban.com/docs)
- **GitHub Issues**: [Report bugs](https://github.com/BloopAI/vibe-kanban/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/BloopAI/vibe-kanban/discussions)
- **Discord**: [Community chat](https://discord.gg/AC4nwVtJM3)

---

<p align="center">
  <strong>Ready to supercharge your development workflow?</strong><br>
  <code>npx vibe-kanban</code>
</p>

<p align="center">
  Start managing your projects with the power of AI coding agents today!
</p>
