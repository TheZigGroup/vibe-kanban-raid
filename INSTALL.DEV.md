# Developer Installation Guide

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
  <strong>Developer Setup & Contribution Guide</strong><br>
  Build, develop, and extend Vibe Kanban
</p>

---

## Table of Contents

- [Welcome Contributors](#welcome-contributors)
- [Prerequisites](#prerequisites)
- [Quick Start for Development](#quick-start-for-development)
- [Project Architecture](#project-architecture)
- [Development Workflow](#development-workflow)
- [Building from Source](#building-from-source)
- [Testing](#testing)
- [Code Quality](#code-quality)
- [Database Development](#database-development)
- [Frontend Development](#frontend-development)
- [Backend Development](#backend-development)
- [Remote Service Development](#remote-service-development)
- [Contributing Guidelines](#contributing-guidelines)
- [Development Tips & Tricks](#development-tips--tricks)
- [Troubleshooting](#troubleshooting)
- [Resources](#resources)

---

## Welcome Contributors

Thank you for your interest in contributing to Vibe Kanban! This guide will help you set up your development environment and understand our development workflow.

### Before You Start

We prefer that ideas and changes are first raised with the core team via:
- [GitHub Discussions](https://github.com/BloopAI/vibe-kanban/discussions)
- [Discord](https://discord.gg/AC4nwVtJM3)

**Please do not open PRs without first discussing your proposal with the team.** This ensures alignment with the existing roadmap and prevents duplicate work.

### Code of Conduct

All contributors must adhere to our [Code of Conduct](CODE-OF-CONDUCT.md). We are committed to providing a welcoming and inclusive environment for everyone.

---

## Prerequisites

### Required Tools

#### Windows-Specific Prerequisites

**IMPORTANT**: Before installing Rust on Windows, you need the Microsoft C++ Build Tools:

1. **Microsoft C++ Build Tools** (Required for Rust compilation)

   **Option A: Build Tools for Visual Studio (Recommended - Smaller)**
   - Download: https://visualstudio.microsoft.com/downloads/
   - Scroll to "Tools for Visual Studio"
   - Download "Build Tools for Visual Studio 2022"
   - Run installer and select: **"Desktop development with C++"** workload

   **Option B: Visual Studio Community (Full IDE)**
   - Download: https://visualstudio.microsoft.com/vs/community/
   - During installation, select: **"Desktop development with C++"** workload

   After installation, restart your terminal/PowerShell.

2. **Verify Build Tools Installation**
   ```powershell
   # Check if link.exe is available
   where link.exe
   # Should show path to Microsoft Visual Studio linker
   ```

#### All Platforms

1. **Rust** (latest stable)

   **Windows**:
   ```powershell
   # Download and run rustup-init.exe from:
   # https://rustup.rs/

   # Or use winget:
   winget install Rustlang.Rustup

   # Restart terminal, then verify:
   rustc --version
   cargo --version
   ```

   **macOS/Linux**:
   ```bash
   # Install via rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env

   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Node.js** (>= 18.0.0)

   **Windows**:
   ```powershell
   # Download from nodejs.org or use winget:
   winget install OpenJS.NodeJS.LTS

   # Verify installation
   node --version  # Should be >= 18
   ```

   **macOS/Linux**:
   ```bash
   # Using nvm (recommended)
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 18
   nvm use 18

   # Verify installation
   node --version  # Should be >= 18
   ```

3. **pnpm** (>= 8.0.0)
   ```bash
   npm install -g pnpm

   # Verify installation
   pnpm --version  # Should be >= 8
   ```

4. **Git** (>= 2.0.0)

   **Windows**:
   ```powershell
   # Download from git-scm.com or use winget:
   winget install Git.Git

   # Verify installation
   git --version
   ```

   **macOS/Linux**:
   ```bash
   git --version
   # Usually pre-installed, or install via package manager
   ```

### Development Tools (Recommended)

```bash
# Auto-reload for Rust development
cargo install cargo-watch

# Database migrations CLI
cargo install sqlx-cli

# Type generation binary
# (built automatically from source)
```

### Optional Tools

- **Docker** - For remote service development
- **VSCode** - Recommended IDE with Rust Analyzer extension
- **rust-analyzer** - LSP for Rust development

---

## Quick Start for Development

### 1. Clone the Repository

```bash
git clone https://github.com/BloopAI/vibe-kanban.git
cd vibe-kanban
```

### 2. Install Dependencies

```bash
pnpm install
```

This will install dependencies for:
- Root workspace
- Frontend (`frontend/`)
- NPX CLI (`npx-cli/`)
- Remote frontend (`remote-frontend/`)

### 3. Start Development Server

**macOS/Linux**:
```bash
pnpm run dev
```

**Windows** (use PowerShell):
```powershell
.\dev.ps1
```

**Note**: Windows users should use the `dev.ps1` PowerShell script instead of `pnpm run dev` because the package.json dev script uses Unix-style commands that don't work in Windows PowerShell.

This command will:
- Find available ports for frontend and backend
- Copy blank database from `dev_assets_seed/` to `dev_assets/`
- Start backend server with hot-reload (cargo-watch)
- Start frontend dev server (Vite)
- Open your browser automatically

**Access the app at**: `http://localhost:3000` (or the port shown in terminal)

### 4. Verify Setup

1. Backend should show logs in the terminal
2. Frontend should open in browser
3. Try creating a project and task
4. Check the database at `dev_assets/vibe-kanban.db`

---

## Project Architecture

### Repository Structure

```
vibe-kanban/
├── crates/                      # Rust backend modules
│   ├── server/                  # Main HTTP/WebSocket server (Axum)
│   ├── db/                      # Database layer (SQLite, SQLx)
│   ├── executors/               # Task execution engine
│   ├── services/                # Business logic services
│   ├── local-deployment/        # Local deployment handling
│   ├── deployment/              # Deployment abstractions
│   ├── remote/                  # Remote deployment support
│   ├── review/                  # Code review CLI tool
│   └── utils/                   # Shared utilities
│
├── frontend/                    # React/TypeScript frontend
│   ├── src/
│   │   ├── components/          # React components
│   │   ├── pages/               # Page components
│   │   ├── hooks/               # Custom React hooks
│   │   ├── stores/              # Zustand state management
│   │   ├── api/                 # API client layer
│   │   └── types/               # TypeScript type definitions
│   └── public/                  # Static assets
│
├── npx-cli/                     # CLI entry point for npx
│   ├── bin/cli.js               # Entry script
│   └── dist/                    # Platform-specific binaries
│
├── remote-frontend/             # Remote deployment frontend
├── docs/                        # Mintlify documentation
├── scripts/                     # Build and development scripts
├── dev_assets_seed/             # Seed database for development
├── dev_assets/                  # Active development database
├── Cargo.toml                   # Rust workspace configuration
├── package.json                 # Root workspace scripts
├── Dockerfile                   # Container configuration
└── local-build.sh               # Local build script
```

### Rust Workspace Structure

The backend is organized as a Cargo workspace with the following crates:

| Crate | Purpose |
|-------|---------|
| `server` | Main HTTP/WebSocket server using Axum |
| `db` | Database layer with SQLx and migrations |
| `executors` | Task execution engine for running AI agents |
| `services` | Business logic services (projects, tasks, etc.) |
| `local-deployment` | Local deployment handling |
| `deployment` | Deployment abstraction layer |
| `remote` | Remote deployment support with Electric Sync |
| `review` | CLI tool for code review |
| `utils` | Shared utility functions |

### Tech Stack Deep Dive

#### Backend (Rust)
- **Runtime**: Tokio (async/await)
- **Web Framework**: Axum (tower-http middleware)
- **Database**: SQLite with SQLx (compile-time checked queries)
- **Serialization**: Serde/serde_json
- **Git Integration**: git2 (libgit2 bindings)
- **Type Generation**: ts-rs (Rust → TypeScript)
- **Error Handling**: anyhow, thiserror
- **Logging**: tracing, tracing-subscriber
- **TLS**: rustls with aws-lc-rs

#### Frontend (TypeScript/React)
- **Framework**: React 18.2
- **Build Tool**: Vite 6.3.5
- **State Management**: Zustand
- **Data Fetching**: TanStack React Query
- **Forms**: TanStack React Form
- **Database**: TanStack Electric DB Collection, wa-sqlite
- **Styling**: Tailwind CSS
- **UI Components**: Radix UI
- **Code Editor**: CodeMirror
- **Rich Text**: Lexical
- **Terminal**: xterm.js
- **Diff Viewer**: @git-diff-view/react
- **Drag & Drop**: dnd-kit

---

## Development Workflow

### Hot-Reload Development

The `pnpm run dev` command provides hot-reload for both frontend and backend:

**Frontend**: Vite dev server with HMR (Hot Module Replacement)
- Changes to React components reload instantly
- CSS changes apply without full page reload

**Backend**: `cargo-watch` monitors Rust files
- Automatically recompiles and restarts server on changes
- Watch paths: `crates/` directory

### Port Management

Development ports are automatically managed:

1. **Auto-allocation**: Script finds free ports starting from 3000
2. **Port persistence**: Saved to `.dev-ports.json` for reuse
3. **Custom ports**: Override with environment variables

```bash
# Use specific ports
FRONTEND_PORT=3001 BACKEND_PORT=3002 pnpm run dev

# Or set base PORT (backend will use PORT+1)
PORT=8080 pnpm run dev
```

### Environment Variables

Development environment variables:

```bash
# Required for development
FRONTEND_PORT=3000           # Frontend dev server port
BACKEND_PORT=3001            # Backend server port
VK_ALLOWED_ORIGINS=http://localhost:3000  # CORS allowed origins
RUST_LOG=debug               # Rust logging level
DISABLE_WORKTREE_ORPHAN_CLEANUP=1  # Disable cleanup for debugging

# Optional
VITE_VK_SHARED_API_BASE=     # Shared API base URL
POSTHOG_API_KEY=             # Analytics (leave empty to disable)
POSTHOG_API_ENDPOINT=        # Analytics endpoint
```

### Development Database

- **Seed database**: `dev_assets_seed/vibe-kanban.db`
- **Active database**: `dev_assets/vibe-kanban.db`

On first run, the seed database is copied to `dev_assets/`. To reset:

```bash
rm -rf dev_assets/
pnpm run dev  # Will copy seed again
```

---

## Building from Source

### Development Build

For local testing with optimizations disabled (faster compilation):

```bash
cargo build
```

### Release Build

For production-optimized binaries:

```bash
./local-build.sh
```

This script:
1. Detects your OS and architecture (linux-x64, macos-x64, macos-arm64, windows-x64)
2. Cleans previous builds
3. Builds frontend with TypeScript compilation and Vite
4. Builds Rust binaries with `--release` flag:
   - `server` - Main Vibe Kanban server
   - `mcp_task_server` - MCP server for task management
   - `review` - Code review CLI tool
5. Creates distribution packages in `npx-cli/dist/[platform]/`

**Custom build target directory**:

```bash
CARGO_TARGET_DIR=/path/to/target ./local-build.sh
```

### Testing the Build

```bash
cd npx-cli
node bin/cli.js
```

### Platform-Specific Builds

**macOS** (Intel):
```bash
./local-build.sh  # Detects x64 automatically
```

**macOS** (Apple Silicon):
```bash
./local-build.sh  # Detects arm64 automatically
```

**Linux**:
```bash
./local-build.sh
```

**Windows**:
```bash
bash local-build.sh  # Use Git Bash or WSL
```

---

## Testing

### Frontend Tests

```bash
cd frontend
pnpm run test
```

### Backend Tests

```bash
cargo test
```

Run tests for a specific crate:

```bash
cargo test -p server
cargo test -p db
```

### Integration Tests

```bash
# Test the NPX package
./test-npm-package.sh
```

### End-to-End Testing

Currently, e2e tests are manual. Test workflow:

1. Start dev server: `pnpm run dev`
2. Create a project
3. Create a task
4. Execute with an AI agent
5. Review changes
6. Merge or reject

---

## Code Quality

### Linting

**Frontend**:
```bash
cd frontend
pnpm run lint           # Check for issues
pnpm run lint:fix       # Auto-fix issues
pnpm run lint:i18n      # Check internationalization
```

**Backend**:
```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Formatting

**Frontend**:
```bash
cd frontend
pnpm run format         # Format all files
pnpm run format:check   # Check formatting
```

**Backend**:
```bash
cargo fmt --all         # Format all Rust code
cargo fmt --all -- --check  # Check formatting
```

### Type Checking

**Frontend**:
```bash
cd frontend
pnpm run check  # TypeScript type checking
```

**Backend**:
```bash
cargo check  # Fast compilation check
```

### Combined Quality Check

Run all quality checks:

```bash
pnpm run check   # Frontend + Backend type checking
pnpm run lint    # Frontend + Backend linting
pnpm run format  # Frontend + Backend formatting
```

---

## Database Development

### Schema and Migrations

Database schema is managed with SQLx migrations in `crates/db/migrations/`.

### Creating a Migration

```bash
cd crates/db
sqlx migrate add <migration_name>
```

This creates a new migration file in `migrations/`. Edit the file to add your SQL.

### Running Migrations

Migrations run automatically when the server starts in development mode.

**Manual migration**:
```bash
sqlx migrate run --database-url sqlite:dev_assets/vibe-kanban.db
```

### Reverting Migrations

```bash
sqlx migrate revert --database-url sqlite:dev_assets/vibe-kanban.db
```

### Preparing SQLx Data

SQLx uses compile-time query verification. After schema changes:

```bash
pnpm run prepare-db
```

This updates `.sqlx/` directory with query metadata.

**Check mode** (CI):
```bash
pnpm run prepare-db:check
```

### Database Tools

**SQLite Browser**:
```bash
# Install sqlite3
sqlite3 dev_assets/vibe-kanban.db

# Or use GUI tools like DB Browser for SQLite
```

**Schema inspection**:
```sql
.schema
.tables
.schema projects
```

---

## Frontend Development

### Starting Frontend Only

```bash
cd frontend
pnpm run dev
```

By default connects to backend at `http://localhost:3001`.

### Building Frontend

```bash
cd frontend
pnpm run build
```

Build output goes to `frontend/dist/`.

### Component Development

**Location**: `frontend/src/components/`

**Best practices**:
- Use TypeScript for all components
- Follow existing component structure
- Use Radix UI for accessible primitives
- Style with Tailwind CSS utility classes
- Use Zustand for global state
- Use TanStack Query for server state

### State Management

**Global State** (Zustand):
```typescript
// frontend/src/stores/useStore.ts
import { create } from 'zustand';

const useStore = create((set) => ({
  projects: [],
  setProjects: (projects) => set({ projects }),
}));
```

**Server State** (TanStack Query):
```typescript
import { useQuery } from '@tanstack/react-query';

const { data, isLoading } = useQuery({
  queryKey: ['projects'],
  queryFn: fetchProjects,
});
```

### Adding New Routes

Edit `frontend/src/App.tsx`:

```typescript
import { Route } from 'react-router-dom';

<Route path="/new-feature" element={<NewFeature />} />
```

### Type Generation

Types are automatically generated from Rust structs using `ts-rs`:

```bash
pnpm run generate-types        # Generate types
pnpm run generate-types:check  # Check types are up-to-date
```

Generated types appear in `frontend/src/types/generated/`.

### Internationalization (i18n)

**Adding translations**:

1. Add key to `frontend/src/locales/en/translation.json`
2. Use in component:
   ```typescript
   import { useTranslation } from 'react-i18next';

   const { t } = useTranslation();
   return <div>{t('key.path')}</div>;
   ```

3. Check i18n usage:
   ```bash
   pnpm run lint:i18n
   ```

---

## Backend Development

### Crate Organization

When adding features, identify the appropriate crate:

- **API endpoints** → `crates/server`
- **Database queries** → `crates/db`
- **Business logic** → `crates/services`
- **Task execution** → `crates/executors`
- **Git operations** → `crates/services` or `crates/executors`
- **Utilities** → `crates/utils`

### Adding a New Endpoint

**1. Define types** in `crates/server/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewFeatureRequest {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewFeatureResponse {
    pub id: i64,
    pub name: String,
}
```

**2. Add handler** in `crates/server/src/routes/`:

```rust
use axum::{extract::State, Json};
use crate::types::{NewFeatureRequest, NewFeatureResponse};

pub async fn create_feature(
    State(state): State<AppState>,
    Json(payload): Json<NewFeatureRequest>,
) -> Result<Json<NewFeatureResponse>, AppError> {
    // Implementation
    Ok(Json(response))
}
```

**3. Register route** in `crates/server/src/main.rs`:

```rust
.route("/api/feature", post(routes::create_feature))
```

**4. Generate TypeScript types**:

```bash
pnpm run generate-types
```

### Database Queries

Use SQLx for type-safe queries:

```rust
use sqlx::{query, query_as};

// Simple query
let rows = query!("SELECT * FROM projects WHERE id = ?", id)
    .fetch_all(&pool)
    .await?;

// Query with mapping
#[derive(Debug, sqlx::FromRow)]
struct Project {
    id: i64,
    name: String,
}

let projects = query_as!(Project, "SELECT id, name FROM projects")
    .fetch_all(&pool)
    .await?;
```

### Error Handling

Use `anyhow` for error propagation:

```rust
use anyhow::{Result, Context};

fn my_function() -> Result<String> {
    let data = read_file("path")?;
    Ok(data)
}

// With context
fn with_context() -> Result<()> {
    do_something()
        .context("Failed to do something")?;
    Ok(())
}
```

For custom errors, use `thiserror`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input")]
    InvalidInput,
}
```

### Logging

Use `tracing` for structured logging:

```rust
use tracing::{info, warn, error, debug, trace};

info!("Server starting on port {}", port);
warn!("Deprecated feature used");
error!("Failed to connect: {}", err);
debug!("Processing request: {:?}", request);
```

### WebSocket Support

WebSocket handlers are in `crates/server/src/websocket/`:

```rust
use axum::extract::ws::{WebSocket, Message};

async fn handle_socket(socket: WebSocket) {
    // Handle WebSocket connection
}
```

---

## Remote Service Development

### Prerequisites

1. **Docker** and **Docker Compose**
2. Create `.env.remote` in `crates/remote/`:

```env
JWT_SECRET=your-secret-key-here
GITHUB_CLIENT_ID=your-github-oauth-client-id
GITHUB_CLIENT_SECRET=your-github-oauth-secret
```

### Starting Remote Stack

```bash
pnpm run remote:dev
```

This starts:
- PostgreSQL database
- Electric Sync service
- Remote API server
- Frontend connected to remote backend

### Remote Database

Prepare remote database:

```bash
pnpm run remote:prepare-db
pnpm run remote:prepare-db:check  # Check mode
```

### Remote Type Generation

```bash
pnpm run remote:generate-types
pnpm run remote:generate-types:check  # Check mode
```

### Remote Architecture

The remote service enables:
- Multi-user collaboration
- Real-time sync via Electric
- OAuth authentication (GitHub)
- Project/workspace sharing

See [crates/remote/README.md](crates/remote/README.md) for details.

---

## Contributing Guidelines

### Discussion First

Before starting work:

1. **Check existing issues/discussions**
2. **Open a discussion** on GitHub Discussions or Discord
3. **Wait for approval** from maintainers
4. **Get assigned** to the issue
5. **Start coding**

### Branch Naming

```
feature/description
fix/description
docs/description
refactor/description
```

### Commit Messages

Follow conventional commits:

```
feat: add new feature
fix: resolve bug in component
docs: update installation guide
refactor: simplify service layer
test: add integration tests
chore: update dependencies
```

### Pull Request Process

1. **Create feature branch** from `main`
2. **Make changes** with tests
3. **Run quality checks**:
   ```bash
   pnpm run check
   pnpm run lint
   pnpm run format
   pnpm run generate-types:check
   pnpm run prepare-db:check
   ```
4. **Commit and push**
5. **Open PR** with description:
   - What changed
   - Why it changed
   - Testing done
   - Screenshots (if UI changes)
6. **Wait for review**
7. **Address feedback**
8. **Merge** (maintainers will merge)

### PR Checklist

- [ ] Discussed with maintainers first
- [ ] Code follows existing style
- [ ] Tests added/updated
- [ ] Types generated (`pnpm run generate-types:check`)
- [ ] Database prepared (`pnpm run prepare-db:check`)
- [ ] Lint passing (`pnpm run lint`)
- [ ] Format passing (`pnpm run format:check`)
- [ ] Documentation updated
- [ ] Changelog updated (if applicable)

---

## Development Tips & Tricks

### Fast Iteration

**Frontend only changes**:
```bash
# Skip backend restart
cd frontend && pnpm run dev
```

**Backend only changes**:
```bash
# Frontend will proxy to backend
cargo run --bin server
```

### Debug Mode

**Verbose logging**:
```bash
RUST_LOG=trace pnpm run dev
```

**Log specific module**:
```bash
RUST_LOG=server=debug,executors=trace pnpm run dev
```

### Database Reset

```bash
# Clear dev database
rm -rf dev_assets/

# Will copy seed on next run
pnpm run dev
```

### Port Conflicts

```bash
# Clear saved ports
node scripts/setup-dev-environment.js clear

# Use custom ports
FRONTEND_PORT=4000 BACKEND_PORT=4001 pnpm run dev
```

### Git Worktree Debugging

By default, orphaned worktrees are cleaned up. To debug:

```bash
DISABLE_WORKTREE_ORPHAN_CLEANUP=1 pnpm run dev
```

### Type Generation Workflow

After changing Rust types:

```bash
# 1. Generate types
pnpm run generate-types

# 2. Verify in frontend
cd frontend
pnpm run check

# 3. Update components as needed
```

### VSCode Setup

**Recommended extensions**:
- rust-analyzer (Rust LSP)
- ESLint (JavaScript/TypeScript linting)
- Prettier (Code formatting)
- Tailwind CSS IntelliSense
- SQLite Viewer

**Settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

### Performance Profiling

**Backend**:
```bash
# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin server
```

**Frontend**:
- Use React DevTools Profiler
- Chrome DevTools Performance tab

### Debugging WebSocket

Use browser DevTools:
1. Open Network tab
2. Filter by "WS"
3. Click WebSocket connection
4. View Messages tab

Or use CLI tools:
```bash
# wscat
npm install -g wscat
wscat -c ws://localhost:3001/ws
```

---

## Troubleshooting

### Issue: `error: linker 'link.exe' not found` (Windows)

**This is the most common Windows development issue.**

**Problem**: Rust needs the Microsoft C++ Build Tools to compile native code on Windows.

**Solution**: Install the Microsoft C++ Build Tools:

**Option A: Build Tools for Visual Studio (Recommended)**
1. Download: https://visualstudio.microsoft.com/downloads/
2. Scroll to "Tools for Visual Studio"
3. Download "Build Tools for Visual Studio 2022"
4. Run installer and select: **"Desktop development with C++"** workload
5. Restart your terminal

**Option B: Visual Studio Community**
1. Download: https://visualstudio.microsoft.com/vs/community/
2. Select: **"Desktop development with C++"** workload
3. Restart your terminal

**Verify**:
```powershell
where link.exe
# Should show path to Microsoft Visual Studio linker
```

After installing, retry:
```powershell
cargo install cargo-watch
cargo install sqlx-cli
```

### Issue: `'export' is not recognized` (Windows)

**Problem**: The `pnpm run dev` command uses Unix-style `export` commands that don't work in Windows PowerShell.

**Solution**: Use the PowerShell development script instead:

```powershell
.\dev.ps1
```

This script is specifically designed for Windows and will:
- Set environment variables correctly
- Start both backend and frontend servers
- Work natively in PowerShell

### Issue: `cargo-watch not found`

```bash
cargo install cargo-watch
```

**Note**: On Windows, ensure you have installed the C++ Build Tools first (see above).

### Issue: `sqlx-cli not found`

```bash
cargo install sqlx-cli
```

**Note**: On Windows, ensure you have installed the C++ Build Tools first (see above).

### Issue: Frontend can't connect to backend

1. Check backend is running: `curl http://localhost:3001/health`
2. Check CORS settings: `VK_ALLOWED_ORIGINS` should include frontend URL
3. Verify ports in `.dev-ports.json`

### Issue: Database migration errors

```bash
# Reset database
rm -rf dev_assets/
pnpm run dev
```

### Issue: Type generation fails

```bash
# Clean and rebuild
cargo clean
pnpm run generate-types
```

### Issue: Port already in use

```bash
# Clear saved ports
node scripts/setup-dev-environment.js clear

# Or use custom ports
FRONTEND_PORT=4000 BACKEND_PORT=4001 pnpm run dev
```

### Issue: Git worktree issues

```bash
# List worktrees
git worktree list

# Remove stale worktrees
git worktree prune
```

### Issue: Build fails on macOS

Ensure Xcode Command Line Tools are installed:
```bash
xcode-select --install
```

### Issue: SQLx compile-time verification fails

```bash
# Update SQLx cache
pnpm run prepare-db
```

---

## Resources

### Documentation

- **User Docs**: [vibekanban.com/docs](https://vibekanban.com/docs)
- **Mintlify Docs**: [docs/README.md](docs/README.md)
- **Remote Service**: [crates/remote/README.md](crates/remote/README.md)

### Community

- **GitHub Discussions**: [Discussions](https://github.com/BloopAI/vibe-kanban/discussions)
- **Discord**: [Join Server](https://discord.gg/AC4nwVtJM3)
- **GitHub Issues**: [Report Bugs](https://github.com/BloopAI/vibe-kanban/issues)

### Learning Resources

**Rust**:
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)

**TypeScript/React**:
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/handbook/intro.html)
- [Vite Guide](https://vitejs.dev/guide/)
- [TanStack Query](https://tanstack.com/query/latest)
- [Zustand](https://github.com/pmndrs/zustand)

**Tools**:
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [pnpm Documentation](https://pnpm.io/)
- [Git Documentation](https://git-scm.com/doc)

### Project-Specific

- **AGENTS.md**: Information about supported AI coding agents
- **CLAUDE.md**: Claude-specific development instructions
- **CODE-OF-CONDUCT.md**: Community guidelines
- **INSTALL.md**: End-user installation guide

---

## Next Steps

After setting up your development environment:

1. **Explore the codebase**: Browse `crates/` and `frontend/src/`
2. **Run the app**: Try creating projects and tasks
3. **Pick an issue**: Find beginner-friendly issues labeled "good first issue"
4. **Join the community**: Introduce yourself on Discord
5. **Start contributing**: Follow the [Contributing Guidelines](#contributing-guidelines)

---

## Getting Help

If you get stuck:

1. **Check this guide** for common issues
2. **Search existing issues** on GitHub
3. **Ask in Discord** - We're friendly!
4. **Open a discussion** for questions

**We're here to help!** Don't hesitate to reach out.

---

<p align="center">
  <strong>Happy coding! 🚀</strong><br>
  Thank you for contributing to Vibe Kanban
</p>

<p align="center">
  <a href="https://github.com/BloopAI/vibe-kanban">GitHub</a> •
  <a href="https://vibekanban.com">Website</a> •
  <a href="https://discord.gg/AC4nwVtJM3">Discord</a> •
  <a href="https://github.com/BloopAI/vibe-kanban/discussions">Discussions</a>
</p>
