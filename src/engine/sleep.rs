use super::Simulator;
use fixedbitset::FixedBitSet;

#[derive(Debug, Clone)]
pub struct SleepDomain {
    pub id: usize,
    pub control_gate: usize,
    pub active_high: bool, // true: High (0b10) = Awake, Low/Floating = Asleep
    pub gates: Vec<usize>,
    pub latch_indices: Vec<usize>,
    pub dormant_bits: FixedBitSet,
    pub is_sleeping: bool,
}

impl SleepDomain {
    pub fn new(
        id: usize,
        control_gate: usize,
        active_high: bool,
        gates: Vec<usize>,
        latch_indices: Vec<usize>,
    ) -> Self {
        let num_latches = latch_indices.len();
        Self {
            id,
            control_gate,
            active_high,
            gates,
            latch_indices,
            dormant_bits: FixedBitSet::with_capacity(num_latches),
            is_sleeping: false,
        }
    }

    /// Packs active 4-state latch signals into a dense 1-bit-per-latch bitset buffer
    pub fn hibernate(&mut self, sim: &mut Simulator) {
        if self.is_sleeping {
            return;
        }

        self.dormant_bits.grow(self.latch_indices.len());
        self.dormant_bits.clear();

        for (bit_idx, &latch_idx) in self.latch_indices.iter().enumerate() {
            if latch_idx < sim.nodes.states.len() {
                let is_high = (sim.nodes.states[latch_idx] & 0b10) != 0;
                if is_high {
                    self.dormant_bits.insert(bit_idx);
                }
            }
        }

        // Mark internal gates as sleeping
        for &gate_idx in &self.gates {
            if gate_idx < sim.nodes.states.len() {
                sim.sleeping_gates.grow(gate_idx + 1);
                sim.sleeping_gates.insert(gate_idx);
            }
        }

        self.is_sleeping = true;
    }

    /// Wakes the domain and unpacks dense bit buffers back into active simulation states
    pub fn wake(&mut self, sim: &mut Simulator) {
        if !self.is_sleeping {
            return;
        }

        // Clear sleeping status from internal gates
        for &gate_idx in &self.gates {
            if gate_idx < sim.sleeping_gates.len() {
                sim.sleeping_gates.set(gate_idx, false);
            }
        }

        // Restore latch states
        for (bit_idx, &latch_idx) in self.latch_indices.iter().enumerate() {
            if latch_idx < sim.nodes.states.len() {
                let restored_state = if self.dormant_bits.contains(bit_idx) {
                    0b10 // High
                } else {
                    0b01 // Low
                };
                sim.nodes.states[latch_idx] = restored_state;
            }
        }

        // Enqueue all gates in domain to re-evaluate with current inputs
        for &gate_idx in &self.gates {
            sim.enqueue_internal(gate_idx);
        }

        self.is_sleeping = false;
    }

    #[inline(always)]
    pub fn should_sleep(&self, ctrl_state: u8) -> bool {
        let is_active = (ctrl_state & 0b10) != 0;
        if self.active_high {
            !is_active
        } else {
            is_active
        }
    }
}
