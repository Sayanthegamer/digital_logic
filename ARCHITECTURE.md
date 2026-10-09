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

### Flat Compilation
The most critical architectural decision for performance is how custom chips (sub-chips) are handled.
- In many visual simulators, nested chips result in tree-walking or virtual function calls at runtime.
- In this project, the `Compiler` natively *flattens* the hierarchy during instantiation.
- Deeply nested components (e.g., CPU -> ALU -> Adder -> XOR -> NAND) are unwrapped. The compiler wires the raw primitive gates (NAND, Input, Output, TriStateBuffer, and `BusResolver` where several drivers share a net) directly to each other. A `Clock` component compiles to an `Input` gate that the editor's tick loop toggles.
- At runtime, the `Simulator` only sees a single flat `Slab<GateNode>` of primitive gates. Each `GateNode` holds the gate type and its two input sources, a `u8` 4-state signal value, its `dependents` list, an `in_queue` flag and its `depth`. Gates are addressed by plain `usize` index, so lookups are O(1).
- **Safe Compilation Boundary**: `instantiate_chip_with_mapping` enforces upfront index and port bounds checking across all connections prior to compilation. Malformed topologies return structured `Result::Err` errors rather than panicking.
- **Cache Defragmentation**: After flattening, `defragment_and_sort_by_depth` runs an $O(N \log N)$ sort and re-inserts every node into a fresh slab in depth order, remapping all source, dependent and queued indices (it returns early if the slab is already compact and sorted). Gates of the same depth end up in a contiguous index range, which improves locality when a layer is evaluated.

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
