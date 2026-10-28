# System Requirements and Dependencies

## Supported Platforms
- **Windows** (x86_64)
- **Linux** (x86_64, Wayland/X11)
- **macOS** (AArch64 / x86_64)
- **Android** (AArch64 / ARMv7)

## Minimum System Requirements
- **CPU**: Modern multi-core processor (x86_64 or ARM64)
- **RAM**: 2 GB (Minimal baseline usage by the simulator itself; compact 14.16 bytes/gate layout fits 100,000 gates in 1.38 MB)
- **Graphics**: Hardware acceleration required (OpenGL 3.3 / WebGL 2 / DirectX 11 / Metal compatible GPU)

## Build Dependencies
To compile the project from source, you need the Rust toolchain and specific platform libraries.

### Rust Toolchain
- **rustc**: Edition 2024 (latest stable recommended)
- **cargo**: Package manager

### Linux Dependencies
On Linux systems (e.g., Ubuntu/Debian), install dependencies for graphics APIs and windowing used by `eframe` / `winit`:
```bash
sudo apt-get update
sudo apt-get install -y pkg-config libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev
```

### Android Dependencies
To package the app for Android:
- **Android NDK & SDK**: Ensure both are installed. Set `ANDROID_NDK_ROOT` to point to the NDK path (e.g. `r25c` or later).
- **Rust Android Targets**: Run `rustup target add aarch64-linux-android armv7-linux-androideabi`.
- **cargo-ndk**: Install via `cargo install cargo-ndk`.
- **Android Host**: NativeActivity architecture (`winit` + `android-activity`).

## Cargo Dependencies
The following core libraries are utilized (see `Cargo.toml` for exact versions):
- `eframe`: Official framework for building native desktop and mobile applications with `egui`.
- `egui`: Immediate-mode graphical user interface library powering the editor canvas, tools, and overlays.
- `serde` & `serde_json`: Serialization and deserialization for saving/loading `.logic` and `.json` blueprint files.
- `fixedbitset`: Bit-packed sets for queue tracking (`in_queue: FixedBitSet`), sleep-control gate bitmasks (`sleep_control_gates`), and dense latch hibernation arrays (`dormant_bits`).
- `slab`: Pre-allocated generational slot indexing and freelist management.
- `rayon`: Work-stealing threadpool and data-parallel iterators for parallel topological depth layer evaluation.
- `rfd`: Native desktop file dialogs for cross-platform project saving/loading (`cfg(not(target_os = "android"))`).
- `ndk-context`, `jni`: Android lifecycle and native activity integration (`cfg(target_os = "android")`).
- `winres`: Windows resource and executable icon compilation (`build-dependencies`).
