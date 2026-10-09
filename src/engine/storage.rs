use super::types::GateType;
use fixedbitset::FixedBitSet;

/// Sentinel indicating no connection (equivalent to Option::None)
pub const NO_SOURCE: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub struct SoAGateStorage {
    // Hot simulation data (evaluated in tight loops across cache lines)
    pub states: Vec<u8>, // 0b00 = Floating, 0b01 = Low, 0b10 = High, 0b11 = Contention
    pub in_queue: FixedBitSet, // 1 bit per gate: true if gate is currently in event_queue
    pub depths: Vec<u32>, // Topological depth (Tarjan SCC depth)

    // Cold topology data
    pub gate_types: Vec<GateType>, // Gate primitive type
    pub sources: Vec<[u32; 2]>, // Input A and Input B source gate indices (NO_SOURCE if unconnected)
    pub dependents: Vec<Vec<u32>>, // Target gate indices depending on this gate's output

    // Allocation tracking
    pub allocated: FixedBitSet, // 1 bit per slot: true if active, false if freed
    pub free_list: Vec<u32>,    // Reusable slot indices
    len: usize,                 // Count of actively allocated gates
}

impl Default for SoAGateStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SoAGateStorage {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            states: Vec::with_capacity(capacity),
            in_queue: FixedBitSet::with_capacity(capacity),
            depths: Vec::with_capacity(capacity),
            gate_types: Vec::with_capacity(capacity),
            sources: Vec::with_capacity(capacity),
            dependents: Vec::with_capacity(capacity),
            allocated: FixedBitSet::with_capacity(capacity),
            free_list: Vec::new(),
            len: 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.states.len()
    }

    #[inline(always)]
    pub fn contains(&self, idx: usize) -> bool {
        idx < self.states.len() && self.allocated.contains(idx)
    }

    pub fn insert(&mut self, gate_type: GateType, initial_state: u8) -> usize {
        if let Some(recycled_idx) = self.free_list.pop() {
            let idx = recycled_idx as usize;
            self.states[idx] = initial_state;
            self.gate_types[idx] = gate_type;
            self.sources[idx] = [NO_SOURCE, NO_SOURCE];
            self.dependents[idx].clear();
            self.depths[idx] = 0;
            self.in_queue.set(idx, false);
            self.allocated.insert(idx);
            self.len += 1;
            idx
        } else {
            let idx = self.states.len();
            self.states.push(initial_state);
            self.gate_types.push(gate_type);
            self.sources.push([NO_SOURCE, NO_SOURCE]);
            self.dependents.push(Vec::new());
            self.depths.push(0);

            if idx >= self.in_queue.len() {
                self.in_queue.grow((idx + 64).max(self.in_queue.len() * 2));
            }
            if idx >= self.allocated.len() {
                self.allocated
                    .grow((idx + 64).max(self.allocated.len() * 2));
            }

            self.in_queue.set(idx, false);
            self.allocated.insert(idx);
            self.len += 1;
            idx
        }
    }

    pub fn remove(&mut self, idx: usize) {
        if !self.contains(idx) {
            return;
        }

        self.allocated.set(idx, false);
        self.in_queue.set(idx, false);
        self.free_list.push(idx as u32);
        self.dependents[idx].clear();
        self.sources[idx] = [NO_SOURCE, NO_SOURCE];
        self.states[idx] = 0b00;
        self.len -= 1;
    }

    pub fn clear(&mut self) {
        self.states.clear();
        self.in_queue.clear();
        self.depths.clear();
        self.gate_types.clear();
        self.sources.clear();
        self.dependents.clear();
        self.allocated.clear();
        self.free_list.clear();
        self.len = 0;
    }

    pub fn connect(&mut self, src_idx: usize, tgt_idx: usize, port: u8) {
        assert!(
            self.contains(src_idx),
            "Source gate index out of bounds: {}",
            src_idx
        );
        assert!(
            self.contains(tgt_idx),
            "Target gate index out of bounds: {}",
            tgt_idx
        );

        if port == 0 {
            self.sources[tgt_idx][0] = src_idx as u32;
        } else {
            self.sources[tgt_idx][1] = src_idx as u32;
        }

        self.dependents[src_idx].push(tgt_idx as u32);
    }

    #[inline(always)]
    pub fn depth(&self, idx: usize) -> u32 {
        self.depths[idx]
    }

    #[inline(always)]
    pub fn set_depth(&mut self, idx: usize, depth: u32) {
        self.depths[idx] = depth;
    }
}
