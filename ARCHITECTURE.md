# Architecture

The Digital Logic Simulator is designed for maximum performance, actively avoiding Object-Oriented Programming (OOP) bottlenecks. The architecture is divided into two primary subsystems: the **Core Engine** and the **Editor UI**.

## Core Engine (`src/engine/`)

The simulation backend is completely decoupled from the UI. It operates on a flat, cache-friendly data structure.

### Event-Driven Simulation
Instead of a naive tick-based evaluation where every gate is processed every frame, the `Simulator` uses an event-driven queue (`event_queue`).
1. When an input changes, only the gates directly dependent on that input are queued for re-evaluation.
2. The `propagate_events` loop processes this queue, propagating state changes forward.
3. This prevents unnecessary calculations, allowing the simulator to handle massive circuits smoothly.

### Flat Compilation
The most critical architectural decision for performance is how custom chips (sub-chips) are handled.
- In many visual simulators, nested chips result in tree-walking or virtual function calls at runtime.
- In this project, the `Compiler` natively *flattens* the hierarchy during instantiation.
- Deeply nested components (e.g., CPU -> ALU -> Adder -> XOR -> NAND) are unwrapped. The compiler wires the raw primitive gates (NAND, Input, Output) directly to each other.
- At runtime, the `Simulator` only sees a single, flat array of primitive gates, ensuring continuous memory layout (`Vec<bool>`, `Vec<usize>`) and O(1) index lookups.

## Editor UI (`src/editor/`)

The frontend combines two rendering paradigms:
1. **Macroquad**: Used for the 2D logic canvas (rendering wires, components, and the grid). It provides high-performance, low-level rendering capabilities.
2. **egui**: Used for the immediate-mode graphical user interface (menus, inspection panels, sidebars). It is integrated via `egui-macroquad` to render on top of the Macroquad canvas.

### Separation of Concerns
The Editor UI has been modularized to ensure high maintainability and structured application state:
- **`state.rs`**: Defines `AppMode` enum that routes execution between the Main Menu, Editor, and other configuration overlays.
- **`gui.rs` & `ui_*.rs`**: Handles layout orchestration, toolbars, properties panels, and standalone menus like Settings or Credits (egui).
- **`drawing.rs` & `drawing_*.rs`**: Handles primitive math, shape rendering, and routing of manhattan wires (Macroquad).
- **`inspection_logic.rs` & `inspection_ui.rs`**: Handles tracing states deep inside sub-chips and visualizing them in a read-only overlay.

### Separation of State
The `Editor` struct maintains the visual state (positions, zooming, tool selection) and interacts with the `Simulator`. Visual components are mapped to underlying simulation indices (`port_to_sim_gate_map`, `visual_to_sim_map`), meaning the UI is just a "viewer" and "controller" for the high-performance core, never dragging down the simulation speed.
