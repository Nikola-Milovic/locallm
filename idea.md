# Local LLM Prompt Interface - Future Project Idea

## Overview

A standalone application that provides a quick-access menu/popup for prompting a local LLM running on GPU. This could work as a companion to whisp-away or as an entirely separate project.

## Core Concept

- Open a lightweight popup/menu (possibly via keyboard shortcut or system tray)
- Type or paste text into an input field
- Prompt a small, fast local LLM (1-3B parameters) running on the GPU
- Use cases:
  - Post-process transcribed text before inserting
  - Reformat text in different styles
  - Quick AI assistance without leaving current workflow
  - Grammar/spelling fixes
  - Text transformations (summarize, expand, translate, etc.)

## Technical Considerations

### LLM Backend Options

- **llama.cpp** - C/C++ inference, good GPU support (CUDA, Vulkan, etc.)
- **Ollama** - Easy model management, runs as daemon
- **vLLM** - High performance, Python-based
- **llama-cpp-python** - Python bindings for llama.cpp

### UI Framework Options

- **GTK4** - Native Linux look, integrates well with system
- **egui** - Rust-native, easy to use, cross-platform
- **Tauri** - Web-based UI with Rust backend, modern look
- **rofi/wofi** - Could create a custom mode for existing launcher

### Model Suggestions (1-3B parameters, fast on GPU)

- Phi-2 / Phi-3-mini (2.7B)
- Gemma-2B
- Qwen 1.5 1.8B
- TinyLlama 1.1B
- StableLM 2 1.6B

## Integration Ideas with whisp-away (skip for now)

Could add an optional post-processing step after transcription:

1. User records voice → transcription via whisper
2. Before output, prompt appears asking "Process with LLM? [Enter to skip]"
3. User types instruction (e.g., "format as bullet points", "fix grammar")
4. LLM processes the transcription
5. Result is typed/copied

## Minimal Implementation Sketch

```
1. Keyboard shortcut triggers popup (e.g., via global hotkey daemon)
2. Popup shows:
   - Input text area (paste/type)
   - Prompt instruction field
   - Submit button / Enter to run
3. Sends request to local LLM daemon
4. Result either:
   - Copied to clipboard
   - Typed at cursor position
   - Displayed for manual copy
```

## Open Questions

- Should this integrate into whisp-away or be a completely separate tool?
- How to handle the popup on Wayland vs X11?
- Should it run its own LLM daemon or connect to Ollama/etc?
- How to make the UI minimal but functional?
