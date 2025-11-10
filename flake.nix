{
  description = "Texler - LaTeX Editor with Web Frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        # Bun package manager (faster than npm)
        bun = pkgs.bun;

        # Development shell with all necessary tools
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            bun
            node2nix

            # Development tools
            git
            curl
            jq
            watchexec

            # For the LaTeX backend
            docker
            docker-compose
          ];

          shellHook = ''
            # Set up Bun environment
            export NODE_ENV=development
            export PATH="${bun}/bin:$PATH"

            # Colors for shell
            export PS1='\[\033[01;32m\][texler-web:\W]\$\[\033[00m\] '

            # Verify bun is available
            echo "🔍 Checking Bun installation..."
            which bun || echo "⚠️  Bun not found in PATH"
            bun --version && echo "✅ Bun version: $(bun --version)" || echo "❌ Bun check failed"

            # Welcome message
            echo ""
            echo "🚀 Welcome to Texler Web Development Environment (Bun + Vite + React 19)"
            echo ""
            echo "📦 Available commands:"
            echo "  bun dev           - Start Vite development server"
            echo "  bun run build     - Build for production (TypeScript + Vite)"
            echo "  bun run preview   - Preview production build"
            echo "  bun test          - Run tests with Vitest"
            echo "  bun run lint       - Lint TypeScript/React code"
            echo "  bunx              - Run any bun package globally"
            echo "  node2nix -i package.json -l bun.lockb -c"
            echo "                    - Generate Nix expressions from bun"
            echo ""
            echo "🐳 Docker commands (if needed):"
            echo "  docker-compose up - Start LaTeX compilation service"
            echo ""

            # Navigate to web app directory if not already there
            if [[ ! -f "package.json" ]]; then
              if [[ -f "../apps/web/package.json" ]]; then
                cd ../apps/web
              elif [[ -f "apps/web/package.json" ]]; then
                cd apps/web
              else
                echo "⚠️  Could not find apps/web/package.json"
                echo "Please navigate to the apps/web directory manually."
              fi
            fi

            # Install dependencies if node_modules doesn't exist or lock file is out of sync
            if [ ! -d "node_modules" ]; then
              echo "📥 Installing bun dependencies..."
              bun install
            fi
          '';
        };

        # Build the web application
        webApp = pkgs.stdenv.mkDerivation {
          name = "texler-web";
          src = ./apps/web;

          buildInputs = with pkgs; [ bun ];

          buildPhase = ''
            export HOME=$(mktemp -d)
            mkdir -p $HOME/.bun
            export BUN_INSTALL_CACHE_DIR=$HOME/.bun-cache

            # Install dependencies
            bun install --cache-dir $HOME/.bun-cache

            # Build the application
            bun run build
          '';

          installPhase = ''
            mkdir -p $out
            cp -r build/* $out/
          '';

          # Don't strip in build phase (causes issues with some JS files)
          dontStrip = true;

          # Cache node_modules for faster builds
          outputHashMode = "recursive";
          outputHashAlgo = "sha256";
          outputHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        };

        # Development server script
        devServer = pkgs.writeShellScriptBin "texler-web-dev" ''
          export NODE_ENV=development
          cd apps/web
          exec ${bun}/bin/bun run dev
        '';

        # Build script
        buildScript = pkgs.writeShellScriptBin "texler-web-build" ''
          export NODE_ENV=production
          cd apps/web
          exec ${bun}/bin/bun run build
        '';

      in
      {
        # Development shell
        devShells.default = devShell;

        # Packages
        packages = {
          default = webApp;
          texler-web = webApp;
          dev-server = devServer;
          build-script = buildScript;
        };

        # Applications
        apps = {
          dev = {
            type = "app";
            program = "${devServer}/bin/texler-web-dev";
            meta = {
              description = "Start Texler web development server";
            };
          };
          build = {
            type = "app";
            program = "${buildScript}/bin/texler-web-build";
            meta = {
              description = "Build Texler web app for production";
            };
          };
        };

        # Default app
        apps.default = {
          type = "app";
          program = "${devServer}/bin/texler-web-dev";
          meta = {
            description = "Start Texler web development server";
          };
        };
      }
    );
}