# System Specifications

This document outlines the operational specifications of the logic engine and schematic editor.

## Primitives & 4-State Signal Semantics

Every signal is an IEEE-style 4-state value stored as a `u8`:
- `0b00`: **Floating** (high-impedance / unconnected)
- `0b01`: **Low** (logic 0 / ground)
- `0b10`: **High** (logic 1 / VCC)
- `0b11`: **Contention** (electrical conflict / bus fight)

An unconnected input pin defaults to Floating (`0b00`).

The simulator core (`GateType`) understands five primitive gates:

1. **Nand**: The universal logic gate.
   - **Dominant-Low Semantics**: A `0b01` Low on either input forces the output to `0b10` High, regardless of whether the opposing input is `0b00` Floating or `0b11` Contention.
   - **Contention Propagation**: If one input is `0b11` Contention and the other is `0b10` High, the gate propagates `0b11` Contention.
   - **Standard Evaluation**: If both inputs are `0b10` High, the output is `0b01` Low. If inputs are Floating without Low, the gate evaluates to `0b00` Floating.
   - **Initial State**: `0b10` High.
2. **Input**: A source of logic level driven by user interaction, outer-chip connections, or an autonomous Clock. Initial state: `0b01` Low.
3. **Output**: A sink that reflects the full 4-state value of its driving source (input A). Initial state: `0b01` Low.
4. **TriStateBuffer**: Input A is data, and input B is enable.
   - When enabled (`0b10`), it drives the 4-state value of input A.
   - When disabled or floating (`0b01` or `0b00`), it outputs `0b00` Floating.
   - When enable is in contention (`0b11`), it drives `0b11` Contention.
   - Initial state: `0b00` Floating.
5. **BusResolver**: Merges multiple drivers on a shared electrical net via bitwise OR (`val_a | val_b`):
   - Floating (`0b00`) yields transparently to active drivers.
   - Concurrent High (`0b10`) and Low (`0b01`) drivers produce Contention (`0b11`).
   - The compiler automatically synthesizes `BusResolver` trees whenever multiple drivers attach to the same net. Initial state: `0b00` Floating.

At the editor and blueprint level (`ComponentType`), additional components exist:
- **Clock**: Autonomous pulse generator with configurable period. Compiles down to an Input gate.
- **SevenSegment**: Multi-segment visual indicator decoding 4-state bus inputs.
- **Junction**: Electrical distribution hub that transparently forwards 1-bit and multi-bit signals across branches without slicing or data corruption.
- **BusJoiner & BusSplitter**: Bundle 2 to 16 individual single-bit lines into unified multi-bit buses and vice versa.
- **SubChip**: Reusable, hierarchical custom chip packaging a complete sub-circuit.

## Event Propagation, Activity-Gating & Recovery

- **Topological Depth Queuing**: The simulator maintains per-depth event queues (`event_queue: Vec<Vec<usize>>`). Gate depths are calculated using Tarjan's SCC condensation algorithm followed by longest-path DAG layering, ensuring all gates within a feedback loop (such as an SR latch) evaluate within the same depth step.
- **Zero-Allocation Hot Loops**: `propagate_events()` operates with zero heap allocations per tick. Scratch buffers (`scratch_queue`, `scratch_updates`, `scratch_next_enqueues`) are reused across depth layers. Dependent iterations use direct indexed slices rather than allocating cloned vectors.
- **Parallel vs Sequential Execution**: When a depth layer holds fewer gates than `dynamic_threshold`, it evaluates sequentially on a single thread. When it meets or exceeds `dynamic_threshold`, Pass 1 computes states in parallel across CPU cores using `rayon::par_iter()`. Pass 2 applies states sequentially and schedules dependents.
- **$O(1)$ Sleep-Domain Gating**: Control gates driving sleep domains are registered in `sleep_control_gates: FixedBitSet`. Gates transitioning state perform an $O(1)$ bit test; non-control transitions skip sleep domain scanning entirely.
- **Activity-Gated Hibernation (`SleepDomain`)**: When a domain's control gate becomes inactive, internal bistable latches are compacted into dense bit-arrays (`FixedBitSet`), and internal gates are skipped during propagation. On re-activation, latches are decompressed and queued for evaluation.
- **Oscillation Cap & Clean Recovery**: To prevent infinite loops caused by zero-delay feedback loops (e.g. an inverter connected to itself), propagation enforces a linear budget: $\text{capacity} \times \text{multiplier}$ (defaulting to 100). If the step count exceeds this budget, propagation halts, returns an `Oscillation detected` error, and cleanly drains all queues and bitsets to allow immediate recovery on the next input edit.

## Compilation, Delta-Compilation & Templates

### 1. Hierarchical Blueprint Structure
Custom chips are saved as `ChipBlueprint` instances containing:
- `name`: Identifier string.
- `inputs`, `outputs`: External port counts.
- `input_names`, `output_names`: Pin label descriptors.
- `components`: Internal primitive, bus, or nested `SubChip` components.
- `connections`: Topological port linkages.

### 2. Static $O(V + E)$ Blueprint Validation
Before loading or compilation:
- `Simulator::validate_library()` runs 3-color DFS cycle detection on the chip library to detect circular blueprint hierarchies.
- Validates that all connection source/target component indices and port numbers fall strictly within declared blueprint interfaces, rejecting malformed blueprints before allocation.

### 3. $O(\Delta)$ Incremental Delta-Compilation
During interactive schematic editing:
- `Editor::compile()` distinguishes between full rebuilds (`compile_full()`) and incremental updates (`compile_incremental()`).
- When the user adds/removes wires, places/deletes primitive components, or places new `SubChip` instances, the compiler preserves existing `SubChip` instances in `SoAGateStorage` along with their internal latch/register states.
- Topological rewiring uses `SoAGateStorage::disconnect_input()` to clear obsolete connections in-place without invalidating gate indices, keeping canvas edit latency under 1 millisecond on million-gate circuits.

### 4. Template Caching
- `compile_subchip_template()` pre-compiles blueprints into reusable `SubchipTemplate` structures.
- Instantiations on canvas clone templates via base offset indexing, bypassing recursive flattening during placement.

### 5. Topological Auto-Gating
- `find_gating_input()` identifies Chip-Select and Write-Enable pins without relying on pin names.
- Analyzes Tarjan SCC feedback loops to locate latch cores and detects control lines via fan-out asymmetry ($2N$ vs $1$). Combinational circuits (Adders, Multiplexers) return `None`, guaranteeing zero false sleep domains.

## Wire Routing & Schematic Rendering

- **Monotonic Ribbon Routing**: Corridor-based routing sorts vertical wire trunks by net origin and Y-overlap, ordering upward and downward branches monotonically to eliminate crossover braids on multi-bit buses.
- **Obstacle-Aware Detours**: 5-segment Manhattan paths calculate clear horizontal bypass channels above or below intermediate components when a straight connection would slice through a chip body.
- **Batched Polylines**: Uninterrupted Manhattan segments are drawn using native `egui::Shape::line` polylines, minimizing GPU draw calls and eliminating corner circle tessellations.
- **Zoom Level-of-Detail (LOD)**: When `zoom < 0.38` or visible wires exceed 2,500, expensive bridge-arc intersection checks are bypassed.
- **Viewport Culling**: `SpatialHashGrid` restricts rendering and intersection calculations to visible viewport bounds.
