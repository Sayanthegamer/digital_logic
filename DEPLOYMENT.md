# Deployment and Build Guide

This simulator is built using Rust. It compiles to native standalone executables.

## Prerequisites

1. Install [Rust via rustup](https://rustup.rs/).
2. For Linux users, install the required graphics/wayland dependencies:
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install -y pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev libwayland-dev
   ```

## Development Build

To build and run the simulator in debug mode (faster compilation, slower runtime performance):

```bash
cargo run
```

To run the test suite:

```bash
cargo test
```

## Production / Release Build

For maximum performance (crucial when testing large circuits like the 8-bit or 16-bit CPUs), you **must** build in release mode. The release mode enables heavy compiler optimizations.

```bash
cargo build --release
```

The resulting executable will be located at:
- Linux/macOS: `./target/release/logic_simulator`
- Windows: `.\target\release\logic_simulator.exe`

You can run it directly:

```bash
cargo run --release
```

## Distribution

Because the application compiles to a statically linked binary (aside from system libraries like OpenGL/X11), you can simply zip the executable generated in the `target/release/` directory and share it with users on the same operating system.
