# Design Philosophy

The primary objective of the Digital Logic Simulator is to exceed the performance of traditional, object-oriented simulators (such as those built in Unity/C#) by an order of magnitude. This allows for the real-time simulation of complex, deeply nested computer architectures, such as a fully functional 16-bit CPU, without frame rate drops.

## The Problem with OOP in Logic Simulators
Many logic simulators use an Object-Oriented approach where every gate and wire is an instance of a class.
- State is passed between objects via virtual method calls or interface implementations (e.g., `gate.Update(signal)`).
- When chips are packaged as "Sub-Chips" and nested (e.g., an Adder inside an ALU inside a CPU), the simulator often traverses a tree of objects at runtime, mapping inputs and outputs dynamically.
- This creates massive overhead, memory fragmentation, and cache misses, severely bottlenecking simulation speed.

## Our Solution: Data-Oriented Design

### 1. The NAND Core & Dominant-Low 4-State Logic
At the mathematical base of the simulator, all combinational logic resolves down to a single operator: the NAND gate. Inputs and Outputs act as state injection and observation points (a Clock compiles to an Input that the editor toggles), while `TriStateBuffer` and `BusResolver` primitives exist to model shared buses:
- **IEEE-Style 4-State Logic**: Signal values are represented as 2-bit states: `0b00` Floating, `0b01` Low, `0b10` High, and `0b11` Contention.
- **Dominant-Low NAND Semantics**: Any `0b01` Low input forces a NAND gate's output to `0b10` High, even if the other input carries `0b11` Contention. If an input is `0b11` Contention and the other is `0b10` High, the output propagates `0b11` Contention. Unconnected or floating inputs (`0b00`) yield to active signals.
- **Synthesized Bus Resolvers**: When multiple drivers attach to a single net, synthesized `BusResolver` primitives merge them via bitwise OR (`val_a | val_b`): Floating yields transparently, while simultaneous High and Low drivers cleanly produce Contention (`0b11`).
- **Zero-Allocation Hot Loops**: Queue-driven event propagation in `Simulator::propagate_events()` swaps reusable scratch buffers (`scratch_queue`, `scratch_updates`, `scratch_next_enqueues`) rather than allocating and freeing vectors on each depth layer.

### 2. Compact Struct-of-Arrays (SoA) Storage
Instead of heap-allocated gate objects that point at each other or bloated structs, the `Simulator` uses a cache-aligned Struct-of-Arrays (`SoAGateStorage`) layout:
- `states: Vec<u8>`: Hot 4-state simulation signals (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). Contiguously packed in memory lines so multi-threaded Rayon workers saturate CPU L1/L2 caches without false sharing or cache eviction.
- `gate_types: Vec<GateType>`: Primitive gate enum definition.
- `sources: Vec<[u32; 2]>`: Input source driver indices (`u32::MAX` for unconnected inputs).
- `dependents: Vec<Vec<u32>>`: Dynamic forward adjacency lists with deduplicated target indices to prevent duplicate queue enqueuing.
- `in_queue: FixedBitSet`: Bit-packed set tracking pending queue membership (1 bit per gate).
- `disconnect_input(tgt_gate, slot)`: In-place disconnection of topological nets that surgical updates require during incremental editing without full graph reconstruction.

Evaluating a NAND gate in the inner loop is an ultra-fast register operation:
```rust
let val_a = if src_a != NO_SOURCE { states[src_a as usize] } else { 0b00 };
let val_b = if src_b != NO_SOURCE { states[src_b as usize] } else { 0b00 };
let new_state = match (val_a, val_b) {
    (0b01, _) | (_, 0b01) => 0b10,
    (0b11, _) | (_, 0b11) => 0b11,
    (0b10, 0b10) => 0b01,
    _ => 0b00,
};
```
This drops memory footprint to **14.16 bytes per gate** (100,000 gates consume only 1.38 MB), easily fitting massive circuits into host L2/L3 caches.

### 3. Flat Compilation & $O(\Delta)$ Incremental Delta-Compilation
When a user builds a complex chip (like an ALU) from smaller sub-chips (like Adders), and places that ALU inside a CPU, the simulator *does not retain this nested hierarchy at runtime*.
- **Flattening Compiler**: The `Compiler` resolves absolute primitive pathways during the compilation phase, flattening intermediate subchip boundaries into a contiguous primitive gate array.
- **Subchip Template Caching**: Rather than repeatedly flattening identical subchips, `SubchipTemplate` pre-compiles prototype topologies. Instantiating a subchip is a fast memory clone with index base offsets, eliminating multi-second UI freezing when placing complex subchips on the canvas. Nesting depth has **zero runtime cost**.
- **$O(\Delta)$ Incremental Delta-Compilation**: Interactive canvas edits (placing/deleting wires, toggling/inserting primitive gates, or placing new subchip instances) execute `compile_incremental()`. Existing SubChip instances retain their exact SoA gate allocations and internal memory latch states. Only the delta components and modified nets are patched into `SoAGateStorage`, reducing canvas edit latency on million-gate designs to under 1 millisecond.

### 4. Activity-Gated Subchip Hibernation & Topological Detection
In a 64 KB RAM module (524,288 latches), only 32 bits (one word) change state per CPU cycle. Iterating through millions of dormant gates across global event queues exhausts CPU cache bandwidth.
- **Sleep Domains (`SleepDomain`)**: When a subchip's Chip-Select or Write-Enable line is inactive, its entire gate set enters hibernation. Its internal latch states are packed into dense bit-arrays (1 bit per latch, shrinking 64 KB RAM to 64 KB of memory), and all internal gate events are skipped.
- **$O(1)$ Sleep-Control Bitset**: `sleep_control_gates: FixedBitSet` tracks control gates driving sleep domains. On state transitions, the simulator performs an $O(1)$ bit test; non-control transitions bypass `sleep_domains` lookups entirely.
- **Topological Fan-Out Asymmetry**: Rather than forcing users to follow strict pin naming conventions (`"EN"`, `"CS"`), the compiler inspects circuit topology using Tarjan's SCC to discover bistable feedback cores (latches) and identifies control lines by fan-out asymmetry ($2N$ fan-out for enable lines vs $1$ for data lines).
- **Combinational Immunity**: Purely combinational circuits (Adders, Multiplexers, Decoders) have zero feedback cycles and are mathematically immune to false sleep domains.

### 5. High-Throughput Native Vector Rendering
The editor frontend is rendered natively through `eframe` and `egui::Painter`:
- **Batched Polyline Drawing**: Uninterrupted Manhattan wire paths are emitted as single contiguous `egui::Shape::line` strokes, eliminating thousands of intermediate corner circle tessellations.
- **Zoom Level-of-Detail (LOD)**: When zoomed out (`zoom < 0.38`) or when viewing dense schematics (>2,500 visible wires), crossing-arc searches are automatically bypassed to maintain high frame rates.
- **Monotonic Ribbon Routing**: Corridors of parallel multi-bit bus wires are sorted by Y-interval overlap and net origin, ordering upward and downward branches monotonically to eliminate crossover braids.
- **Obstacle-Aware Wire Detours**: 5-segment Manhattan routes automatically calculate horizontal bypass channels above or below intermediate components, preventing wires from cutting through chip bodies.

### 6. Error-Resilient Compilation Boundaries
While the runtime simulation loop relies on unchecked direct indexing for raw throughput, the compilation and deserialization boundaries are strictly defensive:
- **Static $O(V + E)$ Blueprint Validation**: Chip libraries are verified upfront using a 3-color DFS cycle detector and pin-bounds check on project load, catching recursion and index corruption without allocating throwaway simulation gates.
- **Oscillation Recovery**: Linear evaluation limits terminate runaway zero-delay loops, and event queues are cleanly flushed to restore normal simulation on the next tick.
- **Defensive Error Handling**: Compilation errors return informative `Result<..., String>` diagnostics to the editor rather than crashing the process.
