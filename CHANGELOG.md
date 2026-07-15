# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.2.0-alpha.4] - 2026-07-15 (Pre-release)

### Fixed
- **Gate Unresponsiveness (in_queue invariant broken)**: Fixed a fatal issue where adding, removing, or wiring components would permanently freeze gates (like NANDs). The simulator was improperly clearing the event queue without resetting the `in_queue` flags, trapping pending gates in a permanently "queued" but un-evaluable state.
- **Rayon Execution Determinism**: Fixed a critical release-build bug where the Rayon `par_iter().filter_map()` pipeline evaluated gates out of order under heavy optimization chunking. Rewrote the parallel reduction to use an `IndexedParallelIterator` (`map().collect().flatten()`) to strictly preserve chronological topological evaluation.
- **Removed Culling Debug Text**: Cleaned up the development culling debug counter that was hardcoded to draw at the top of the canvas during rendering optimizations.

## [3.2.0-alpha.2] - 2026-07-14 (Pre-release)

### Added
- **Intelligent Hardware Profiler**: Introduced a runtime calibration module (`src/engine/profiler.rs`) that micro-benchmarks the host CPU on startup to dynamically determine the exact Rayon parallelization crossover threshold. This completely eliminates magic numbers and adapts scaling to any hardware profile.

### Optimized
- **Multi-Threaded Event Loop**: Upgraded the core simulator loop (`propagate_events`) to utilize a two-pass Map-Reduce pattern with `rayon::par_iter()`. Safe concurrent evaluation is guaranteed by isolating nodes into strict topological depth layers via Tarjan's SCC.
- **L1/L2 Cache Defragmentation**: Implemented a topological sorting pass on the simulation `Slab` immediately after hierarchy flattening (`calculate_depths`). This guarantees that independent gates executing in parallel on the same clock phase are situated contiguously in memory, eliminating cache misses and maximizing hardware pre-fetcher efficiency.
- **Viewport Culling (Spatial Hash)**: Purged the $O(N)$ AABB drawing loop for components and wires. The editor now queries independent `SpatialHashGrid`s for components and wires bounded by the `viewport_rect`, isolating only the visible on-screen elements. This completely drops the rendering workload from $O(V + E)$ down to purely the on-screen element count, restoring blazing fast framerates on massive circuits.

### Fixed
- **Slab ABA Component Poisoning**: Fixed a critical memory safety/logic flaw where deleting a gate mid-tick and immediately placing a new component could cause the fresh component to inherit stale simulation events. The event queue is now safely purged on all topological circuit modifications.

## [3.1.0-alpha.4] - 2026-07-14 (Pre-release)

### Fixed
- **Wire Crossing Visual Artifacts**: Eliminated massive visual artifacts at wire crossings caused by the previous alpha blending technique. Replaced overlapping `draw_circle` segments with continuous parametric lines (`draw_line`) in bridge arcs to eliminate fuzzy dotted overdraw.
- **Dynamic Arc Radii**: Updated `gap_radius` calculations to dynamically match the thickness of the lowest wire it is crossing, eliminating the issue of active "glow" blooming through the gaps in the upper wire.
- **Diagonal Crossing Alignment**: Fixed floating arc issues where 45-degree chamfer crossings resulted in misaligned bridge arcs. Arcs are now drawn parametrically along the true vector direction (`dir`) of the wire and bulged exactly perpendicular to the local segment rather than strictly up/down/left/right.
- **Wire "Clipping" and Clutter**: Added 1D interval hole merging (`O(K log K)`) so that multiple adjacent gaps from tightly packed vertical wire bundles smoothly merge into a single extended gap. Replaced rigid multi-bump rendering with a single unified dynamic bridge arc across the entire bundle, removing ugly overlapping wavy artifacts.
- **Zero-Allocation Rendering**: Resolved rendering stutter ("not performative") from `Vec::new()` calls inside the massive inner wire loop. Pre-hoisted and `clear()`ed Vectors significantly drops dynamic allocations per frame to zero.

## [3.1.0-alpha.1] - 2026-07-13 (Pre-release)

### Fixed
- **History Snapshot Regression**: Restored undo snapshot creation on component drag starts, fixing a critical regression where component movements could not be undone.
- **Documentation Accuracy**: Updated `SCALE_BREAKING_POINTS.md` to accurately reflect that wire routing and crossing detection have been mitigated (with hard limits) rather than fully achieving O(1)/resolved status.

### Optimized (Phase 8: Performance & Scaling Overhaul)
- **Topological Simulation (S-01, S-02)**: Completely rebuilt the simulator's depth calculator using Tarjan's Strongly Connected Components (SCC) algorithm. This replaces Kahn's algorithm, natively isolating concurrent independent feedback loops into their own deterministic depth layers. This eliminates non-deterministic data races, false-positive oscillations on SR latches, and the O(N³) Bellman-Ford bottleneck.
- **O(N) Spatial Hashing for Rendering (S-04)**: Wire intersection/crossing deduplication was completely rewritten using a 2D spatial hash grid for the *primary intersection search*, dropping the $O(N^2)$ sweep-and-prune worst-case entirely.
- **O(N) Spatial Hash Lane Allocation (S-05)**: Modified `recompute_wire_offsets` to replace $O(N^2)$ interval coloring with an $O(N)$ Spatial Hash algorithm. This eliminated the 5,000 connection hard-limit, and correctly checks dynamically dragged wires against static wire bounds to prevent visual collisions.
- **Linear Scan Removal (SC-01)**: Fully replaced all `self.components.iter().find()` usage with O(1) component HashMaps across all files in `src/editor/` and `src/engine/`.
- **Compiler Optimizations (SC-03, SC-05, SC-06)**: Eliminated heavy $O(E)$ nested linear scans in `instantiate_chip_with_mapping` with pre-built Maps, reused allocation buffers, and transitioned component hierarchy paths to `u64` hashes.
- **History Memory Limits (S-06)**: Capped the undo/redo stack to 30 elements and restricted deep cloning to action-release points rather than every drag frame to prevent RAM leaking.

## [3.0.0-alpha.1] - 2026-07-10

### Added
- **Auto Arrange:** Added an automatic layout engine based on the Sugiyama framework. This feature automatically sorts components left-to-right, reserves routing space (dummy nodes) for long wires, minimizes crossings via multi-sweep barycenter heuristics, aligns components on the Y-axis to straighten wires, and snaps everything to the grid. Available via the new "Auto Arrange" button in the toolbar.
- **Smart Perpendicular Junction Tap Routing**: Wires now dynamically attach/tap perpendicularly along the nearest point of a stretched Junction component body relative to the target/source coordinates. This completely eliminates wire-endpoint clutter and overlaps around stretched Junction blocks.
- **Connection Deduplication**: Added connection filtering to prevent identical wire connection duplicates from stacking on top of each other.

### Optimized
- **Global Channel-Based Wire Routing**: Replaced the local, quadratic $O(n \cdot m)$ dynamic routing offset solver with a global lane allocation pass. Using greedy interval coloring and spatial sorting, the editor resolves and caches non-overlapping alternating offsets (0, +12, -12, +24, -24, ...) on compile and layout changes, reducing the per-frame lookup to a clean $O(1)$ HashMap fetch.
- **Decomposed Input Management**: Decomposed the massive, monolithic `input.rs` and `input_interactions.rs` files into 10 highly cohesive submodules (navigation, keyboard, hover, interactions, press, down, release, delete, context_menu, and simulation) to improve maintainability and decouple state orchestration.

## [2.3.0-alpha.1] - 2026-07-09

### Added
- **Manual Wire Nudging:** You can now manually click and drag any wire to adjust its routing offset, allowing precise control to detour around large components. Custom routing offsets are seamlessly integrated with the global lane solver and are saved within your `.logic` project file.

### Fixed
- **Jump Arc Consistency:** Fixed a bug where intersecting wire jump arcs would draw horizontally or vertically depending on arbitrary processing order, often using the wrong color. Arcs are now strictly bound to the horizontal wire.
- **Backward Routing Overlap:** Fixed an issue where multiple wires routing backwards (right-to-left) into the same component would draw overlapping vertical segments that caused visual tearing and color swapping. Backward routes now intelligently stagger like bus splitters to guarantee they never overlap.
- **Zoom Clamping Limits:** Drastically lowered the minimum zoom clamp down to `0.01` to allow smooth recentering and viewing of massive circuits without artificial cutoffs.
- **Recenter Annotations:** The Recenter button now includes floating text annotations when framing the circuit bounding box, mirroring the SVG export logic.

## [2.2.0-alpha.5] - 2026-07-09
### Fixed
- **DPI-Aware Camera Centering:** Fixed `apply_camera_bounds` to use the cached canvas viewport area in physical pixels (`self.ui.canvas_viewport`) instead of the raw window boundaries. This ensures that camera recentering and fit-to-screen functions calculate zoom and pan accurately on high DPI screens and respect egui side panels without clipping/hiding components.

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


