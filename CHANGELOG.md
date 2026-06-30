# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-alpha.2] - 2026-06-30

### Added
- **UI Menu Scrolling & Navigation:**
  - Added flanking scroll buttons (◀ and ▶) to the top Controls panel and the bottom Parts Catalog.
  - Implemented vertical mouse wheel redirection to horizontal scrolling for improved menu navigation on desktop.

### Changed
- **Editor Architecture:**
  - Decomposed massive monolithic files (`gui.rs`, `drawing.rs`, `inspection.rs`) into smaller, modular, maintainable, and highly cohesive domain-specific files (`drawing_shapes.rs`, `ui_properties.rs`, `inspection_logic.rs`, etc.).

## [0.2.0-alpha.1] - 2026-06-30
### Changed
- **Android Build Pipeline:**
  - Transitioned from `cargo-quad-apk` to native Gradle-based build system using `cargo-ndk`.
  - Upgraded project target SDK, build tools, and Android support structures.
  - Manually injected JNI `quad_main` entry point to prevent runtime loading issues.
- **Rust Toolchain:**
  - Upgraded codebase to Rust 2024 Edition.
  - Re-enabled modern features (e.g. `let_chains`) across the project.

## [0.1.0-alpha.1] - Initial Baseline

### Added
- **Core Engine:**
  - Event-driven propagation simulator (`Simulator`).
  - Primitives: Nand, Input, Output, and Clock.
  - Multi-domain clock tick resolution.
  - Oscillation detection mechanism to prevent infinite loops.
  - Flat array compiler for turning deeply nested `ComponentType::SubChip` graphs into contiguous primitive arrays.

- **Editor UI:**
  - Macroquad integration for high-performance 2D logic canvas rendering.
  - Egui integration for inspection panels, toolbars, and library menus.
  - Pan/Zoom controls and grid snapping.
  - Wire routing and connection mechanics.
  - Persistence layer allowing saving and loading of `.logic` project files using `serde`.

- **Android Support:**
  - Configured `cargo-apk` integration for compiling to native Android APKs.
  - Implemented target-gated platform features (e.g., file dialogs are target-gated on desktop, falling back to a fixed directory on Android).
  - Created automated build and code-signing pipelines in GitHub Actions.

- **CI/CD & Release Automation:**
  - Integrated automated GitHub Release creation and asset deployment triggered by version tags (`v*`).
  - Added cryptographic build attestation (provenance verification) for both Windows and Android builds.
  - Integrated automated VirusTotal scanning for release binaries.


- **Documentation:**
  - Added ARCHITECTURE.md, DESIGN.md, SPEC.md, SYSTEM.md, DEPLOYMENT.md, and README.md detailing the core philosophy.


