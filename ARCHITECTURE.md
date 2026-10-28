# Architecture

The Digital Logic Simulator is designed for maximum performance, actively avoiding Object-Oriented Programming (OOP) bottlenecks. The architecture is divided into two primary subsystems: the **Core Engine** and the **Editor UI**.

## Core Engine (`src/engine/`)

The simulation backend is completely decoupled from the UI. It operates on a flat, cache-friendly data structure.

### Event-Driven Simulation
Instead of a naive tick-based evaluation where every gate is processed every frame, the `Simulator` uses an event-driven queue (`event_queue`).
1. **Topological Depth Layering (Tarjan SCC)**: `calculate_depths()` runs Tarjan's algorithm to find strongly connected components, collapses each one (feedback structures such as latches become a single node) into a condensation DAG, and assigns every component its longest-path depth (`depth(v) = max(depth(u) + 1)` over its predecessors). All gates in one SCC share a depth. Pending work is held in per-depth queues (`event_queue: Vec<Vec<usize>>`), so execution order is deterministic.
2. When an input changes, only the gates directly dependent on that input are queued for re-evaluation.
3. The `propagate_events` loop walks the depth layers in order and processes each layer in **two passes**. Pass 1 computes each queued gate's new state from a read-only view of the node array. When the layer's queue length reaches a hardware-calibrated threshold (`dynamic_threshold`), this runs on `rayon::par_iter()` across all cores; smaller layers run sequentially. Pass 2 applies the changed states sequentially and enqueues the dependents of every gate that changed. Because pass 1 only reads and pass 2 only writes, there are no data races, and results are identical whether or not the layer ran in parallel. If a changed gate re-enqueues a gate at an earlier depth (a feedback loop), the events are picked up once the loop runs past the last non-empty layer.
4. **IEEE-Style 4-State Contention Propagation**: Gates evaluate 4-state signals (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). `GateType::Nand` enforces dominant-low semantics (`0b01` forces output `0b10` High even if the other input is `0b11` Contention) and propagates `0b11` Contention when paired with `0b10` High.
5. **Hardware Profiler & Safe Fallback**: The dynamic crossover threshold is calibrated once at startup via `detect_parallel_crossover_threshold()` and cached in a `OnceLock`. If parallel evaluation fails to beat sequential execution up to 16,000 gates (such as on single-core or constrained mobile devices), the threshold defaults to `usize::MAX`, safely disabling parallel thread dispatch to prevent performance regressions.
6. **Oscillation Budget Scaling & Clean Recovery**: The evaluation step cap scales linearly as $\text{capacity} \times \text{multiplier}$ (defaulting to 100), preventing quadratic execution explosions on large circuits while terminating genuine zero-delay feedback loops in bounded time. When an oscillation aborts propagation, pending queues and `in_queue` bitsets are cleanly drained so subsequent user edits or input changes recover immediately.

### Compact Struct-of-Arrays (SoA) Node Storage (`src/engine/storage.rs`)
The simulator stores gates in a high-density, cache-aligned Struct-of-Arrays (`SoAGateStorage`) structure:
- **`states: Vec<u8>`**: Hot simulation signal states (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). Stored contiguously in cache lines for lightning-fast reads during gate evaluation.
- **`gate_types: Vec<GateType>`**: Gate enum primitive per index.
- **`sources: Vec<[u32; 2]>`**: Flattened static topology connections (`input_a`, `input_b`), using `NO_SOURCE` (`u32::MAX`) sentinels.
- **`dependents: Vec<Vec<u32>>`**: Forward adjacency lists of gates triggered by state transitions.
- **`in_queue: FixedBitSet`**: Bit-packed set tracking pending queue membership, replacing byte/word booleans.
- **Memory Density**: Drops the memory footprint from ~150 bytes per gate down to **14.16 bytes per gate** (100,000 gates consume only 1.38 MB, fitting inside host CPU L2/L3 caches).

### Hierarchical Subchip Template Caching (`src/engine/compiler.rs`)
To prevent multi-second UI freezing when placing complex subchips (e.g. 32-bit ALUs or register banks) on the canvas, the compiler utilizes template caching:
- `compile_subchip_template()` pre-compiles a chip blueprint into a reusable `SubchipTemplate` containing local gate types, internal connections, interface offsets, and relative clocks/sleep domains.
- Instantiating identical chips clones the template into the simulator via `instantiate_into()` with simple base offset arithmetic, avoiding repeated recursive blueprint flattening.

### Activity-Gated Subchip Hibernation (Sleep Domains, `src/engine/sleep.rs`)
In large architectures (such as 64 KB RAM containing 524,288 latches), only a single word changes state on any given clock cycle:
- The compiler registers `SleepDomain` boundaries for gated subchips, controlled by a driving gate (`control_gate`) and active polarity.
- When inactive (`should_sleep()`), the domain enters hibernation: its latch states are compacted into dense bit-arrays (`dormant_bits: FixedBitSet`, 1 bit per latch), and all internal gates are removed from active event queues.
- When awakened, `wake()` restores the latch states and schedules gates for normal evaluation.
- Across `defragment_and_sort_by_depth()`, `sleeping_gates` bitsets and domain indices are dynamically remapped in lockstep.

## Editor UI (`src/editor/`)

The frontend runs on native **[eframe](https://docs.rs/eframe/latest/eframe/) + [egui](https://docs.rs/egui/latest/egui/)** (`0.31`), unifying both the 2D hardware-accelerated schematic canvas (`egui::Painter`) and the docking/overlay panels under a single rendering context without `macroquad` or `miniquad` glue layers:
- **`math.rs`**: Provides lightweight, zero-overhead 2D geometry primitives (`Vec2`, `Rect`, `Color`) with direct `egui::Pos2`, `egui::Rect`, and `egui::Color32` conversions.
- **`state.rs`**: Defines `AppMode`, `FrameInput` (capturing pointer, multi-touch, scroll, and keyboard events from `egui::Context`), `CanvasState`, `UiState`, and `EngineState`.
- **`gui.rs` & `ui_*.rs`**: Handles layout orchestration, toolbars, properties panels, and standalone menus like Settings, Global Library Manager, or Credits (`egui`). Centralized `apply_library_index_map()` and `delete_library_chip()` helpers keep all `SubChip` references synchronized across the main canvas, stashed blueprints, undo/redo history stacks, and nested library blueprints.
- **`drawing.rs`, `drawing_shapes.rs`, & `drawing_wires.rs`**: Renders anti-aliased components, 7-segment displays, and Manhattan wires with bridge arcs onto `egui::Painter`. Both component and wire passes use `SpatialHashGrid` for $O(K)$ Viewport Culling, and wire crossing detection runs strictly on visible frustum-culled wires.
- **`inspection_logic.rs` & `inspection_ui.rs`**: Cycle-safe driver tracing (`trace_local_driver_visited`) and read-only sub-chip interior visualization.
- **`canvas.rs`**: State-preserving compilation snapshots active `Input`/`Clock` states, clock counters, and `InstanceTree` gate states before rebuilding the simulator graph, ensuring live circuit memory is preserved across interactive edits.
