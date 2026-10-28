# Architecture

The Digital Logic Simulator is designed for maximum performance, actively avoiding Object-Oriented Programming (OOP) bottlenecks. The architecture is divided into two primary subsystems: the **Core Engine** and the **Editor UI**.

## Core Engine (`src/engine/`)

The simulation backend is completely decoupled from the UI. It operates on a flat, cache-friendly data structure.

### Event-Driven Simulation
Instead of a naive tick-based evaluation where every gate is processed every frame, the `Simulator` uses an event-driven queue (`event_queue`).
1. **Topological Depth Layering (Tarjan SCC)**: `calculate_depths()` runs Tarjan's algorithm to find strongly connected components, collapses each one (feedback structures such as latches become a single node) into a condensation DAG, and assigns every component its longest-path depth (`depth(v) = max(depth(u) + 1)` over its predecessors). All gates in one SCC share a depth. Pending work is held in per-depth queues (`event_queue: Vec<Vec<usize>>`), so execution order is deterministic.
2. **Zero-Allocation Evaluation Hot Loop**: `propagate_events()` executes without per-tick heap allocations. Rather than deallocating and reallocating queue buffers (such as via `std::mem::take`), the engine maintains reusable scratch buffers (`scratch_queue`, `scratch_updates`, `scratch_next_enqueues`). Dependent iterations use direct indexed slices rather than cloning vectors, eliminating memory churn during high-frequency clock cycling.
3. **Dual-Pass Layer Evaluation**: The `propagate_events` loop walks the depth layers in order and processes each layer in **two passes**. Pass 1 computes each queued gate's new state from a read-only view of the node array. When the layer's queue length reaches a hardware-calibrated threshold (`dynamic_threshold`), this runs on `rayon::par_iter()` across all cores; smaller layers run sequentially. Pass 2 applies the changed states sequentially and enqueues the dependents of every gate that changed. Because pass 1 only reads and pass 2 only writes, there are no data races, and results are identical whether or not the layer ran in parallel. If a changed gate re-enqueues a gate at an earlier depth (a feedback loop), the events are picked up once the loop runs past the last non-empty layer.
4. **IEEE-Style 4-State Contention Propagation**: Gates evaluate 4-state signals (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). `GateType::Nand` enforces dominant-low semantics (`0b01` forces output `0b10` High even if the other input is `0b11` Contention) and propagates `0b11` Contention when paired with `0b10` High.
5. **Hardware Profiler & Safe Fallback**: The dynamic crossover threshold is calibrated once at startup via `detect_parallel_crossover_threshold()` and cached in a `OnceLock`. If parallel evaluation fails to beat sequential execution up to 16,000 gates (such as on single-core or constrained mobile devices), the threshold defaults to `usize::MAX`, safely disabling parallel thread dispatch to prevent performance regressions.
6. **Oscillation Budget Scaling & Clean Recovery**: The evaluation step cap scales linearly as $\text{capacity} \times \text{multiplier}$ (defaulting to 100), preventing quadratic execution explosions on large circuits while terminating genuine zero-delay feedback loops in bounded time. When an oscillation aborts propagation, pending queues and `in_queue` bitsets are cleanly drained so subsequent user edits or input changes recover immediately.

### Compact Struct-of-Arrays (SoA) Node Storage (`src/engine/storage.rs`)
The simulator stores gates in a high-density, cache-aligned Struct-of-Arrays (`SoAGateStorage`) structure:
- **`states: Vec<u8>`**: Hot simulation signal states (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). Stored contiguously in cache lines for lightning-fast reads during gate evaluation.
- **`gate_types: Vec<GateType>`**: Gate enum primitive per index.
- **`sources: Vec<[u32; 2]>`**: Flattened static topology connections (`input_a`, `input_b`), using `NO_SOURCE` (`u32::MAX`) sentinels.
- **`dependents: Vec<Vec<u32>>`**: Forward adjacency lists of gates triggered by state transitions. Inward connections are deduplicated to prevent redundant queue scheduling.
- **`in_queue: FixedBitSet`**: Bit-packed set tracking pending queue membership, replacing byte/word booleans.
- **Dynamic Net Disconnection**: `disconnect_input(tgt_gate, slot)` clears specific pin wirings and prunes backward dependent pointers in-place, enabling surgical incremental re-wiring without graph rebuilds.
- **Memory Density**: Drops the memory footprint from ~150 bytes per gate down to **14.16 bytes per gate** (100,000 gates consume only 1.38 MB, fitting inside host CPU L2/L3 caches).

### Hierarchical Compilation & $O(\Delta)$ Delta-Compilation (`src/engine/compiler.rs`, `src/editor/canvas.rs`)
To scale beyond 1,000,000+ gates without editor lag:
- **Full Flattening Compilation**: When loading or restructuring chip definitions, `compile_full()` flattens hierarchical blueprints into a global gate array. Intermediate connections are resolved to primitive drivers, and multi-bit buses are expanded into discrete single-bit paths.
- **Template Caching**: `compile_subchip_template()` pre-compiles chip blueprints into reusable `SubchipTemplate` instances containing relative gate types, internal wirings, and interface mappings. Instantiations on canvas copy templates with base offset addition, eliminating redundant recursive tree traversals.
- **$O(\Delta)$ Incremental Delta-Compilation**: Interactive canvas edits (adding/deleting wires, placing/removing primitive gates, or placing new subchip instances) execute `compile_incremental()`. Existing `SubChip` allocations, internal gate offsets, and memory latch states are preserved untouched in memory. Only the delta components and modified nets are patched into `SoAGateStorage`, reducing recompile latency on million-gate designs from seconds to under 1 millisecond.
- **Static Blueprint Validation**: `Simulator::validate_library()` runs an $O(V + E)$ 3-color DFS cycle detector and pin-bounds check on chip libraries during project load, verifying validity without allocating throwaway simulation gates.

### Activity-Gated Subchip Hibernation (Sleep Domains, `src/engine/sleep.rs`)
In large architectures (such as 64 KB RAM containing 524,288 latches), only a single word changes state on any given clock cycle:
- **Topological Auto-Gating**: Chip-Select (CS) and Write-Enable (WE) control lines are discovered topologically during compilation via Tarjan SCC feedback analysis and fan-out asymmetry ($2N$ vs $1$), providing automatic sleep-gating even with arbitrary or default pin names.
- **$O(1)$ Sleep-Control Bitset**: `sleep_control_gates: FixedBitSet` registers all gates driving sleep domains. On state transitions, the simulator performs an $O(1)$ bit test; non-control transitions bypass `sleep_domains` lookups entirely.
- **Bit-Packed Latch Compression**: When inactive (`should_sleep()`), domain latches are compacted into dense bit-arrays (`dormant_bits: FixedBitSet`, 1 bit per latch), and internal gates are unlinked from active event queues.
- **Awakening & Remapping**: `wake()` restores latch states and schedules gates for normal evaluation. Across `defragment_and_sort_by_depth()`, `sleeping_gates` bitsets and domain indices are dynamically remapped in lockstep.

## Editor UI (`src/editor/`)

The frontend runs on native **[eframe](https://docs.rs/eframe/latest/eframe/) + [egui](https://docs.rs/egui/latest/egui/)** (`0.31`), unifying both the 2D hardware-accelerated schematic canvas (`egui::Painter`) and the docking/overlay panels under a single rendering context without foreign framework glue layers:
- **`math.rs`**: Lightweight, zero-overhead 2D geometry primitives (`Vec2`, `Rect`, `Color`) with bidirectional conversions for `egui::Pos2`, `egui::Rect`, and `egui::Color32`.
- **`state.rs`**: Tracks `AppMode`, `FrameInput` (capturing pointer, multi-touch, scroll, and keyboard events from `egui::Context`), `CanvasState`, `UiState`, and `EngineState`. Caches port mapping tables and delta-compilation snapshots.
- **`gui.rs` & `ui_*.rs`**: Manages window layouts, toolbars, properties panels, and standalone modal overlays (Settings, Global Library Manager, Credits). Centralized `apply_library_index_map()` and `delete_library_chip()` helpers keep all `SubChip` references synchronized across the main canvas, stashed blueprints, undo/redo history stacks, and nested library blueprints.
- **`drawing.rs`, `drawing_shapes.rs`, & `drawing_wires.rs`**:
  - **Batched Polylines**: Uninterrupted Manhattan wire paths are drawn via contiguous `egui::Shape::line` polylines, eliminating 16-triangle corner circle tessellations and minimizing draw calls.
  - **Multi-Tier Zoom LOD**: Viewport-culled spatial indexing (`SpatialHashGrid`) restricts rendering to visible bounds. When zoomed out (`zoom < 0.38`) or rendering over 2,500 visible wires, expensive bridge-arc intersection searches are bypassed.
  - **Monotonic Ribbon Routing**: Corridors of parallel multi-bit bus wires are sorted by Y-interval overlap and net origin, ordering upward and downward branches monotonically to eliminate crossover braids.
  - **Obstacle-Aware Wire Detours**: 5-segment Manhattan routes automatically calculate horizontal bypass channels above or below intermediate components, preventing wires from cutting through chip bodies.
- **`inspection_logic.rs` & `inspection_ui.rs`**: Cycle-safe driver tracing (`trace_local_driver_visited`) and read-only sub-chip interior visualization.
- **`canvas.rs`**: State-preserving compilation snapshots active `Input`/`Clock` states, clock counters, and `InstanceTree` gate states before rebuilding the simulator graph, ensuring live circuit memory is preserved across interactive edits.

## Cross-Platform Host Architecture

- **Desktop (Windows, Linux, macOS)**: Runs via `eframe::run_native` backed by `winit` and standard platform graphics backends (DirectX/Vulkan on Windows, OpenGL/Wayland/X11 on Linux, Metal on macOS).
- **Mobile (Android)**: Replaced legacy Macroquad JNI wrappers (`QuadSurface` / `QuadNative.java`) with a native `android.app.NativeActivity` host (`winit` + `android-activity`). Fullscreen window insets and touch-gesture pipelines pipe directly into `egui::RawInput`.
