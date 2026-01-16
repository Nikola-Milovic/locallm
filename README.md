# LocalLM

A simple chat UI for [Ollama](https://ollama.ai), built with Rust and iced. Designed for Linux/Wayland.

![LocalLM Screenshot](screenshot.png)

## Features

- 💬 Chat interface with message history
- 📋 Click any message to copy it
- ⌨️ Enter to send, Shift+Enter for new line
- 📊 AMD GPU stats (VRAM, usage, temperature)
- 🔄 Auto-detects models from Ollama

## Quick Start

### 1. Install & Run Ollama

```bash
# NixOS (AMD GPU)
nix-shell -p ollama-rocm

# Set GFX version if needed (e.g., RX 7000 series)
export HSA_OVERRIDE_GFX_VERSION="11.0.0"

# Start Ollama
ollama serve
```

### 2. Pull a Model

```bash
ollama pull llama3.2:3b
```

### 3. Run LocalLM

```bash
nix develop
cargo run --release
```

## Sway (Floating Window)

Add to `~/.config/sway/config`:

```
for_window [app_id="locallm"] floating enable
```

## Configuration

Config file: `~/.config/locallm/config.toml`

```toml
ollama_url = "http://127.0.0.1:11434"
default_model = "llama3.2:3b"
# system_prompt = "You are a helpful assistant."
auto_copy = false
show_gpu_stats = true
```

## NixOS / Home Manager

```nix
# flake.nix inputs
inputs.locallm.url = "github:you/locallm";

# home.nix
{ inputs, ... }: {
  imports = [ inputs.locallm.homeManagerModules.default ];
  
  programs.locallm = {
    enable = true;
    defaultModel = "llama3.2:3b";
  };
}
```

## Keybindings

| Key | Action |
|-----|--------|
| Enter | Send message |
| Shift+Enter | New line |
| Click message | Copy to clipboard |

## Building

```bash
nix develop
cargo build --release
```

Binary will be at `target/release/locallm`.

## License

MIT
