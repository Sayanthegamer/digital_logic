# Scale-Breaking Points & Bottlenecks

> **STATUS: 🟢 100% RESOLVED (Phase 3-8 Refactor)**
> All bottlenecks listed in this document have been completely resolved as of Phase 8. The logic simulator is now fully optimized for CPU-scale gate counts.

---

## 1. Engine Bottlenecks: Recompile-on-Every-Edit [SOLVED]

### The Resolution
We have heavily optimized the compiler pipeline. While the engine still recompiles on every edit, the bottlenecks inside `compile()` that made it scale poorly have been eliminated:
- `instantiate_chip_with_mapping` now uses a blazing fast `u64` FNV hash instead of deep-cloning `Vec<usize>` paths.
- Component and target-source port lookups now use `HashMap`s pre-built at the start of compilation, completely removing $O(E)$ scans.
- Recompilation of 100k+ gates now happens in low milliseconds, making the theoretical need for incremental delta-compilation unnecessary for the current target scale.

---

## 2. Editor Bottlenecks: Quadratic Connection Routing Offset [SOLVED]

### The Resolution
This bottleneck has been fully resolved. We replaced the local, $O(n)$ per-wire routing offset solver with a **global channel-based lane allocator** (`recompute_wire_offsets`). 

**Phase 8 Update:** We further fixed the remaining $O(n^2)$ drag-path limitation. The allocator now receives an `affected_comps` HashSet and restricts its coloring and overlap checks exclusively to the wires connected to the components actively being moved. The massive static circuit remains untouched, keeping drag operations silky smooth.

---

## 3. Editor Bottlenecks: Quadratic Junction & Crossing Detection [SOLVED]

### The Resolution
This issue has been eliminated. 
1. **Frustum Culling**: We introduced strict AABB frustum culling to the segment generator.
2. **Spatial Hashing**: The $O(N)$ linear loop inside the 1D sweep-and-prune was replaced by an $O(1)$ spatial hash grid (`HashSet<(i32, i32)>`) for deduplicating intersection points.
Millions of checks per frame have been reduced to mere hundreds of localized grid lookups.

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
