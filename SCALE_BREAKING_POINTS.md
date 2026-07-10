# Scale-Breaking Points & Bottlenecks

This document details the known scaling chokepoints and performance limitations of the Digital Logic Simulator. While the simulator's core uses a highly efficient, cache-friendly Struct-of-Arrays (SoA) layout with a flattened gate graph, the editor/rendering layer and compile pipeline contain several $O(n^2)$ bottlenecks that will cause severe performance degradation at CPU-scale gate counts (e.g., 1,500–3,000+ gates, 2,000+ wires).

---

## 1. Engine Bottlenecks: Recompile-on-Every-Edit

### The Issue
Currently, editing the circuit (placing components, deleting wires, or changing connections) triggers a full recompilation of the visual graph into the simulator's flat gate array representation.
- **Complexity**: $O(V + E)$ where $V$ is the number of nested gates and $E$ is the number of connections.
- **Impact**: For small circuits, recompilation takes milliseconds. However, as the canvas approaches CPU-scale (thousands of gates, deep blueprint nesting), recompilation on every edit will introduce a noticeable lag (hiccup) in the editor UI during interactive operations.

---

## 2. Editor Bottlenecks: Quadratic Connection Routing Offset [SOLVED]

### The Resolution
This bottleneck has been fully resolved. We replaced the local, $O(n)$ per-wire routing offset solver with a **global channel-based lane allocator** (`recompute_wire_offsets`). 

Instead of computing offsets dynamically on every frame query, the solver runs once during compilation or frame updates while dragging a component or wire. It:
1. Groups all connections by spatial corridors based on their ideal vertical segment coordinates in world coordinates.
2. Checks Y-interval overlaps (with a 4px cushion) to detect conflicts.
3. Applies greedy interval coloring to assign non-conflicting lane indices (0, 1, 2, ...).
4. Maps lanes to alternating offsets (0, +12, -12, +24, -24, ...) and caches the results in a `HashMap` directly on the `Editor` struct.

### Call Sites & Complexity
The `get_connection_routing_offset` method is now a simple $O(1)$ cached map lookup:
```rust
pub fn get_connection_routing_offset(&self, conn: &VisualConnection) -> f32 {
    self.wire_offsets.get(conn).copied().unwrap_or(0.0)
}
```
This entirely eliminates frame-by-frame traversal and intermediate `HashMap` allocations in:
- The main drawing loop (`drawing.rs`).
- The intersection bridge arc renderer (`wire_junctions.rs`).
- Input event handlers (`input_interactions` submodules).

The rendering complexity has been reduced from $O(n \cdot m)$ to a clean $O(n)$ cached layout query per frame.

### Remaining Drag-Path Limitations
> [!IMPORTANT]
> While the rendering/hit-testing paths are now a fast $O(1)$ map lookup from the cache, the lane allocation pass itself (`recompute_wire_offsets`) has an $O(n^2)$ time complexity in terms of total connections (due to the pairwise Y-overlap checks required for greedy coloring).
>
> Currently, this allocator is executed unconditionally **every frame** during active component or wire drags in `input.rs`:
> ```rust
> if self.canvas.dragging_comp_id.is_some() || self.canvas.dragging_wire.is_some() {
>     self.recompute_wire_offsets();
> }
> ```
> At massive scale (thousands of wires), running this $O(n^2)$ pass 60 times a second can cause drag stutter. 
> 
> **Proposed Fix**: Throttle or localize the drag-time recomputation (e.g. only recompute lane offsets for connections connected to the dragged component's inputs and outputs rather than re-coloring the entire canvas graph).

---

## 3. Editor Bottlenecks: Quadratic Junction & Crossing Detection

### The Issue
The method `find_wire_intersections` in [wire_junctions.rs](file:///c:/Users/Anon/Desktop/logic-sim(rust)/logic_simulator/src/editor/wire_junctions.rs#L203) queries and compares every pair of segment segments across the entire canvas:
```rust
for i in 0..all_segments.len() {
    for j in (i + 1)..all_segments.len() {
        if all_segments[i].conn_idx == all_segments[j].conn_idx {
            continue;
        }
        ...
    }
}
```

### Scaling Challenges
1. **No Frustum Culling**: Unlike wire rendering, which culls wires outside the viewport, `find_wire_intersections` evaluates every wire segment on the entire canvas.
2. **Segment Multiplier**: Each visual wire consists of 3 to 5 Manhattan segments. The total number of segments is therefore $3n$ to $5n$. 
3. **Quadratic Scaling**: The nested loop compares segment pairs. For $S$ segments, it performs $\frac{S^2}{2}$ segment intersection checks. At 2,000 wires, $S \approx 8,000$, resulting in upwards of 32 million checks per frame.

---

## 4. Metadata Discrepancy: BusJoiner / BusSplitter Port Counts

### The Detail
`BusJoiner` and `BusSplitter` have a mismatch between their official port metadata and their internal representation in the compiler/canvas maps:
- **Official Interface**: `BusJoiner` is configured with `(w, 1)` ports, and `BusSplitter` with `(1, w)` ports.
- **Internal Mapping**: Both components allocate `w` inputs and `w` outputs. The outputs are mapped as `OutputSource::PassedThrough(i)`.

### Impact
This does not break compilation because the backward signal walk (`trace_root`) resolves the signals via recursive `PassedThrough` logic. However, this discrepancy means:
- The visual "bus" wire's activity color only reflects the state of bit 0 of the bus rather than evaluating the entire word.
- It can cause confusion when extending the compiler or inspector logic.

---

## 5. Junction Port Overlap & Connection Stacking [SOLVED]

### The Resolution
This layout clutter issue has been fully resolved.
1. **Perpendicular Tap Routing**: We introduced `get_connection_ports` and `get_junction_connect_pos` inside `editor/mod.rs`. Wires connected to a stretched Junction now tap perpendicularly along the closest coordinate of the Junction's body instead of routing to its endpoints. This entirely prevents horizontal/vertical overlapping segments on fan-out and fan-in wires connecting to Junction buses.
2. **Wire Deduplication**: We added a filtering rule in `input_release.rs` that cleans up matching connections (`src_comp_id`, `src_port`, `tgt_comp_id`, `tgt_port`) before pushing new ones, ensuring no duplicate wires stack on top of each other.

