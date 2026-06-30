# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
  - Restructured codebase into a Cargo Workspace (isolating the `desktop` binary crate from the root `logic_simulator` library crate) to solve `cargo-apk` build conflicts.


- **Documentation:**
  - Added ARCHITECTURE.md, DESIGN.md, SPEC.md, SYSTEM.md, DEPLOYMENT.md, and README.md detailing the core philosophy.


