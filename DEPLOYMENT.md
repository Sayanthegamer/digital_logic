# Deployment and Build Guide

This simulator is built using Rust and `eframe`. It compiles to native standalone executables for desktop and Android.

## Prerequisites

1. Install [Rust via rustup](https://rustup.rs/).
2. For Linux users, install the required graphics/Wayland dependencies for `eframe`/`winit`:
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install -y pkg-config libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev
   ```

## Development Build

To build and run the simulator in debug mode:

```bash
cargo run
```

To run the complete test suite (51 unit tests and 7 integration suites, 67 tests total):

```bash
cargo test
```

## Production / Release Build

For maximum performance (crucial when simulating complex circuits like 32-bit CPUs, multi-bank RAM, or 1,000,000+ gate circuits), you **must** build in release mode:

```bash
cargo build --release
```

The resulting executable will be located at:
- **Linux/macOS**: `./target/release/logic_simulator_bin`
- **Windows**: `.\target\release\logic_simulator_bin.exe`

You can run it directly:

```bash
cargo run --release
```

## Distribution

Because the application compiles to a standalone binary (linked to system graphics backends like DirectX, Vulkan, Metal, or OpenGL/Wayland/X11), you can package the executable generated in `target/release/` and distribute it directly.

## Android Build

The application compiles as a native Android APK using `android.app.NativeActivity` and `eframe` / `winit`, offering fullscreen hardware acceleration and touch-gesture support.

### Prerequisites

1. **Install Android NDK & SDK**:
   - Ensure the Android SDK and NDK are installed on your machine.
   - Set the `NDK_HOME` or `ANDROID_NDK_ROOT` environment variable to point to your NDK installation directory.

2. **Add Android Targets**:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi
   ```

3. **Install cargo-ndk**:
   ```bash
   cargo install cargo-ndk
   ```

### Building the Release APK

1. **Compile Native Libraries**:
   Compile the Rust codebase using `cargo-ndk` to generate the `.so` native library binaries:
   ```bash
   cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --release
   ```

2. **Assemble APK with Gradle**:
   Navigate to the `android/` Gradle project directory and compile the package:
   ```bash
   cd android
   ./gradlew assembleRelease
   ```

The resulting release APK will be located at:
- `android/app/build/outputs/apk/release/app-release.apk`
