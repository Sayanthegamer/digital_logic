# Scale-Breaking Points & Bottlenecks

This document details the known scaling chokepoints and performance limitations of the Digital Logic Simulator. While the simulator's core uses a highly efficient, cache-friendly Struct-of-Arrays (SoA) layout with a flattened gate graph, the editor/rendering layer and compile pipeline contain several $O(n^2)$ bottlenecks that will cause severe performance degradation at CPU-scale gate counts (e.g., 1,500–3,000+ gates, 2,000+ wires).

---

## 1. Engine Bottlenecks: Recompile-on-Every-Edit

### The Issue
Currently, editing the circuit (placing components, deleting wires, or changing connections) triggers a full recompilation of the visual graph into the simulator's flat gate array representation.
- **Complexity**: $O(V + E)$ where $V$ is the number of nested gates and $E$ is the number of connections.
- **Impact**: For small circuits, recompilation takes milliseconds. However, as the canvas approaches CPU-scale (thousands of gates, deep blueprint nesting), recompilation on every edit will introduce a noticeable lag (hiccup) in the editor UI during interactive operations.

---

## 2. Editor Bottlenecks: Quadratic Connection Routing Offset

### The Issue
The connection routing offset function `get_connection_routing_offset` in [wire_junctions.rs](file:///c:/Users/Anon/Desktop/logic-sim(rust)/logic_simulator/src/editor/wire_junctions.rs#L69) is $O(n)$ per call:
```rust
pub fn get_connection_routing_offset(&self, conn: &VisualConnection) -> f32 {
    let mut sharing: Vec<&VisualConnection> = self.connections.iter()
        .filter(|c| c.src_comp_id == conn.src_comp_id && c.src_port == conn.src_port)
        .collect();
    ...
}
```
Every time a connection is queried, it scans all active visual connections. 
Furthermore, if multiple wires share a source port, it builds a `HashMap` mapping all components by ID:
```rust
let comp_by_id: std::collections::HashMap<usize, &VisualComponent> =
    self.components.iter().map(|c| (c.id, c)).collect();
```
This hashmap allocation and traversal happens **on every single invocation** of a shared port query.

### Call Sites & Quadratic Behavior
This function is called:
- Once per wire in the main drawing loop (`drawing.rs`).
- Once per wire segment inside `find_wire_intersections` (`wire_junctions.rs`).
- In multiple places inside input event handler loops (`input.rs`).

Because this is executed once per wire, every single frame, it leads to $O(n \cdot m)$ complexity where $n$ is the number of connections and $m$ is the number of components. At CPU-scale layout sizes (2,000+ wires), this leads to millions of operations and frequent hashmap reallocations, severely impacting the frame rate.

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
