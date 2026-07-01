use super::types::*;
use std::collections::VecDeque;

pub struct Simulator {
    pub gates: Vec<PrimitiveGate>,
    pub states: Vec<u8>,
    pub dependents: Vec<Vec<usize>>,
    pub event_queue: VecDeque<usize>,
    pub in_queue: Vec<bool>,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            states: Vec::new(),
            dependents: Vec::new(),
            event_queue: VecDeque::new(),
            in_queue: Vec::new(),
        }
    }

    /// Adds a gate of a specific type to the simulator.
    /// Inputs are initially set to None (floating).
    /// Returns the unique index of the added gate.
    pub fn add_gate(&mut self, gate_type: GateType) -> usize {
        let index = self.gates.len();
        self.gates.push(PrimitiveGate {
            gate_type,
            input_a_source: None,
            input_b_source: None,
        });

        // 0b00 = Floating, 0b01 = Low, 0b10 = High, 0b11 = Contention
        let initial_state = match gate_type {
            GateType::Nand => 0b10,
            GateType::Input | GateType::Output => 0b01,
            GateType::TriStateBuffer => 0b00,
            GateType::BusResolver => 0b00,
        };

        self.states.push(initial_state);
        self.dependents.push(Vec::new());
        self.in_queue.push(false);

        // Queue the newly created gate for initial propagation check
        self.enqueue(index);

        index
    }

    /// Connects the output of source_idx to target_idx on the specified port.
    /// port is 0 for input_a_source, 1 for input_b_source.
    pub fn connect(&mut self, source_idx: usize, target_idx: usize, port: u8) {
        assert!(
            source_idx < self.gates.len(),
            "Source index out of bounds: {}",
            source_idx
        );
        assert!(
            target_idx < self.gates.len(),
            "Target index out of bounds: {}",
            target_idx
        );

        let target_gate = &mut self.gates[target_idx];
        match port {
            0 => target_gate.input_a_source = Some(source_idx),
            1 => target_gate.input_b_source = Some(source_idx),
            _ => panic!("Invalid port: {}. Must be 0 or 1.", port),
        }

        self.dependents[source_idx].push(target_idx);

        // Queue the target gate because its connection just changed
        self.enqueue(target_idx);
    }

    /// Sets the value of an Input gate.
    /// If the state changes, enqueues all dependents for evaluation.
    pub fn set_input(&mut self, gate_idx: usize, value: bool) {
        assert!(
            gate_idx < self.gates.len(),
            "Gate index out of bounds: {}",
            gate_idx
        );
        assert!(
            self.gates[gate_idx].gate_type == GateType::Input,
            "Cannot set input on a non-Input gate: {:?}",
            self.gates[gate_idx].gate_type
        );

        let new_state = if value { 0b10 } else { 0b01 };
        if self.states[gate_idx] != new_state {
            self.states[gate_idx] = new_state;
            for &dep_idx in &self.dependents[gate_idx] {
                if !self.in_queue[dep_idx] {
                    self.in_queue[dep_idx] = true;
                    self.event_queue.push_back(dep_idx);
                }
            }
        }
    }

    /// Gets the boolean evaluation of the current state of a gate by its index.
    pub fn get_state(&self, gate_idx: usize) -> bool {
        assert!(
            gate_idx < self.states.len(),
            "Gate index out of bounds: {}",
            gate_idx
        );
        (self.states[gate_idx] & 0b10) != 0
    }

    /// Gets the raw 2-bit state of a gate by its index (00=Float, 01=Low, 10=High, 11=Contention).
    pub fn get_raw_state(&self, gate_idx: usize) -> u8 {
        assert!(
            gate_idx < self.states.len(),
            "Gate index out of bounds: {}",
            gate_idx
        );
        self.states[gate_idx]
    }

    /// Internal helper to push a gate to the event queue if it isn't already there.
    fn enqueue(&mut self, idx: usize) {
        if !self.in_queue[idx] {
            self.in_queue[idx] = true;
            self.event_queue.push_back(idx);
        }
    }

    /// Internal helper to pop a gate from the event queue.
    fn dequeue(&mut self) -> Option<usize> {
        if let Some(idx) = self.event_queue.pop_front() {
            self.in_queue[idx] = false;
            Some(idx)
        } else {
            None
        }
    }

    /// Propagates events through the network using the event queue.
    /// Floating inputs default to 00.
    /// Returns the number of events processed or an Err if it exceeds max_steps (oscillation).
    pub fn propagate_events(&mut self, max_steps: usize) -> Result<usize, String> {
        let mut steps = 0;

        while let Some(idx) = self.dequeue() {
            if steps >= max_steps {
                return Err(format!(
                    "Oscillation detected: exceeded max_steps limit of {}",
                    max_steps
                ));
            }

            let gate = &self.gates[idx];
            let val_a = gate
                .input_a_source
                .map(|s_idx| self.states[s_idx])
                .unwrap_or(0b00);
            let val_b = gate
                .input_b_source
                .map(|s_idx| self.states[s_idx])
                .unwrap_or(0b00);

            let new_state = match gate.gate_type {
                GateType::Input => self.states[idx],
                GateType::Output => val_a,
                GateType::Nand => {
                    let a_bool = (val_a & 0b10) != 0;
                    let b_bool = (val_b & 0b10) != 0;
                    if !(a_bool && b_bool) { 0b10 } else { 0b01 }
                }
                GateType::TriStateBuffer => {
                    // a = Data, b = Enable
                    let en_bool = (val_b & 0b10) != 0;
                    if en_bool {
                        let data_bool = (val_a & 0b10) != 0;
                        if data_bool { 0b10 } else { 0b01 }
                    } else {
                        0b00 // Floating
                    }
                }
                GateType::BusResolver => {
                    val_a | val_b // The bitwise magic!
                }
            };

            if self.states[idx] != new_state {
                self.states[idx] = new_state;

                for &dep_idx in &self.dependents[idx] {
                    if !self.in_queue[dep_idx] {
                        self.in_queue[dep_idx] = true;
                        self.event_queue.push_back(dep_idx);
                    }
                }
            }

            steps += 1;
        }

        Ok(steps)
    }
}
