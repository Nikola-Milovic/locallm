# LocalLM

A lightweight chat UI for local LLMs via [Ollama](https://ollama.ai), built with Rust. Designed for NixOS + Wayland.

## Features

- 💬 **Chat interface** - Conversational UI with message history
- 🔄 **Streaming responses** - See tokens as they're generated
- 📊 **GPU stats** - Real-time VRAM usage, GPU load, and temperature (AMD)
- 📋 **Clipboard integration** - Copy responses with one click
- 🔍 **Auto-detect models** - Lists all models available in Ollama
- ❄️ **NixOS/Home Manager integration** - Declarative configuration

## Prerequisites: Ollama Setup

LocalLM is a UI that connects to Ollama. You need to have Ollama running separately.

### AMD GPU (ROCm)

On NixOS, add to your `configuration.nix`:

```nix
{ pkgs, ... }: {
  # Enable ROCm for AMD GPUs
  hardware.graphics.enable = true;  # or hardware.opengl.enable on older NixOS
  
  # Install Ollama with ROCm support
  environment.systemPackages = [ pkgs.ollama-rocm ];
  
  # Optional: run as a systemd service
  systemd.services.ollama = {
    description = "Ollama LLM Server";
    after = [ "network.target" ];
    wantedBy = [ "multi-user.target" ];
    
    environment = {
      # Required for some AMD GPUs (e.g., RX 7000 series)
      HSA_OVERRIDE_GFX_VERSION = "11.0.0";
    };
    
    serviceConfig = {
      ExecStart = "${pkgs.ollama-rocm}/bin/ollama serve";
      Restart = "always";
      User = "ollama";
      Group = "ollama";
    };
  };
  
  users.users.ollama = {
    isSystemUser = true;
    group = "ollama";
    extraGroups = [ "video" "render" ];
  };
  users.groups.ollama = {};
}
```

Or run manually:

```bash
# Set GFX version if needed for your GPU
export HSA_OVERRIDE_GFX_VERSION="11.0.0"

# Start Ollama
ollama serve
```

### NVIDIA GPU (CUDA)

```nix
{ pkgs, ... }: {
  # Enable NVIDIA drivers
  services.xserver.videoDrivers = [ "nvidia" ];
  hardware.nvidia.package = config.boot.kernelPackages.nvidiaPackages.stable;
  
  # Install Ollama (CUDA support is automatic)
  environment.systemPackages = [ pkgs.ollama ];
}
```

### Pull a Model

After Ollama is running:

```bash
# Small, fast model (~4GB)
ollama pull llama3.2:3b

# Tiny model for testing (~2GB)
ollama pull tinyllama

# Or any model from https://ollama.ai/library
ollama pull mistral
ollama pull codellama
```

## Installation

### NixOS with Home Manager

Add to your flake inputs:

```nix
{
  inputs.locallm.url = "github:you/locallm";
}
```

Then in your Home Manager config:

```nix
{ inputs, ... }: {
  imports = [ inputs.locallm.homeManagerModules.default ];

  programs.locallm = {
    enable = true;
    ollamaUrl = "http://127.0.0.1:11434";
    defaultModel = "llama3.2:3b";
    showGpuStats = true;
    # systemPrompt = "You are a helpful assistant.";
  };
}
```

### Development

```bash
# Enter dev shell
nix develop

# Build and run
cargo run

# Build release
cargo build --release
```

## Configuration

Config file: `~/.config/locallm/config.toml`

```toml
# Ollama API URL
ollama_url = "http://127.0.0.1:11434"

# Default model (must be pulled in Ollama)
default_model = "llama3.2:3b"

# Optional system prompt for all conversations
# system_prompt = "You are a helpful assistant. Be concise."

# Auto-copy responses to clipboard
auto_copy = false

# Show GPU stats panel (AMD only for now)
show_gpu_stats = true
```

## Usage

1. **Make sure Ollama is running** (`ollama serve` or via systemd)
2. **Launch LocalLM**
3. **Select a model** from the dropdown
4. **Type your message** and press Enter or click Send
5. **Watch the response stream** in real-time
6. **Copy responses** with the Copy button

### Keyboard Shortcuts

- `Enter` - Send message
- Click **Copy** - Copy last assistant response to clipboard
- Click **Clear** - Clear chat history

## GPU Stats

The bottom bar shows real-time AMD GPU stats:
- **VRAM**: Used / Total MB and percentage
- **GPU**: GPU utilization percentage
- **Temp**: GPU temperature (if available)

Stats are read from `/sys/class/drm/card0/device/` (no extra tools needed).

## Architecture

```
┌─────────────────────────────────────────────┐
│              LocalLM (Rust)                 │
├─────────────────────────────────────────────┤
│  UI Layer (iced)                            │
│  - Chat message bubbles                     │
│  - Model selector                           │
│  - Streaming text display                   │
│  - GPU stats bar                            │
├─────────────────────────────────────────────┤
│  Ollama Client (reqwest)                    │
│  - HTTP API communication                   │
│  - Streaming response parsing               │
│  - Model listing                            │
├─────────────────────────────────────────────┤
│  System Integration                         │
│  - wl-copy for clipboard                    │
│  - sysfs for GPU stats                      │
│  - XDG config directories                   │
└─────────────────────────────────────────────┘
        │
        ▼ HTTP (localhost:11434)
┌─────────────────────────────────────────────┐
│              Ollama Server                  │
│  - Model management                         │
│  - Inference (ROCm/CUDA/CPU)               │
│  - Auto load/unload models                  │
└─────────────────────────────────────────────┘
```

## Troubleshooting

### "Ollama not running"

Make sure Ollama is serving:

```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start it if not
ollama serve

# Or check systemd service
systemctl status ollama
```

### "No models found"

Pull at least one model:

```bash
ollama pull llama3.2:3b
```

### AMD GPU not detected / slow inference

1. Make sure you're using `ollama-rocm` package
2. Set the GFX version override if needed:

```bash
export HSA_OVERRIDE_GFX_VERSION="11.0.0"  # For RDNA3
```

Check your GPU's GFX version with:

```bash
rocminfo | grep gfx
```

### GPU stats not showing

GPU stats require an AMD GPU with proper sysfs entries. Check if these files exist:

```bash
cat /sys/class/drm/card0/device/mem_info_vram_used
cat /sys/class/drm/card0/device/mem_info_vram_total
```

## License

MIT
