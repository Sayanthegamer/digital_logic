# Design Philosophy

The primary objective of the Digital Logic Simulator is to exceed the performance of traditional, object-oriented simulators (such as those built in Unity/C#) by an order of magnitude. This allows for the real-time simulation of complex, deeply nested computer architectures, such as a fully functional 16-bit CPU, without frame rate drops.

## The Problem with OOP in Logic Simulators
Many logic simulators use an Object-Oriented approach where every gate and wire is an instance of a class.
- State is passed between objects via virtual method calls or interface implementations (e.g., `gate.Update(signal)`).
- When chips are packaged as "Sub-Chips" and nested (e.g., an Adder inside an ALU inside a CPU), the simulator often traverses a tree of objects at runtime, mapping inputs and outputs dynamically.
- This creates massive overhead, memory fragmentation, and cache misses, severely bottlenecking simulation speed.

## Our Solution: Data-Oriented Design

### 1. The NAND Core
At the mathematical base of the simulator, all complex logic resolves down to a single primitive: the NAND gate. (Inputs, Outputs, and Clocks act as state injection points rather than logic operators). By standardizing the logic operator, the execution loop is incredibly simple and uniform.

### 2. Struct of Arrays (SoA)
Instead of an array of objects `[{type, state, connections}, ...]`, the `Simulator` manages separate, contiguous arrays:
- `states: Vec<bool>`: The current binary state of every primitive.
- `gates: Vec<PrimitiveGate>`: The topology (type and input sources).
- `dependents: Vec<Vec<usize>>`: A forward-mapped adjacency list showing which gates rely on a specific gate's output.

During simulation, evaluating a NAND gate is essentially a few pointer jumps in a contiguous memory block:
```rust
let val_a = states[gate.input_a];
let val_b = states[gate.input_b];
states[idx] = !(val_a && val_b);
```
This data layout is extremely friendly to CPU caches.

### 3. Flat Compilation Hierarchy
When a user builds a complex chip (like an ALU) from smaller sub-chips (like Adders), and then places that ALU inside a CPU, the simulator *does not retain this nested hierarchy at runtime*.
- The `Compiler` resolves the absolute primitive pathways during the "packaging" phase.
- It bypasses the abstract boundaries of sub-chips. If an input pin on the top level connects through 5 nested layers down to a specific primitive NAND gate, the compiler wires the top-level simulator directly to that specific primitive.
- As a result, nesting depth has **zero runtime cost**. A CPU built of 10,000 nested chips runs exactly as fast as 10,000 raw NAND gates laid out flat.
