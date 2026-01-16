{
  description = "LocalLM - Lightweight chat UI for local Ollama LLMs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Build dependencies
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        # Runtime dependencies
        buildInputs = with pkgs; [
          # GUI dependencies (iced)
          wayland
          wayland-protocols
          libxkbcommon
          vulkan-loader

          # For X11 fallback
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi

          # TLS for HTTP client
          openssl

          # Clipboard
          wl-clipboard
        ];
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "locallm";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # OpenSSL configuration
          OPENSSL_NO_VENDOR = 1;

          postInstall = ''
            mkdir -p $out/share/applications
            cp ${./locallm.desktop} $out/share/applications/locallm.desktop
          '';

          meta = with pkgs.lib; {
            description = "Lightweight chat UI for local Ollama LLMs";
            homepage = "https://github.com/you/locallm";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          RUST_BACKTRACE = "1";
          OPENSSL_NO_VENDOR = 1;

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
          '';
        };
      }
    ) // {
      # Home Manager module
      homeManagerModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.locallm;
        in
        {
          options.programs.locallm = {
            enable = lib.mkEnableOption "LocalLM - local LLM chat interface";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              description = "The locallm package to use";
            };

            ollamaUrl = lib.mkOption {
              type = lib.types.str;
              default = "http://127.0.0.1:11434";
              description = "Ollama API URL";
            };

            defaultModel = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Default model to use";
            };

            systemPrompt = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = "Default system prompt";
            };

            autoCopy = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Automatically copy responses to clipboard";
            };

            showGpuStats = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "Show GPU stats (VRAM usage, etc.)";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package pkgs.wl-clipboard ];

            xdg.configFile."locallm/config.toml".text = ''
              ollama_url = "${cfg.ollamaUrl}"
              ${lib.optionalString (cfg.defaultModel != null) ''default_model = "${cfg.defaultModel}"''}
              ${lib.optionalString (cfg.systemPrompt != null) ''system_prompt = "${cfg.systemPrompt}"''}
              auto_copy = ${lib.boolToString cfg.autoCopy}
              show_gpu_stats = ${lib.boolToString cfg.showGpuStats}
            '';
          };
        };
    };
}
