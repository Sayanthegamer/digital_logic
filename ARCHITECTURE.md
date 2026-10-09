# Architecture

The Digital Logic Simulator is designed for maximum performance, actively avoiding Object-Oriented Programming (OOP) bottlenecks. The architecture is divided into two primary subsystems: the **Core Engine** and the **Editor UI**.

## Core Engine (`src/engine/`)

The simulation backend is completely decoupled from the UI. It operates on a flat, cache-friendly data structure.

### Event-Driven Simulation
Instead of a naive tick-based evaluation where every gate is processed every frame, the `Simulator` uses an event-driven queue (`event_queue`).
1. **Topological Depth Layering (Tarjan SCC)**: `calculate_depths()` runs Tarjan's algorithm to find strongly connected components, collapses each one (feedback structures such as latches become a single node) into a condensation DAG, and assigns every component its longest-path depth (`depth(v) = max(depth(u) + 1)` over its predecessors). All gates in one SCC share a depth. Pending work is held in per-depth queues (`event_queue: Vec<Vec<usize>>`), so execution order is deterministic.
2. When an input changes, only the gates directly dependent on that input are queued for re-evaluation.
3. The `propagate_events` loop walks the depth layers in order and processes each layer in **two passes**. Pass 1 computes each queued gate's new state from a read-only view of the node array. When the layer's queue length reaches a hardware-calibrated threshold (`dynamic_threshold`), this runs on `rayon::par_iter()` across all cores; smaller layers run sequentially. Pass 2 applies the changed states sequentially and enqueues the dependents of every gate that changed. Because pass 1 only reads and pass 2 only writes, there are no data races, and results are identical whether or not the layer ran in parallel. If a changed gate re-enqueues a gate at an earlier depth (a feedback loop), the events are picked up once the loop runs past the last non-empty layer.
4. This prevents unnecessary calculations and is what lets the simulator target 100,000+ gate CPUs in real time. Parallelism only helps when a layer is wide; long narrow chains (e.g. a ripple-carry adder) are evaluated layer by layer on one thread.
5. **Hardware Profiler & Safe Fallback**: The dynamic crossover threshold is calibrated once at startup via `detect_parallel_crossover_threshold()` and cached in a `OnceLock`. If parallel evaluation fails to beat sequential execution up to 16,000 gates (such as on single-core or constrained mobile devices), the threshold defaults to `usize::MAX`, safely disabling parallel thread dispatch to prevent performance regressions.
6. **Oscillation Budget Scaling**: The evaluation step cap scales linearly as $\text{capacity} \times \text{multiplier}$ (defaulting to 100), preventing quadratic execution explosions on large circuits while terminating genuine zero-delay feedback loops in bounded time.

### Compact Struct-of-Arrays (SoA) Node Storage (`src/engine/storage.rs`)
The simulator stores gates in a high-density, cache-aligned Struct-of-Arrays (`GateStorage`) structure:
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
- When inactive (`should_sleep()`), the domain enters hibernation: its latch states are compacted into dense bit-arrays (`latch_bits: FixedBitSet`, 1 bit per latch), and all internal gates are removed from active event queues.
- When awakened, `wake()` restores the latch states and schedules gates for normal evaluation.

### Automatic Topological Sleep-Gating Detection (`find_gating_input`)
The compiler automatically identifies Chip-Select / Enable control lines even when circuits have arbitrary or default pin names:
1. **Nominal Fast-Path**: Matches explicit pin labels (`"cs"`, `"en"`, `"enable"`, `"we"`, etc.).
2. **Subchip Gating Fan-Out**: Scores inputs by the number of nested subchip gating ports they drive.
3. **Primitive Latch Core SCC Analysis**: Uses Tarjan's SCC algorithm on raw components to discover bistable feedback latch cores and scores inputs by their fan-out to the gating stages ($2N$ fan-out for enable vs $1$ for data).
4. **Combinational Immunity**: Purely combinational circuits have zero feedback cycles and return `None`, guaranteeing zero false-positive sleep domains.

### Safe Compilation & Cache Defragmentation
- **Safe Compilation Boundary**: `instantiate_chip_with_mapping` enforces upfront index and port bounds checking across all connections prior to compilation. Malformed topologies return structured `Result::Err` errors rather than panicking.
- **Topological Sorting & Dynamic Remapping**: After flattening, `defragment_and_sort_by_depth` sorts gates strictly by topological depth and packs them contiguously. It simultaneously remaps all `domain.gates`, `domain.control_gate`, and `domain.latch_indices` across the reordering, maximizing memory locality and hardware prefetcher saturation.

## Editor UI (`src/editor/`)

The frontend combines two rendering paradigms:
1. **Macroquad**: Used for the 2D logic canvas (rendering wires, components, and the grid). It provides high-performance, low-level rendering capabilities.
2. **egui**: Used for the immediate-mode graphical user interface (menus, inspection panels, sidebars). It is integrated via `egui-macroquad` to render on top of the Macroquad canvas.

### Separation of Concerns
The Editor UI has been modularized to ensure high maintainability and structured application state:
- **`state.rs`**: Defines `AppMode` enum that routes execution between the Main Menu, Editor, and other configuration overlays.
- **`gui.rs` & `ui_*.rs`**: Handles layout orchestration, toolbars, properties panels, and standalone menus like Settings or Credits (egui).
- **`drawing.rs` & `drawing_*.rs`**: Handles primitive math, shape rendering, and routing of manhattan wires (Macroquad). Drawing loops utilize a `SpatialHashGrid` for $O(K)$ Viewport Culling, ensuring the GPU only ever renders components currently visible on-screen. Wire routing offsets are resolved globally via a channel-based lane allocator and cached, enabling $O(1)$ lookup complexity in the render path.
- **`inspection_logic.rs` & `inspection_ui.rs`**: Handles tracing states deep inside sub-chips and visualizing them in a read-only overlay.
- **`input.rs` & `input_*.rs`**: Handles user input events (touch camera controls, keyboard shortcuts, magnetic port hovering, context menu targets, and canvas left-click drag-and-drop state machines). To optimize maintainability, these are structured as a slim coordinator (`input.rs`) dispatching to separate submodules representing distinct interaction phases (`input_press.rs`, `input_down.rs`, `input_release.rs`, `input_delete.rs`, `input_keyboard.rs`, `input_navigation.rs`, `input_hover.rs`, `input_simulation.rs`, `input_context_menu.rs`, `input_interactions.rs`).
- **`persistence.rs`**: Handles `.logic` / `.json` project save/load. Before importing project chips into global storage or overriding canvas state, `load_project()` executes a pre-validation pass using a scratch simulator to guarantee blueprint integrity.

### Separation of State
The `Editor` struct maintains the visual state (positions, zooming, tool selection) and interacts with the `Simulator`. Visual components are mapped to underlying simulation indices (`port_to_sim_gate_map`, `visual_to_sim_map`), meaning the UI is just a "viewer" and "controller" for the high-performance core, never dragging down the simulation speed.
