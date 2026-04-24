# Oflama

`Oflama` is a command‑line tool and terminal user interface (TUI) written in Rust. It aims to emulate the ease of use of `ollama` while giving the user full control over compiling `llama.cpp` (with hardware‑specific optimizations), managing GGUF models from Hugging Face, and orchestrating the server.

## Features

- **Build Manager**: Downloads and compiles `llama.cpp` locally optimized for your hardware (CUDA, Vulkan, SYCL).
- **Hugging Face Explorer**: Search and download GGUF models directly from Hugging Face.
- **Launch Assistant**: Analyzes your machine’s hardware and recommends inference configurations (layer offload) and server parameters.
- **Service Mode**: Quickly generates `.service` files to run in background using Systemd.

## Installation

You need to have Rust (2021 edition) and Cargo installed.

```bash
git clone <repository-url>
cd oflama
cargo build --release
```

## Usage

Run the binary:

```bash
cargo run --release
```

Navigate the different tabs with `Tab` and `Shift+Tab`.

## Architecture

- `tui`/`ui`: UI built with Ratatui.
- `builder`: Orchestrates CMake and compilation.
- `huggingface`: Client for model search and download.
- `orchestrator`: Inference and hardware resource manager.
- `service`: Systemd unit generator.

## License

MIT
