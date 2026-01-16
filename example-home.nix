# Example Home Manager configuration for LocalLM
# Add this to your home.nix or import it

{ config, pkgs, inputs, ... }:

{
  imports = [
    inputs.locallm.homeManagerModules.default
  ];

  programs.locallm = {
    enable = true;

    # Ollama API URL (default)
    ollamaUrl = "http://127.0.0.1:11434";

    # Default model to select on startup
    defaultModel = "llama3.2:3b";

    # Optional system prompt
    # systemPrompt = "You are a helpful assistant. Be concise and clear.";

    # Auto-copy responses to clipboard
    autoCopy = false;

    # Show GPU stats (VRAM usage, etc.)
    showGpuStats = true;
  };
}
