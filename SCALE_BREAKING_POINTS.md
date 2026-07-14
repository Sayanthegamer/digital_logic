# Scale-Breaking Points & Bottlenecks

> **STATUS: 🟢 RESOLVED (Phase 3-9 Refactor)**
> All major bottlenecks identified in the engine and editor have been resolved with O(1) or O(N) optimized replacements, enabling 100k+ gate scale without structural freezing or stutter.

---

## 1. Engine Bottlenecks: Recompile-on-Every-Edit [SOLVED]

### The Resolution
We have heavily optimized the compiler pipeline. While the engine still recompiles on every edit, the bottlenecks inside `compile()` that made it scale poorly have been eliminated:
- `instantiate_chip_with_mapping` now uses a blazing fast `u64` FNV hash instead of deep-cloning `Vec<usize>` paths.
- Component and target-source port lookups now use `HashMap`s pre-built at the start of compilation, completely removing $O(E)$ scans.
- Recompilation of 100k+ gates now happens in low milliseconds, making the theoretical need for incremental delta-compilation unnecessary for the current target scale.

---

## 2. Engine Bottlenecks: Single-Threaded Event Loop [SOLVED]

### The Resolution
The simulator historically suffered from thread-pool overhead and data race constraints which confined event propagation to a single core. This bottleneck has been completely bypassed:
- **Intelligent Hardware Profiler**: A dynamic runtime calibrator tests the host machine on startup to calculate the exact Rayon crossover threshold where parallelization beats single-threaded execution. 
- **Topological Map-Reduce**: Gates inside the same topological depth layer are safely evaluated concurrently via `rayon::par_iter()`. This provides extreme throughput for massive circuits without encountering data races or breaking memory safety (the ABA slab issue was also patched to clear the queue on topology changes).

## 2. Editor Bottlenecks: Wire Routing & Crossings [100% RESOLVED]

### The Resolution
All O(n²) bottlenecks related to wire rendering and routing have been eliminated:
- **Rendering Stutter (`recompute_wire_offsets`):** Replaced O(N²) greedy lane coloring with an O(N) Spatial Hash Grid. The 5,000 connection hard limit has been completely removed.
- **Rendering Stutter (Wire Crossings):** Replaced the 1D sweep-and-prune worst-case O(N²) algorithm with a full O(N) 2D Spatial Hash Grid for the primary intersection search.
- **Rendering Stutter (Wire Gaps/Arcs):** Replaced rigid, loop-heavy arc rendering and masking with a mathematically precise 1D interval hole merging algorithm ($O(K \log K)$) on individual wire segments, completely eliminating visual clipping, overlapping wavy artifacts, and heavy `Vec` allocations during the rendering cycle.
- **Drag Lane Collisions:** Modified `recompute_wire_offsets` to check dragged dynamic wires directly against the untouched static wires via spatial hash bucketing (`(col, row)` keys), making conflict checking O(1) amortized per segment.

---

## 4. Metadata Discrepancy: BusJoiner / BusSplitter Port Counts [SOLVED]

### The Resolution
This was resolved during the Phase 5 decoupling. We added a dedicated `bus_width` attribute to `Component` and `VisualComponent`. Port counts are now correctly centralized into a shared `get_port_counts()` method, mapping directly to the true hardware constraints without hacky workarounds.

---

## 5. Junction Port Overlap & Connection Stacking [SOLVED]

### The Resolution
This layout clutter issue has been fully resolved.
1. **Perpendicular Tap Routing**: We introduced `get_connection_ports` and `get_junction_connect_pos` inside `editor/mod.rs`. Wires connected to a stretched Junction now tap perpendicularly along the closest coordinate of the Junction's body instead of routing to its endpoints. This entirely prevents horizontal/vertical overlapping segments on fan-out and fan-in wires connecting to Junction buses.
2. **Wire Deduplication**: We added a filtering rule in `input_release.rs` that cleans up matching connections (`src_comp_id`, `src_port`, `tgt_comp_id`, `tgt_port`) before pushing new ones, ensuring no duplicate wires stack on top of each other.
