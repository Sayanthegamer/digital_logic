# System Specifications

This document outlines the operational specifications of the logic engine.

## Primitives

Every signal is a 4-state value stored as a `u8`: `0b00` Floating, `0b01` Low, `0b10` High, `0b11` Contention. An unconnected input reads as Floating (`0b00`).

The simulator core (`GateType`) understands five primitive gates:

1. **Nand**: The universal logic gate. Only the High bit of each input counts, so Floating and Low both read as false. Output is `0b10` (High) unless both inputs are High, in which case it is `0b01` (Low). An unconnected NAND therefore evaluates to High. Initial state: High.
2. **Input**: A source of logic level (High/Low) driven by user interaction, outer-chip connections, or a compiled Clock. Initial state: Low.
3. **Output**: A sink that passes along the full 4-state value of its driving source (input A). Initial state: Low.
4. **TriStateBuffer**: Input A is data and input B is enable. When enabled it drives High or Low according to the data input; when disabled it outputs Floating (`0b00`). Initial state: Floating.
5. **BusResolver**: Merges two drivers by bitwise OR of their states. Floating yields to the other driver, and a High driver meeting a Low driver produces Contention (`0b11`). The compiler inserts these automatically when several drivers feed the same net. Initial state: Floating.

At the editor/blueprint level (`ComponentType`) there are also **Clock**, an autonomous component that flips after a localized number of simulation ticks (its `period`) and compiles to an Input gate, plus display and bus-routing components (`SevenSegment`, `Junction`, `BusJoiner`, `BusSplitter`) and `SubChip` references.

## Event Propagation

- The simulator uses a `Vec<Vec<usize>>` (`event_queue`) to queue primitive indices that require evaluation, one queue per topological depth. Depths come from Tarjan's SCC algorithm followed by longest-path layering over the condensation DAG, so every gate in a feedback loop shares one depth.
- When an Input state changes (or a Clock ticks), its immediate dependents are pushed to the queue. A gate is queued at most once at a time (`in_queue`).
- **Parallel Evaluation**: During `propagate_events`, the engine loops through the depth layers in order. For each layer it first computes all new states from the current node states, then applies the changes and enqueues dependents of every gate whose state changed. When the layer's queue is at least `dynamic_threshold` long (calibrated once per process by a hardware profiler), the compute pass runs on `rayon::par_iter()`; otherwise it runs sequentially. Results are collected in order, so processing is deterministic. If a change re-enqueues a gate at an earlier depth, the events are picked up once the loop runs past the last non-empty layer.
- **Oscillation Detection**: To prevent infinite loops caused by zero-delay feedback loops (e.g., an inverter connected to itself), `propagate_events(max_steps_multiplier)` computes a budget of `node_slab_capacity × max(multiplier, 100)` gate evaluations, counted cumulatively across the whole propagation pass. The editor passes `max(10 × node_count, 1000)` as the multiplier. If the total exceeds the budget, it returns an `Oscillation detected` error, halting the loop and displaying an error in the UI.

## Custom Chips (Sub-Chips)

Custom chips are stored in a blueprint `library`. A `ChipBlueprint` consists of:
- `inputs`: Number of external input ports.
- `outputs`: Number of external output ports.
- `components`: A list of internals (primitives such as Nands and Clocks, display and bus-routing components, or nested Sub-chips referencing other blueprints).
- `connections`: Abstract links between component ports.

When instantiated:
- The compiler traces connections backward from targets to their absolute "root driver" (a primitive gate).
- Ports mapped merely to pass-through a signal (Input -> Output) are mathematically resolved without allocating a physical "buffer" gate in the array.
- If several drivers feed the same internal port, the compiler synthesizes a `BusResolver` tree so High/Low conflicts surface as Contention instead of one driver silently overwriting the other.

## Clocks and Multi-Domain Timing

- **Tick Base**: The simulator does not rely on real-time rendering frames to dictate logic time. Logic time is advanced explicitly by a "tick" loop in the editor (`editor/input_simulation.rs`), not by the engine.
- **Multi-Domain**: Different clocks can have different periods (default 20 ticks when unset). The compiler registers each Clock in an `active_clocks` list as a `CompiledClock` pointing at its Input gate. On each tick the editor increments a localized counter per clock. When a counter reaches `max(period / 2, 1)`, it resets, flips the clock's Input state and enqueues its dependents.
- This allows complex sequences (like a CPU clock running faster than a peripheral display clock) to function synchronously within the same flat data array.
