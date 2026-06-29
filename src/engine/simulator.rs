use super::types::*;
use std::collections::VecDeque;

pub struct Simulator {
    pub gates: Vec<PrimitiveGate>,
    pub states: Vec<bool>,
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

        // Floating inputs default to false (Logic 0).
        // A NAND gate with floating inputs evaluates to !(false && false) = true.
        // Input and Output gates default to false.
        let initial_state = match gate_type {
            GateType::Nand => true,
            GateType::Input | GateType::Output => false,
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

        if self.states[gate_idx] != value {
            self.states[gate_idx] = value;
            let deps = self.dependents[gate_idx].clone();
            for dep_idx in deps {
                self.enqueue(dep_idx);
            }
        }
    }

    /// Gets the current state of a gate by its index.
    pub fn get_state(&self, gate_idx: usize) -> bool {
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
    /// Floating inputs default to false.
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
                .unwrap_or(false);
            let val_b = gate
                .input_b_source
                .map(|s_idx| self.states[s_idx])
                .unwrap_or(false);

            let new_state = match gate.gate_type {
                GateType::Input => self.states[idx],
                GateType::Output => val_a,
                GateType::Nand => !(val_a && val_b),
            };

            if self.states[idx] != new_state {
                self.states[idx] = new_state;

                let deps = self.dependents[idx].clone();
                for dep_idx in deps {
                    self.enqueue(dep_idx);
                }
            }

            steps += 1;
        }

        Ok(steps)
    }
}
