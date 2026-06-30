# System Requirements and Dependencies

## Supported Platforms
- **Windows** (x86_64)
- **Linux** (x86_64, Wayland/X11)
- **macOS** (AArch64 / x86_64)
- **Android** (AArch64 / ARMv7)


## Minimum System Requirements
- **CPU**: Modern multi-core processor (x86_64 or ARM64)
- **RAM**: 2 GB (Minimal usage by the simulator itself, scaling based on the complexity of loaded circuits)
- **Graphics**: Hardware acceleration required (OpenGL 3.3 / WebGL 2 / DirectX 11 / Metal compatible)

## Build Dependencies
To compile the project from source, you need the Rust toolchain and specific platform libraries.

### Rust Toolchain
- **rustc**: Edition 2024 (latest stable recommended)
- **cargo**: Package manager

### Linux Dependencies
On Linux systems (e.g., Ubuntu/Debian), you must install dependencies for graphics and audio APIs used by Macroquad/egui.
```bash
sudo apt-get update
sudo apt-get install -y pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev libwayland-dev
```

### Android Dependencies
To package the app for Android, you must configure:
- **Android NDK & SDK**: Ensure both are installed. Set `ANDROID_NDK_ROOT` to point to the NDK path (e.g., version `r25c`).
- **Rust Android Targets**: Run `rustup target add aarch64-linux-android armv7-linux-androideabi`.
- **cargo-apk**: The compilation tool installed via `cargo install cargo-apk`.


## Cargo Dependencies
The following core libraries are utilized (see `Cargo.toml` for exact versions):
- `macroquad`: High-performance 2D rendering and window creation.
- `egui`: Immediate mode GUI for editor interfaces.
- `egui-macroquad`: Binding layer between Macroquad and egui.
- `serde` & `serde_json`: Serialization and deserialization for saving/loading `.logic` blueprint files.
- `rfd`: Native file dialogs for cross-platform file saving/loading.
