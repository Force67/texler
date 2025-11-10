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

        # Node.js version (React works well with Node 20)
        nodejs = pkgs.nodejs_20;

        # Development shell with all necessary tools
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            nodejs
            nodePackages.pnpm
            yarn
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
            # Set up Node.js environment
            export NODE_ENV=development
            export PATH="${nodejs}/bin:$PATH"

            # Enable corepack for modern npm/pnpm management
            export COREPACK_ENABLE_STRICT=0

            # Colors for shell
            export PS1='\[\033[01;32m\][texler-web:\W]\$\[\033[00m\] '

            # Welcome message
            echo ""
            echo "🚀 Welcome to Texler Web Development Environment"
            echo ""
            echo "📦 Available commands:"
            echo "  npm start         - Start React development server"
            echo "  npm run build     - Build for production"
            echo "  npm test          - Run tests"
            echo "  node2nix -i apps/web/package.json -l apps/web/package-lock.json -c"
            echo "                    - Generate Nix expressions from npm"
            echo ""
            echo "🐳 Docker commands (if needed):"
            echo "  docker-compose up - Start LaTeX compilation service"
            echo ""
            echo "📂 Working directory: apps/web"
            echo ""
            cd apps/web

            # Install dependencies if node_modules doesn't exist
            if [ ! -d "node_modules" ]; then
              echo "📥 Installing npm dependencies..."
              npm ci
            fi
          '';
        };

        # Build the web application
        webApp = pkgs.stdenv.mkDerivation {
          name = "texler-web";
          src = ./apps/web;

          buildInputs = with pkgs; [ nodejs ];

          buildPhase = ''
            export HOME=$(mktemp -d)
            mkdir -p $HOME/.npm
            export npm_config_cache=$HOME/.npm-cache
            export npm_config_offline=true

            # Install dependencies
            npm ci --cache $HOME/.npm-cache

            # Build the application
            npm run build
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
          exec ${nodejs}/bin/npm start
        '';

        # Build script
        buildScript = pkgs.writeShellScriptBin "texler-web-build" ''
          export NODE_ENV=production
          cd apps/web
          exec ${nodejs}/bin/npm run build
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