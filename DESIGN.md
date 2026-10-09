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

### 2. Compact Struct-of-Arrays (SoA) Storage
Instead of heap-allocated gate objects that point at each other or bloated structs, the `Simulator` uses a cache-aligned Struct-of-Arrays (`GateStorage`) layout:
- `states: Vec<u8>`: Hot 4-state simulation signal (`0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention). Contiguously packed in memory lines so multi-threaded Rayon workers saturate CPU L1/L2 caches without false sharing or cache eviction.
- `gate_types: Vec<GateType>`: Primitive gate enum definition.
- `sources: Vec<[u32; 2]>`: Input source driver indices (`u32::MAX` for unconnected inputs).
- `dependents: Vec<Vec<u32>>`: Dynamic forward adjacency lists.
- `in_queue: FixedBitSet`: Bit-packed set tracking pending queue membership (1 bit per gate).

During simulation, evaluating a NAND gate is an ultra-fast lookup:
```rust
let val_a = if src_a != NO_SOURCE { states[src_a as usize] } else { 0b00 };
let val_b = if src_b != NO_SOURCE { states[src_b as usize] } else { 0b00 };
let high = |v: u8| (v & 0b10) != 0;
let new_state = if !(high(val_a) && high(val_b)) { 0b10 } else { 0b01 };
```
This reduces active memory footprint to **14.16 bytes per gate** (100,000 gates consume only 1.38 MB), easily fitting massive circuits into host L2/L3 caches.

### 3. Flat Compilation Hierarchy & Subchip Templates
When a user builds a complex chip (like an ALU) from smaller sub-chips (like Adders), and then places that ALU inside a CPU, the simulator *does not retain this nested hierarchy at runtime*.
- The `Compiler` resolves the absolute primitive pathways during the "packaging" phase.
- It bypasses the abstract boundaries of sub-chips. If an input pin on the top level connects through 5 nested layers down to a specific primitive NAND gate, the compiler wires the top-level simulator directly to that specific primitive.
- **Subchip Template Caching**: Rather than repeatedly flattening identical subchips, `SubchipTemplate` pre-compiles prototype topologies. Instantiating a subchip is a fast memory clone with index base offsets, eliminating multi-second UI freezing when placing complex subchips on the canvas.
- As a result, nesting depth has **zero runtime cost**. A CPU built of 10,000 nested chips runs exactly as fast as 10,000 raw NAND gates laid out flat.

### 4. Activity-Gated Subchip Hibernation & Topological Detection
In a 64 KB RAM module (524,288 latches), only 32 bits (one word) change state per CPU cycle. Iterating through millions of dormant gates across global event queues exhausts CPU cache bandwidth.
- **Sleep Domains (`SleepDomain`)**: When a subchip's Chip-Select or Write-Enable line is inactive, its entire gate set enters hibernation. Its internal latch states are packed into dense bit-arrays (1 bit per latch, shrinking 64 KB RAM to 64 KB of memory), and all internal gate events are skipped.
- **Topological Fan-Out Asymmetry**: Rather than forcing users to follow strict pin naming conventions (`"EN"`, `"CS"`), the compiler inspects circuit topology using Tarjan's SCC to discover bistable feedback cores (latches) and identifies control lines by fan-out asymmetry ($2N$ fan-out for enable lines vs $1$ for data lines).
- **Combinational Immunity**: Purely combinational circuits (Adders, Multiplexers, Decoders) have zero feedback cycles and are mathematically immune to false sleep domains.

### 5. Error-Resilient Compilation Boundaries
While the runtime simulation loop relies on unchecked direct indexing for raw throughput, the compilation and deserialization boundaries are strictly defensive:
- Blueprints are validated upfront for out-of-bounds component indices and port counts before memory allocation.
- Compilation errors return informative `Result<..., String>` diagnostics to the editor rather than crashing the process.
- The persistence layer verifies blueprints via a sandboxed scratch simulator before committing changes to the user's permanent chip library.
