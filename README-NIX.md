# Texler - NixOS Development Guide

This guide helps you set up and develop Texler on NixOS using the provided Nix flake.

## Prerequisites

- NixOS with flakes enabled
- Docker (for the LaTeX compilation backend)
- Git

## Quick Start

### 1. Clone and Enter Development Shell

```bash
git clone <your-repo-url>
cd texler
nix develop
```

This will:
- Install Bun package manager and runtime
- Set up the Bun + Vite development environment
- Install web app dependencies automatically with bun.lockb
- Provide helpful development tools

### 2. Start the Full Development Stack

#### Option A: Start both web frontend and LaTeX backend
```bash
# Terminal 1: Start LaTeX compilation service
docker-compose up

# Terminal 2: Start Bun + Vite development server (in nix develop shell)
bun run dev
```

#### Option B: Use Nix apps directly
```bash
# Start Bun + Vite development server
nix run .#dev

# Or build for production
nix run .#build
```

### 3. Access the Application

- **Web Frontend**: http://localhost:3000
- **LaTeX API**: http://localhost:8081
- **LaTeX Health Check**: http://localhost:8081/health

## Available Commands

Inside the `nix develop` shell:

```bash
# Ultra-fast Bun + Vite development
bun run dev          # Start Vite development server (~3x faster than npm)
bun run build        # Build for production (TypeScript + Vite)
bun run preview      # Preview production build
bun test             # Run tests with Vitest
bun run lint         # Lint TypeScript/React code
bunx                # Run any Bun package globally

# Nix-specific commands
node2nix -i apps/web/package.json -l apps/web/bun.lockb -c
                  # Generate Nix expressions from bun dependencies
```

## Nix Flake Structure

The flake provides:

### Development Shell
- Node.js 18 with npm
- pnpm and yarn support
- node2nix for reproducible builds
- Docker integration for LaTeX backend
- Common development tools (git, curl, jq, watchexec)

### Packages
- `texler-web`: Production build of the web app
- `dev-server`: Development server script
- `build-script`: Build script for production

### Apps
- `dev`: Start development server
- `build`: Build for production

## Environment Configuration

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
```

Key environment variables:
- `REACT_APP_LATEX_API_URL`: LaTeX compilation service URL
- `GENERATE_SOURCEMAP`: Enable/disable source maps in development
- `PORT`: React development server port

## Production Build with Nix

### Option 1: Using the flake
```bash
nix build .#texler-web
```

### Option 2: Build result
The built web app will be in `./result` directory.

### Option 3: Direct Nix build
```bash
nix build
```

## Development Workflow

1. **Daily Development**:
   ```bash
   nix develop  # Enter dev shell
   npm start    # Start React dev server
   ```

2. **Testing**:
   ```bash
   nix develop
   npm test
   ```

3. **Production Build**:
   ```bash
   nix build .#texler-web
   ```

4. **Updating Dependencies**:
   ```bash
   nix develop
   cd apps/web
   npm install   # Update dependencies
   npm update    # Update packages
   ```

## Troubleshooting

### Node.js Version Issues
The flake uses Node.js 18. If you encounter version conflicts:
```bash
nix develop  # Always use the shell for consistent versions
```

### Permission Issues with Docker
Make sure your user is in the docker group:
```bash
sudo usermod -aG docker $USER
# Then log out and back in
```

### LaTeX Service Not Responding
Check if Docker is running and the container is healthy:
```bash
docker ps
docker logs texler-latex
```

### Build Issues
Clear Nix cache and retry:
```bash
nix store gc
nix develop
cd apps/web
rm -rf node_modules
npm ci
```

## Advanced Nix Usage

### Using node2nix for Reproducible Builds
For truly reproducible builds with Nix:

1. Generate Nix expressions:
   ```bash
   nix develop
   node2nix -i apps/web/package.json -l apps/web/package-lock.json -c
   ```

2. This will create `node-packages.nix` and related files for full reproducibility.

### Local Development Overrides
You can override the flake inputs for testing:
```bash
nix develop --override-input nixpkgs github:NixOS/nixpkgs/nixos-23.11
```

## Contributing

When contributing to Texler:
1. Make your changes
2. Test in the Nix development environment
3. Update dependencies if needed
4. Ensure both the web app and LaTeX service work together

Enjoy developing Texler with the reproducibility and power of Nix! 🚀