# Design Philosophy

The primary objective of the Digital Logic Simulator is to exceed the performance of traditional, object-oriented simulators (such as those built in Unity/C#) by an order of magnitude. This allows for the real-time simulation of complex, deeply nested computer architectures, such as a fully functional 16-bit CPU, without frame rate drops.

## The Problem with OOP in Logic Simulators
Many logic simulators use an Object-Oriented approach where every gate and wire is an instance of a class.
- State is passed between objects via virtual method calls or interface implementations (e.g., `gate.Update(signal)`).
- When chips are packaged as "Sub-Chips" and nested (e.g., an Adder inside an ALU inside a CPU), the simulator often traverses a tree of objects at runtime, mapping inputs and outputs dynamically.
- This creates massive overhead, memory fragmentation, and cache misses, severely bottlenecking simulation speed.

## Our Solution: Data-Oriented Design

### 1. The NAND Core
At the mathematical base of the simulator, all combinational logic resolves down to a single operator: the NAND gate. Inputs and Outputs act as state injection and observation points (a Clock compiles to an Input that the editor toggles), while `TriStateBuffer` and `BusResolver` primitives exist to model shared buses. By standardizing the logic operator, the execution loop stays small and uniform. When multiple drivers connect to a shared net, synthesized `BusResolver` primitives merge signals via a bitwise OR (`val_a | val_b`): Floating (`0b00`) yields transparently to active drivers, while simultaneous High (`0b10`) and Low (`0b01`) drive produces Contention (`0b11`).

### 2. Flat, Index-Based Node Storage
Instead of heap-allocated gate objects that point at each other, the `Simulator` keeps every primitive in one `slab::Slab<GateNode>` and refers to gates by `usize` index:
- `gate: PrimitiveGate`: The gate type and its two input sources (`Option<usize>` indices).
- `state: u8`: The current 4-state signal (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention).
- `dependents: Vec<usize>`: A forward adjacency list of the gates that read this gate's output.
- `depth` and `in_queue`: Scheduling metadata for the per-depth event queues.

During simulation, evaluating a NAND gate is a couple of index lookups into that slab:
```rust
let val_a = input_a_source.map(|s| nodes[s].state).unwrap_or(0b00);
let val_b = input_b_source.map(|s| nodes[s].state).unwrap_or(0b00);
let high = |v: u8| (v & 0b10) != 0;
let new_state = if !(high(val_a) && high(val_b)) { 0b10 } else { 0b01 };
```
This is an array-of-structs layout with compact (single-byte) state, not a strict struct-of-arrays. The locality gain comes from ordering: immediately after compilation the `Simulator` re-inserts all nodes sorted by topological depth, so gates in the same layer sit next to each other when a Rayon thread picks up a chunk of the `event_queue`.

### 3. Flat Compilation Hierarchy
When a user builds a complex chip (like an ALU) from smaller sub-chips (like Adders), and then places that ALU inside a CPU, the simulator *does not retain this nested hierarchy at runtime*.
- The `Compiler` resolves the absolute primitive pathways during the "packaging" phase.
- It bypasses the abstract boundaries of sub-chips. If an input pin on the top level connects through 5 nested layers down to a specific primitive NAND gate, the compiler wires the top-level simulator directly to that specific primitive.
- As a result, nesting depth has **zero runtime cost**. A CPU built of 10,000 nested chips runs exactly as fast as 10,000 raw NAND gates laid out flat.

### 4. Error-Resilient Compilation Boundaries
While the runtime simulation loop relies on unchecked direct indexing for raw throughput, the compilation and deserialization boundaries are strictly defensive:
- Blueprints are validated upfront for out-of-bounds component indices and port counts before memory allocation.
- Compilation errors return informative `Result<..., String>` diagnostics to the editor rather than crashing the process.
- The persistence layer verifies blueprints via a sandboxed scratch simulator before committing changes to the user's permanent chip library.
