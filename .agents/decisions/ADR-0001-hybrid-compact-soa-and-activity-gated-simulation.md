# ADR-0001: Hybrid Compact SoA Bit-Slab and Activity-Gated Simulation Architecture

- **Date:** 2026-10-09
- **Status:** 🟢 Accepted
- **Decider(s):** Human Engineer & Lead Architect
- **Governance Tier:** Tier 1 (Impacted Vectors: Vector 4: Performance & Scaling, Vector 5: Memory Layout, Vector 6: Execution Model, Vector 8: Algorithmic Compilation)

---

## 1. Context & Problem Statement

The simulator's goal is to simulate a complete **32-bit CPU architecture** (ALU, registers, control decoders, program counter) alongside **kilobyte/megabyte-scale RAM and persistent storage** built **100% purely out of nested custom chips assembled from raw NAND gates**.

While the current engine (v3.2.4) easily simulates 100,000 gates at 60 FPS, scaling purely from NAND gates to 64 KB of RAM (~4,000,000 NAND gates) or 1 MB of storage (~60,000,000 NAND gates) reveals three structural breaking points:
1. **Memory Representation Density:** At ~150 bytes per gate (`Slab<GateNode>` + heap `Vec<usize>` dependents), 4M gates consume ~600 MB and 60M gates consume ~9 GB of RAM.
2. **Monolithic Recompilation:** `compile()` currently flattens the entire nested hierarchy from scratch on every canvas wire/move action. Recompiling 4M+ gates on every mouse drag causes multi-second UI freezing.
3. **The Inactivity Paradox:** In a 64 KB RAM module (524,288 latches), only 32 bits (one word) change state per CPU cycle. Iterating through millions of static gates across global event queues exhausts CPU cache bandwidth.

---

## 2. The Decision: Why This Option Won

We adopt a **Hybrid Architecture** combining:
1. **Compact Struct-of-Arrays (SoA) Bit-Slab:**
   - Decouple hot simulation data (`state: u8`, `in_queue: bool` packed in contiguous cache lines) from cold topology data (`sources`, `dependents`).
   - Reduces active gate memory footprint from ~150 bytes down to ~8–12 bytes per gate, fitting millions of gates comfortably into host L2/L3 CPU caches.
2. **Activity-Gated Subchip Hibernation (Sleep Domains):**
   - For deeply nested repetitive circuits (like RAM and register banks), the compiler identifies Chip-Select / Write-Enable activation boundaries.
   - Dormant banks are put to sleep, storing internal latch states as dense bit-arrays (1 bit per latch, shrinking 64 KB RAM from 600 MB to 64 KB of host memory).
   - Only active banks schedule gates into the primary Tarjan SCC event queues.
3. **Incremental Delta-Compilation:**
   - Caches compiled subchip instance graphs. Editing canvas wires only recompiles local dirty sub-nets rather than re-flattening intact custom chips (like completed ALUs or memory blocks).

---

## 3. Rejected Alternatives ("Why Not That?")

* **Alternative 1: Pure Flat Event-Driven Expansion (Current v3.2.4 Architecture)**
  - *Why Considered:* Simplest model; zero nesting boundaries at runtime.
  - *Why Rejected:* Prohibitive memory footprint (~9 GB for 60M gates) and multi-second compilation times during canvas dragging.
* **Alternative 2: Pure Bit-Parallel JIT Compiler (Verilator / Dynarec Engine)**
  - *Why Considered:* Blazing execution speed (10–50 MHz CPU clocks) by generating straight-line 64-bit SIMD assembly.
  - *Why Rejected:* Breaks live interactive editing (requires an explicit heavy JIT recompile step before running), introduces external JIT backend dependencies (Dynasm/Cranelift), and complicates interactive "Look Inside" signal probing.

---

## 4. Conscious Trade-offs (What We Sacrificed)

- **Engine State Complexity:** Introducing sleep boundaries and dormant state compaction adds domain-gating logic compared to a purely uniform flat array.
- **Compiler Boundary Analysis:** The compiler must perform static analysis to detect gating inputs (e.g., Chip Select / Enable lines) to guarantee dormant subchips do not miss asynchronous signals.

---

## 5. Revisit Trigger (When to Change Your Mind)

Revisit this architecture if:
1. The user demands CPU clock frequencies exceeding 10 MHz (at which point a JIT / SIMD bit-parallel compilation pass becomes mandatory).
2. The user moves to hardware-accelerated GPU compute shaders (e.g. WebGPU / Vulkan compute pipelines) for multi-million gate simulation.
