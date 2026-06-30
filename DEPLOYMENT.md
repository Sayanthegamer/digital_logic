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

## Android Build

You can also build the application as a standalone Android APK. Because the interface is built with Macroquad and egui, it is fully compatible with touch screens and compiles directly to native Android code.

### Prerequisites

1. **Install Android NDK & SDK**:
   - Ensure the Android SDK and NDK are installed on your machine.
   - Set the `NDK_HOME` environment variable to point to your NDK installation directory.

2. **Add Android Targets**:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi
   ```

3. **Install cargo-quad-apk**:
   ```bash
   cargo install cargo-quad-apk
   ```

### Building the Release APK

1. **Generate a Keystore**:
   To build in release mode, a keystore file named `release.keystore` must exist in the root directory of the `logic_simulator` package. You can generate a self-signed one using:
   ```bash
   keytool -genkey -v -keystore release.keystore -alias release-key -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android, O=Android, C=US" -storepass android -keypass android
   ```
   *(Note: The keystore credentials `android` and alias `release-key` match the pre-configured settings in `Cargo.toml`).*

2. **Compile the APK (Local Setup)**:
   ```bash
   cargo quad-apk build --release
   ```

The resulting signed APK will be located at:
- `.\target\android-artifacts\release\apk\logic_simulator.apk`

