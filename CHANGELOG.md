# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0-alpha.4] - 2026-07-09

### Added
- **SVG Vector Picture Export:** Added a toolbar option to export a high-fidelity vector diagram of the entire circuit bounds to `.svg` format. Wires, component labels, body shapes, accent colors, ports, custom sub-chip port names, and annotations are fully structured, translated, styled, and rendered.
- **SVG Export Test Coverage:** Added unit test `test_svg_export` to verify generated SVG file integrity.

## [2.2.0-alpha.3] - 2026-07-09

### Added
- **High DPI Resolution Support:** Configured Macroquad `Conf` with `high_dpi: true` to enable sharp, high-resolution rendering on modern Retina and high-DPI displays, resolving blurry/pixelated viewport issues.
- **Multisample Anti-Aliasing (MSAA):** Added `sample_count: 4` to window configuration to smoothly render diagonal wire lines, circles, and borders.
- **Dynamic Text Decluttering on Zoom Out:** Automatically hide component labels and sub-chip port names when zooming out below 35% (`zoom < 0.35`) to avoid illegible microscopic text and screen pixelation.

## [2.2.0-alpha.2] - 2026-07-08

### Added
- **Scale-Breaking Points Documentation:** Added `SCALE_BREAKING_POINTS.md` documenting known performance bottlenecks, quadratic complexities ($O(n^2)$ routing offset and junction detection), and port count metadata mismatches.

### Fixed
- **Seven-Segment Display Port Allocation:** Fixed top-level `SevenSegment` component allocation to correctly allocate all 8 input gates (A-G and minus segment), correcting an off-by-one error where the 8th port (minus segment) silently dropped its connection.

## [2.2.0-alpha.1] - 2026-07-07

### Added
- **True Multi-Bit Buses:** Implemented high-level Bus routing capabilities using Joiner and Splitter components.
- **Bus Joiner & Splitter Components:** Added dynamic Joiner and Splitter parts under the "Bus & Routing" catalog to group/split 2-16 single-bit signals.
- **Thick Bus Line Rendering:** Bus connections are drawn at 2.2x thickness with glowing bloom overlays for distinct visual representation on the canvas.
- **Sorted Fan-Out Routing:** Overhauled Manhattan connection routing to dynamically sort and lane fan-out lines, entirely avoiding overlapping overlaps on shared source/target nodes.
- **Continuous Wire Crossing Arcs:** Improved bridge arc visual quality by showing the underlying continuous wire passing through crossings.
- **Dynamic Port Scaling:** Visual port pins, selection boxes, and click hit-testing now automatically resize and align in real-time as bus widths change.

## [2.1.0-alpha.2] - 2026-07-02

### Added
- **Coordinate Transformation Tests:** Added new unit tests verifying the correctness of screen-to-world and world-to-screen coordinate mapping.
- **Port Count Estimation Tests:** Implemented unit tests checking input/output port counts for standard components and custom subchips.
- **Compilation Error Handling Tests:** Added testing for invalid nested Input/Output components during blueprint compilation.
- **Project Persistence Tests:** Added integration tests verifying standard save and load flows via a temporary file path.
- **Blueprint Recursion Cycle Test:** Added a test verifying cycle detection during custom chip compilation.

### Fixed
- **Insecure storage on Android:** Replaced fixed project save location with standard internal sandbox folder query via JNI `getFilesDir`.
- **Malicious File Size cap:** Introduced a 50MB maximum load cap to protect memory exhaustion.
- **Blueprint Instantiation Cycles:** Implemented a compiler stack-based cycle detector to prevent stack overflow crashes on cyclic subchips.
- **Bounds-Safe Library Lookups:** Avoided index out-of-bounds panics by using `.get()` / `.get_mut()` instead of direct index operators on the blueprint library.
- **Junction Stretch Coordinate Overflow:** Clamped stretchable Junction sizes to `2000.0` pixels to avoid coordinates overflowing to infinity.
- **UI Scale Sanitization:** Added range clamping (`0.5..=3.0`) for target interface scaling settings to prevent divide-by-zero crashes.

### Optimized
- **TraceNode & CanvasNode Allocations:** Avoided redundant allocations by deriving `Copy` on these data nodes and removing `.clone()` calls.
- **HashMap Entry Collision Allocation:** Prevented key clones inside mapping resolution by using `get_mut` checks.
- **Monolithic UI and Input controller:** Extracted drawing layouts and update handling routines into distinct sub-helpers.

## [2.1.0-alpha.1] - 2026-07-01

### Added
- **Look Inside Camera Preservation:** Implemented a new camera stack system. When zooming into nested sub-chips via "Look Inside", your previous pan and zoom states are saved. Exiting the inspection view now perfectly restores your original view.
- **Dynamic Text Fitting:** Chip bodies now calculate their minimum width dynamically based on their pin names and titles, entirely eliminating text overlap and collision bugs for custom chips.
- **Undo/Redo Camera Snapshots:** The `CanvasSnapshot` system now saves and restores the camera's pan and zoom offsets, ensuring `Ctrl+Z` undoes camera framing as well.

## [2.0.0-alpha.1] - 2026-07-01

### Added
- **Main Menu UI Overhaul:** Introduced a structured AppMode routing system, converting the simulator from a single-screen canvas to a full application.
- **Application Modes:** Added dedicated screens for New Project, Open Project, Settings, Credits, and Manage Chips.
- **Library Management:** Users can now safely delete saved sub-chips from the "Manage Chips" menu, which properly clears corresponding placed components on the canvas.
- **Settings Overlay:** Modified the graphics settings panel to be accessible directly from the Main Menu.

## [1.2.0-alpha.1] - 2026-07-01

### Added
- **UI Component Catalog Search:** Added a search bar to dynamically filter components in the catalog.
- **Stretchable Bus Junction:** Bus Junction components can now be stretched horizontally or vertically.

### Fixed
- **Canvas & Viewport:** Centering logic refactored to use measured canvas viewport rectangle.
- **Undo/Redo & Snapping:** Integrated stretchable junctions properly with grid snapping and history tracking.
- **Compilation & Build:** Resolved a double mutable borrow (E0499) in input handling and fixed binary/library output filename collisions in Cargo.toml.

## [0.2.0-alpha.4] - 2026-06-30

### Added
- **Text Annotation Editing & Deletion:**
  - Added double-click/double-tap detection on text annotations to automatically focus and edit text.
  - Added keyboard shortcut deletion support (via `Delete` or `Backspace` keys) for selected annotations.
  - Added a "Delete Selected" button to the properties panel when an annotation is selected.

## [0.2.0-alpha.3] - 2026-06-30

### Fixed
- **Android Gradle Build Pipelines:**
  - Removed deprecated package attribute in `AndroidManifest.xml` in favor of configuration in `build.gradle`.
  - Added native NDK stripping suppression to silence redundant `.so` warnings.
  - Added `@SuppressWarnings("deprecation")` to deprecated UI APIs in `MainActivity.java` that are necessary for back-compatibility with older Android versions.

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


