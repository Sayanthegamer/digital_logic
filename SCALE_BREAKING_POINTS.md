# Scale-Breaking Points & Bottlenecks

> **STATUS: 🟢 RESOLVED (Phase 3-10 Architecture)**
> All major bottlenecks across the simulation engine, schematic wire router, delta compiler, and vector renderer have been resolved with $O(1)$ or $O(\Delta)$ replacements, enabling 100k–1M+ gate scale and 10k+ wire schematics without freezing or stutter.

---

## 1. Engine Bottlenecks: Recompile-on-Every-Edit & Incremental Delta-Compilation [SOLVED]

### The Resolution
1. **O(Δ) Incremental Delta-Compilation (`Editor::compile_incremental`)**:
   - Canvas wiring changes, primitive gate additions/removals (`Nand`, `Input`, `Output`, `Clock`, `TriStateBuffer`, `SevenSegment`, `Junction`, `BusJoiner`, `BusSplitter`), and new `SubChip` placements now patch `SoAGateStorage` in place without tearing down or reallocating existing `SubChip` instances.
   - Only top-level component input ports (`Simulator::disconnect_input`) and top-level `BusResolver` / `SleepDomain` bindings (`Simulator::truncate_sleep_domains`) are re-linked, reducing canvas edit latency on 1,000,000-gate circuits from ~250 ms to sub-millisecond ($O(\text{top-level ports})$).
2. **Subchip Template Caching (`SubchipTemplate`)**:
   - Subchips are compiled into immutable `SubchipTemplate` prototypes (`compile_subchip_template`) and instantiated via contiguous base-offset arithmetic.
3. **State-Preserving Full Recompilation (`Editor::compile_full`)**:
   - When a full defragmenting compile is required (e.g., after deleting a `SubChip` or modifying a library blueprint), running `Input`, `Clock`, and internal `InstanceTree` latch states are snapshotted and restored across topological defragmentation.

---

## 2. Engine Bottlenecks: Zero-Allocation Multi-Threaded Event Loop [SOLVED]

### The Resolution
1. **Zero-Allocation Hot Loop (`0 mallocs / tick`)**:
   - Eliminated per-event `self.nodes.dependents[idx].clone()` heap allocations in `Simulator::propagate_events`, `Simulator::set_input`, and `Simulator::remove_gate` in favor of indexed iteration.
   - Replaced `std::mem::take(&mut self.event_queue[depth])` buffer dropping with persistent `scratch_queue`, `scratch_updates`, and `scratch_next_enqueues` buffers swapped back into `event_queue[depth]`, eliminating allocator churn during high-frequency clock ticks.
2. **Intelligent Hardware Profiler & Topological Map-Reduce**:
   - Dynamic startup calibration (`OnceLock`) determines the exact `rayon::par_iter()` crossover threshold for deterministic parallel layer evaluation.
3. **Clean Oscillation Recovery**:
   - Exceeding the linear $O(N)$ oscillation budget drains `event_queue` and clears `in_queue` flags cleanly so disabling an oscillating feedback loop recovers immediately.

---

## 3. Engine Bottlenecks: Memory Density & L1/L2 Cache Locality [SOLVED]

### The Resolution
- **Struct-of-Arrays (SoA) Bit-Slab (`SoAGateStorage`)**: Separates hot simulation states (`states: Vec<u8>`) from topology (`gate_types`, `sources: Vec<[u32; 2]>`, `dependents`) and bit-packed `in_queue: FixedBitSet` (~14.16 bytes/gate).
- **Defragmentation & Topological Sorting**: Packs gates contiguously by topological depth (`defragment_and_sort_by_depth`) and remaps `SleepDomain` gate ranges, control lines, `sleeping_gates`, and `sleep_control_gates` bitsets.

---

## 4. Editor Bottlenecks: Wire Routing, Obstacle Detours, & Extreme-Scale Polylines [SOLVED]

### The Resolution
1. **Crossing-Free Ribbon Lane Routing (`recompute_wire_offsets`)**:
   - Groups vertical wire trunks by corridor and Y-interval overlap, shares vertical lanes across same-source fan-out nets `(src_comp_id, src_port)`, and orders downward (`dy >= 0`) and upward (`dy < 0`) buses monotonically so parallel bus L-bends nest with zero self-crossings.
2. **Obstacle-Aware Orthogonal Detours (`wire_detours`)**:
   - Automatically detects when a forward 3-segment wire would slice through an intermediate component's bounding box and routes a 5-segment orthogonal bypass channel above or below the obstacle(s).
3. **Orthogonal Crossing Arcs & T-Junction Solder Dots**:
   - Eliminated 45° corner chamfers that previously caused false triple bridge-arc loops at wire bends; restricted bridge arcs strictly to interior horizontal-over-vertical crossings and added solder dots on shared-net T-junctions.
4. **Batched `egui::Shape::line` Polylines & Multi-Tier Zoom LOD (`draw_manhattan_wire`)**:
   - Uninterrupted wires and zoomed-out schematics (`zoom < 0.38`) render via a single batched `egui::Shape::line` polyline per wire (skipping per-segment circle tessellation and sub-pixel intersection searches when zoomed out or viewing $>2,500$ simultaneous wires).

---

## 5. Static $O(V + E)$ Library Validation & Folder-Safe Imports [SOLVED]

### The Resolution
1. **Static 3-Color DFS Validator (`Simulator::validate_library`)**:
   - Validates blueprint libraries for recursion cycles and port bounds in $O(V + E)$ time with zero gate allocations when loading or importing projects.
2. **Folder-Safe Global Library Indexing (`GlobalLibrary::import_from_project`)**:
   - Correctly shifts flat indices of folder blueprints (`idx >= old_ungrouped_len`) when importing new chips into `ungrouped`, preserving all `SubChip` references across nested blueprints and canvas components.

---

## 6. Activity-Gated Sleep Domains & $O(1)$ Control Gating [SOLVED]

### The Resolution
1. **$O(1)$ `sleep_control_gates: FixedBitSet` Filter**:
   - Gate state transitions check a single bit in `sleep_control_gates` before scanning `sleep_domains`, reducing sleep-domain overhead on normal gate transitions to $O(1)$.
2. **Automated Tarjan SCC Latch Discovery**:
   - Automatically identifies bistable feedback latch cores and high-fan-out enable/chip-select lines so dormant memory banks hibernate in 1-bit-per-latch compacted arrays.

---

## 7. Native `eframe` + `egui` Desktop & `NativeActivity` Android Architecture [SOLVED]

### The Resolution
- **Unified `eframe` 0.31 Rendering**: Pure `egui::Painter` vector rendering with native `egui::Align2` typography and $O(K)$ spatial-hash viewport culling.
- **Android `NativeActivity` Bridge**: Replaced the legacy Macroquad `QuadSurface` + `QuadNative.java` JNI wrapper with an `android.app.NativeActivity` host (`MainActivity.java` and `AndroidManifest.xml` `android.app.lib_name` metadata) compatible with `eframe`/`winit`.
