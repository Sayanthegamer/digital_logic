# System Specifications

This document outlines the operational specifications of the logic engine.

## Primitives

The core simulation understands only a limited set of primitive gate types (`GateType` / `ComponentType`):

1. **Nand**: The universal logic gate. Evaluates as `!(A && B)`. Floating inputs default to `false`. Therefore, an unconnected NAND evaluates to `true`.
2. **Input**: A source of logic level (True/False) driven by user interaction or outer-chip connections.
3. **Output**: A sink that displays or passes along the state of its driving source.
4. **Clock**: An autonomous component that flips its binary state after a localized number of simulation ticks (its `period`).

## Event Propagation

- The simulator uses a `Vec<usize>` layered structure to queue primitive indices that require evaluation, segregated strictly by their topological depth.
- When an Input state changes (or a Clock ticks), its immediate dependents are pushed to the queue.
- **Parallel Evaluation**: During `propagate_events`, the engine loops through each topological depth layer sequentially. Inside a single depth layer, all queued gates are evaluated concurrently via `rayon::par_iter()`. A runtime calibrator automatically determines if the queue size at that specific depth is large enough to warrant thread-pool overhead vs sequential execution.
- **Oscillation Detection**: To prevent infinite loops caused by zero-delay feedback loops (e.g., an inverter connected to itself), the `propagate_events` function accepts a `max_steps` limit. If the queue processes more events than this limit in a single propagation phase, it throws an `Oscillation detected` error, halting the loop and displaying an error in the UI.

## Custom Chips (Sub-Chips)

Custom chips are stored in a blueprint `library`. A `ChipBlueprint` consists of:
- `inputs`: Number of external input ports.
- `outputs`: Number of external output ports.
- `components`: A list of internals (Nands, Clocks, or nested Sub-chips referencing other blueprints).
- `connections`: Abstract links between component ports.

When instantiated:
- The compiler traces connections backward from targets to their absolute "root driver" (a primitive gate).
- Ports mapped merely to pass-through a signal (Input -> Output) are mathematically resolved without allocating a physical "buffer" gate in the array.

## Clocks and Multi-Domain Timing

- **Tick Base**: The simulator does not rely on real-time rendering frames to dictate logic time. Logic time is advanced explicitly by a "tick" loop in the editor logic.
- **Multi-Domain**: Different clocks can have different periods. The system maintains an `active_clocks` registry. During each engine update, it increments a localized counter for each clock. When a counter reaches `period / 2`, it flips the clock's logic state and enqueues its dependents.
- This allows complex sequences (like a CPU clock running faster than a peripheral display clock) to function synchronously within the same flat data array.
