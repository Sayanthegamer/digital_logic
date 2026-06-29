use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GateType {
    Nand,
    Input,
    Output,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrimitiveGate {
    pub gate_type: GateType,
    pub input_a_source: Option<usize>,
    pub input_b_source: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComponentType {
    Nand,
    Input,
    Output,
    Clock,
    SubChip(usize), // Index of the sub-chip blueprint in the library/registry
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Component {
    pub component_type: ComponentType,
    pub pos: (f32, f32), // Visual layout position for inspection mode
    pub clock_period: Option<usize>, // Localized period in ticks (only for ComponentType::Clock)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SourcePort {
    ChipInput(usize), // The i-th input of the custom chip itself
    ComponentOutput { component_idx: usize, port_idx: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TargetPort {
    ChipOutput(usize), // The j-th output of the custom chip itself
    ComponentInput { component_idx: usize, port_idx: usize },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    pub source: SourcePort,
    pub target: TargetPort,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChipBlueprint {
    pub name: String,
    pub inputs: usize,
    pub outputs: usize,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputSource {
    DrivenByGate(usize),  // Driven by a primitive gate index
    PassedThrough(usize), // Driven directly by outer chip input port index
    Floating,             // Not connected internally
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstantiatedInterface {
    pub inputs: Vec<Vec<(usize, u8)>>, // Outer chip input index -> primitive targets (gate_idx, port)
    pub outputs: Vec<OutputSource>,    // Outer chip output index -> its driver
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TraceNode {
    ChipInput(usize),
    ChipOutput(usize),
    CompInput { component_idx: usize, port_idx: usize },
    CompOutput { component_idx: usize, port_idx: usize },
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompiledClock {
    pub gate_idx: usize,
    pub period: usize,
    pub counter: usize,
    pub visual_id: Option<usize>, // Top-level visual component ID if mapped
}

pub struct Simulator {
    pub gates: Vec<PrimitiveGate>,
    pub states: Vec<bool>,
    pub dependents: Vec<Vec<usize>>,
    pub event_queue: VecDeque<usize>,
    pub in_queue: Vec<bool>,
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
        assert!(source_idx < self.gates.len(), "Source index out of bounds: {}", source_idx);
        assert!(target_idx < self.gates.len(), "Target index out of bounds: {}", target_idx);

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
        assert!(gate_idx < self.gates.len(), "Gate index out of bounds: {}", gate_idx);
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
        assert!(gate_idx < self.states.len(), "Gate index out of bounds: {}", gate_idx);
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
            let val_a = gate.input_a_source.map(|s_idx| self.states[s_idx]).unwrap_or(false);
            let val_b = gate.input_b_source.map(|s_idx| self.states[s_idx]).unwrap_or(false);

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

    /// Instantiates a custom chip from the blueprint library and records mapping of instances to compiled gate indices.
    pub fn instantiate_chip_with_mapping(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
        path: &[usize],
        instance_map: &mut std::collections::HashMap<(Vec<usize>, usize), usize>,
        instance_outputs: &mut std::collections::HashMap<(Vec<usize>, usize), Vec<OutputSource>>,
        active_clocks: &mut Vec<CompiledClock>,
    ) -> Result<InstantiatedInterface, String> {
        let blueprint = library.get(blueprint_idx)
            .ok_or_else(|| format!("Blueprint not found at index {}", blueprint_idx))?;

        // 1. Instantiate all internal components
        let mut component_ports: Vec<(Vec<Vec<(usize, u8)>>, Vec<OutputSource>)> = Vec::new();

        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            match &component.component_type {
                ComponentType::Nand => {
                    let nand_idx = self.add_gate(GateType::Nand);
                    // Record visual component instance mapping
                    instance_map.insert((path.to_vec(), comp_idx), nand_idx);
                    
                    component_ports.push((
                        vec![vec![(nand_idx, 0)], vec![(nand_idx, 1)]],
                        vec![OutputSource::DrivenByGate(nand_idx)],
                    ));
                }
                ComponentType::Clock => {
                    let clock_idx = self.add_gate(GateType::Input);
                    // Record visual component instance mapping
                    instance_map.insert((path.to_vec(), comp_idx), clock_idx);
                    
                    let period = component.clock_period.unwrap_or(20);
                    active_clocks.push(CompiledClock {
                        gate_idx: clock_idx,
                        period,
                        counter: 0,
                        visual_id: None, // Will be linked at the top-level compile
                    });

                    component_ports.push((
                        vec![], // Clocks have no input ports
                        vec![OutputSource::DrivenByGate(clock_idx)],
                    ));
                }
                ComponentType::SubChip(sub_idx) => {
                    // Recursively compile sub-chip with sub-path
                    let mut sub_path = path.to_vec();
                    sub_path.push(comp_idx);
                    let sub_interface = self.instantiate_chip_with_mapping(*sub_idx, library, &sub_path, instance_map, instance_outputs, active_clocks)?;
                    instance_outputs.insert((path.to_vec(), comp_idx), sub_interface.outputs.clone());
                    component_ports.push((sub_interface.inputs, sub_interface.outputs));
                }
                ComponentType::Input | ComponentType::Output => {
                    return Err("Blueprint components cannot contain top-level Input or Output internally".to_string());
                }
            }
        }

        // Helper closures to avoid duplicate recursive functions and cleanly capture state.
        let get_immediate_source = |node: &TraceNode| -> Option<TraceNode> {
            match node {
                TraceNode::ChipOutput(out_idx) => {
                    blueprint.connections.iter()
                        .find(|conn| conn.target == TargetPort::ChipOutput(*out_idx))
                        .map(|conn| match conn.source {
                            SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                            SourcePort::ComponentOutput { component_idx, port_idx } => {
                                TraceNode::CompOutput { component_idx, port_idx }
                            }
                        })
                }
                TraceNode::CompInput { component_idx, port_idx } => {
                    blueprint.connections.iter()
                        .find(|conn| conn.target == TargetPort::ComponentInput { component_idx: *component_idx, port_idx: *port_idx })
                        .map(|conn| match conn.source {
                            SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                            SourcePort::ComponentOutput { component_idx, port_idx } => {
                                TraceNode::CompOutput { component_idx, port_idx }
                            }
                        })
                }
                TraceNode::CompOutput { component_idx, port_idx } => {
                    let component = &blueprint.components[*component_idx];
                    match &component.component_type {
                        ComponentType::Nand | ComponentType::Input | ComponentType::Output | ComponentType::Clock => None,
                        ComponentType::SubChip(_) => {
                            let (_, ref outputs) = component_ports[*component_idx];
                            match outputs[*port_idx] {
                                OutputSource::PassedThrough(in_idx) => {
                                    Some(TraceNode::CompInput { component_idx: *component_idx, port_idx: in_idx })
                                }
                                _ => None,
                            }
                        }
                    }
                }
                TraceNode::ChipInput(_) => None,
            }
        };

        let trace_root = |start_node: TraceNode| -> OutputSource {
            let mut current = start_node;
            let mut visited = std::collections::HashSet::new();
            
            loop {
                if !visited.insert(current.clone()) {
                    return OutputSource::Floating;
                }
                
                match current {
                    TraceNode::ChipInput(idx) => {
                        return OutputSource::PassedThrough(idx);
                    }
                    TraceNode::CompOutput { component_idx, port_idx } => {
                        let component = &blueprint.components[component_idx];
                        match &component.component_type {
                            ComponentType::Nand | ComponentType::Clock => {
                                let (_, ref outputs) = component_ports[component_idx];
                                match outputs[0] {
                                    OutputSource::DrivenByGate(g_idx) => return OutputSource::DrivenByGate(g_idx),
                                    _ => return OutputSource::Floating,
                                }
                            }
                            ComponentType::SubChip(_) => {
                                let (_, ref outputs) = component_ports[component_idx];
                                match outputs[port_idx] {
                                    OutputSource::DrivenByGate(g_idx) => return OutputSource::DrivenByGate(g_idx),
                                    OutputSource::Floating => return OutputSource::Floating,
                                    OutputSource::PassedThrough(in_idx) => {
                                        current = TraceNode::CompInput { component_idx, port_idx: in_idx };
                                    }
                                }
                            }
                            ComponentType::Input | ComponentType::Output => return OutputSource::Floating,
                        }
                    }
                    _ => {
                        if let Some(next_node) = get_immediate_source(&current) {
                            current = next_node;
                        } else {
                            return OutputSource::Floating;
                        }
                    }
                }
            }
        };

        // 2. Wire up all component inputs by tracing their root drivers
        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            let input_ports_count = match &component.component_type {
                ComponentType::Nand => 2,
                ComponentType::Input => 0,
                ComponentType::Output => 1,
                ComponentType::Clock => 0,
                ComponentType::SubChip(sub_idx) => library[*sub_idx].inputs,
            };

            for port_idx in 0..input_ports_count {
                let start_node = TraceNode::CompInput { component_idx: comp_idx, port_idx };
                let driver = trace_root(start_node);

                if let OutputSource::DrivenByGate(src_g_idx) = driver {
                    // Get all primitive targets of this input port
                    let targets = &component_ports[comp_idx].0[port_idx];
                    for &(tgt_g_idx, tgt_port) in targets {
                        self.connect(src_g_idx, tgt_g_idx, tgt_port);
                    }
                }
            }
        }

        // 3. Resolve the inputs interface for the parent chip
        let mut inputs = vec![Vec::new(); blueprint.inputs];
        for i in 0..blueprint.inputs {
            for (comp_idx, component) in blueprint.components.iter().enumerate() {
                let input_ports_count = match &component.component_type {
                    ComponentType::Nand => 2,
                    ComponentType::Input => 0,
                    ComponentType::Output => 1,
                    ComponentType::Clock => 0,
                    ComponentType::SubChip(sub_idx) => library[*sub_idx].inputs,
                };
                
                for port_idx in 0..input_ports_count {
                    let comp_in_node = TraceNode::CompInput { component_idx: comp_idx, port_idx };
                    let driver = trace_root(comp_in_node);
                    if driver == OutputSource::PassedThrough(i) {
                        let targets = &component_ports[comp_idx].0[port_idx];
                        inputs[i].extend(targets.iter().copied());
                    }
                }
            }
        }

        // 4. Resolve the outputs interface for the parent chip
        let mut outputs = Vec::new();
        for j in 0..blueprint.outputs {
            let start_node = TraceNode::ChipOutput(j);
            let driver = trace_root(start_node);
            outputs.push(driver);
        }

        Ok(InstantiatedInterface { inputs, outputs })
    }

    /// Instantiates a custom chip from the blueprint library using compile-time wire aliasing.
    /// Bypasses string lookups and physical buffer gates, resolving inputs/outputs to their root sources.
    pub fn instantiate_chip(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
    ) -> Result<InstantiatedInterface, String> {
        let mut dummy_map = std::collections::HashMap::new();
        let mut dummy_outputs = std::collections::HashMap::new();
        let mut dummy_clocks = Vec::new();
        self.instantiate_chip_with_mapping(blueprint_idx, library, &[], &mut dummy_map, &mut dummy_outputs, &mut dummy_clocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_nand() {
        let mut sim = Simulator::new();
        
        let in_a = sim.add_gate(GateType::Input);
        let in_b = sim.add_gate(GateType::Input);
        let nand = sim.add_gate(GateType::Nand);
        let out = sim.add_gate(GateType::Output);

        sim.connect(in_a, nand, 0);
        sim.connect(in_b, nand, 1);
        sim.connect(nand, out, 0);

        // Run initial propagation to settle floating/initial connections
        assert!(sim.propagate_events(100).is_ok());

        // Test NAND truth table:
        // A=0, B=0 => OUT=1
        sim.set_input(in_a, false);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=0, B=1 => OUT=1
        sim.set_input(in_a, false);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=0 => OUT=1
        sim.set_input(in_a, true);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=1 => OUT=0
        sim.set_input(in_a, true);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);
    }

    #[test]
    fn test_nand_inverter() {
        let mut sim = Simulator::new();

        let input = sim.add_gate(GateType::Input);
        let nand = sim.add_gate(GateType::Nand);
        let out = sim.add_gate(GateType::Output);

        sim.connect(input, nand, 0);
        sim.connect(input, nand, 1);
        sim.connect(nand, out, 0);

        assert!(sim.propagate_events(100).is_ok());

        // Input = false => Out = true
        sim.set_input(input, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // Input = true => Out = false
        sim.set_input(input, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);
    }

    #[test]
    fn test_and_gate() {
        let mut sim = Simulator::new();

        let in_a = sim.add_gate(GateType::Input);
        let in_b = sim.add_gate(GateType::Input);
        let nand1 = sim.add_gate(GateType::Nand);
        let nand2 = sim.add_gate(GateType::Nand); // inverter
        let out = sim.add_gate(GateType::Output);

        sim.connect(in_a, nand1, 0);
        sim.connect(in_b, nand1, 1);
        sim.connect(nand1, nand2, 0);
        sim.connect(nand1, nand2, 1);
        sim.connect(nand2, out, 0);

        assert!(sim.propagate_events(100).is_ok());

        // Test AND truth table:
        // A=0, B=0 => OUT=0
        sim.set_input(in_a, false);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // A=0, B=1 => OUT=0
        sim.set_input(in_a, false);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // A=1, B=0 => OUT=0
        sim.set_input(in_a, true);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // A=1, B=1 => OUT=1
        sim.set_input(in_a, true);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);
    }

    #[test]
    fn test_or_gate() {
        let mut sim = Simulator::new();

        let in_a = sim.add_gate(GateType::Input);
        let in_b = sim.add_gate(GateType::Input);
        let inv_a = sim.add_gate(GateType::Nand);
        let inv_b = sim.add_gate(GateType::Nand);
        let nand = sim.add_gate(GateType::Nand);
        let out = sim.add_gate(GateType::Output);

        sim.connect(in_a, inv_a, 0);
        sim.connect(in_a, inv_a, 1);
        sim.connect(in_b, inv_b, 0);
        sim.connect(in_b, inv_b, 1);

        sim.connect(inv_a, nand, 0);
        sim.connect(inv_b, nand, 1);
        sim.connect(nand, out, 0);

        assert!(sim.propagate_events(100).is_ok());

        // Test OR truth table:
        // A=0, B=0 => OUT=0
        sim.set_input(in_a, false);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // A=0, B=1 => OUT=1
        sim.set_input(in_a, false);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=0 => OUT=1
        sim.set_input(in_a, true);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=1 => OUT=1
        sim.set_input(in_a, true);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);
    }

    #[test]
    fn test_sr_latch() {
        let mut sim = Simulator::new();

        // Active-low inputs: S_bar and R_bar
        let s_bar = sim.add_gate(GateType::Input);
        let r_bar = sim.add_gate(GateType::Input);
        let q_nand = sim.add_gate(GateType::Nand);
        let q_bar_nand = sim.add_gate(GateType::Nand);
        let q_out = sim.add_gate(GateType::Output);
        let q_bar_out = sim.add_gate(GateType::Output);

        // Cross-coupled logic:
        // Q = Nand(S_bar, Q_bar)
        sim.connect(s_bar, q_nand, 0);
        sim.connect(q_bar_nand, q_nand, 1);

        // Q_bar = Nand(Q, R_bar)
        sim.connect(q_nand, q_bar_nand, 0);
        sim.connect(r_bar, q_bar_nand, 1);

        // Connect to outputs
        sim.connect(q_nand, q_out, 0);
        sim.connect(q_bar_nand, q_bar_out, 0);

        // Initial inputs to 1 (inactive state)
        sim.set_input(s_bar, true);
        sim.set_input(r_bar, true);
        assert!(sim.propagate_events(100).is_ok());

        // Latch should settle in a valid state (either Q=0/Q_bar=1 or Q=1/Q_bar=0)
        let q_init = sim.get_state(q_out);
        let q_bar_init = sim.get_state(q_bar_out);
        assert!(q_init != q_bar_init, "Initial state should be complementary");

        // Action: Pulse Set (S_bar = 0)
        sim.set_input(s_bar, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(q_out), true);
        assert_eq!(sim.get_state(q_bar_out), false);

        // Release Set (S_bar = 1) -> state should latch (Q=1, Q_bar=0)
        sim.set_input(s_bar, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(q_out), true);
        assert_eq!(sim.get_state(q_bar_out), false);

        // Action: Pulse Reset (R_bar = 0)
        sim.set_input(r_bar, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(q_out), false);
        assert_eq!(sim.get_state(q_bar_out), true);

        // Release Reset (R_bar = 1) -> state should latch (Q=0, Q_bar=1)
        sim.set_input(r_bar, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(q_out), false);
        assert_eq!(sim.get_state(q_bar_out), true);
    }

    #[test]
    fn test_oscillation_detection() {
        let mut sim = Simulator::new();

        // Create an oscillator: NAND gate with output fed back to both inputs
        let nand = sim.add_gate(GateType::Nand);
        sim.connect(nand, nand, 0); // feed back to port 0
        sim.connect(nand, nand, 1); // feed back to port 1

        // Attempting to propagate events should exceed the max_steps limit
        let res = sim.propagate_events(100);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Oscillation detected"));
    }

    #[test]
    fn test_compilation_and_nesting() {
        let library = vec![
            // 0: AND (2 NANDs)
            ChipBlueprint {
                name: "AND".to_string(),
                inputs: 2,
                outputs: 1,
                input_names: vec!["A".to_string(), "B".to_string()],
                output_names: vec!["OUT".to_string()],
                components: vec![
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 0
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 1
                ],
                connections: vec![
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
                ],
            },
            // 1: OR (3 NANDs)
            ChipBlueprint {
                name: "OR".to_string(),
                inputs: 2,
                outputs: 1,
                input_names: vec!["A".to_string(), "B".to_string()],
                output_names: vec!["OUT".to_string()],
                components: vec![
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 0: A inverter
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 1: B inverter
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 2: NAND combination
                ],
                connections: vec![
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                    Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
                ],
            },
            // 2: XOR (nested OR, NAND, AND)
            ChipBlueprint {
                name: "XOR".to_string(),
                inputs: 2,
                outputs: 1,
                input_names: vec!["A".to_string(), "B".to_string()],
                output_names: vec!["OUT".to_string()],
                components: vec![
                    Component { component_type: ComponentType::SubChip(1), pos: (0.0, 0.0), clock_period: None }, // Comp 0: OR
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None },       // Comp 1: Nand
                    Component { component_type: ComponentType::SubChip(0), pos: (0.0, 0.0), clock_period: None }, // Comp 2: AND
                ],
                connections: vec![
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
                ],
            },
            // 3: PassThrough
            ChipBlueprint {
                name: "PassThrough".to_string(),
                inputs: 1,
                outputs: 1,
                input_names: vec!["IN".to_string()],
                output_names: vec!["OUT".to_string()],
                components: vec![],
                connections: vec![
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ChipOutput(0) },
                ],
            },
            // 4: BufferTest (Nand inverter -> PassThrough -> ChipOutput)
            ChipBlueprint {
                name: "BufferTest".to_string(),
                inputs: 1,
                outputs: 1,
                input_names: vec!["IN".to_string()],
                output_names: vec!["OUT".to_string()],
                components: vec![
                    Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None }, // Comp 0: inverter
                    Component { component_type: ComponentType::SubChip(3), pos: (0.0, 0.0), clock_period: None }, // Comp 1: PassThrough
                ],
                connections: vec![
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                    Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                    Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
                ],
            }
        ];

        let mut sim = Simulator::new();

        // 1. Compile XOR
        let in_a = sim.add_gate(GateType::Input);
        let in_b = sim.add_gate(GateType::Input);
        let xor_interface = sim.instantiate_chip(2, &library).expect("Failed to compile XOR");
        let out = sim.add_gate(GateType::Output);

        for &(tgt_idx, tgt_port) in &xor_interface.inputs[0] {
            sim.connect(in_a, tgt_idx, tgt_port);
        }
        for &(tgt_idx, tgt_port) in &xor_interface.inputs[1] {
            sim.connect(in_b, tgt_idx, tgt_port);
        }

        match xor_interface.outputs[0] {
            OutputSource::DrivenByGate(g_idx) => {
                sim.connect(g_idx, out, 0);
            }
            _ => panic!("Expected XOR output to be driven by a gate!"),
        }

        assert!(sim.propagate_events(100).is_ok());

        // Test XOR truth table:
        // A=0, B=0 => OUT=0
        sim.set_input(in_a, false);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // A=0, B=1 => OUT=1
        sim.set_input(in_a, false);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=0 => OUT=1
        sim.set_input(in_a, true);
        sim.set_input(in_b, false);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), true);

        // A=1, B=1 => OUT=0
        sim.set_input(in_a, true);
        sim.set_input(in_b, true);
        assert!(sim.propagate_events(100).is_ok());
        assert_eq!(sim.get_state(out), false);

        // 2. Compile and test BufferTest (inverter -> PassThrough -> output)
        let mut sim2 = Simulator::new();
        let b_in = sim2.add_gate(GateType::Input);
        let b_interface = sim2.instantiate_chip(4, &library).expect("Failed to compile BufferTest");
        let b_out = sim2.add_gate(GateType::Output);

        for &(tgt_idx, tgt_port) in &b_interface.inputs[0] {
            sim2.connect(b_in, tgt_idx, tgt_port);
        }

        match b_interface.outputs[0] {
            OutputSource::DrivenByGate(g_idx) => {
                sim2.connect(g_idx, b_out, 0);
            }
            _ => panic!("Expected BufferTest output to resolve to DrivenByGate!"),
        }

        // Test inverter + PassThrough:
        // Input = false => Output = true
        sim2.set_input(b_in, false);
        assert!(sim2.propagate_events(100).is_ok());
        assert_eq!(sim2.get_state(b_out), true);

        // Input = true => Output = false
        sim2.set_input(b_in, true);
        assert!(sim2.propagate_events(100).is_ok());
        assert_eq!(sim2.get_state(b_out), false);
    }

    #[test]
    fn test_serialization() {
        let blueprint = ChipBlueprint {
            name: "TestBP".to_string(),
            inputs: 1,
            outputs: 1,
            input_names: vec!["IN".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component { component_type: ComponentType::Clock, pos: (10.0, 20.0), clock_period: Some(50) },
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 },
                    target: TargetPort::ChipOutput(0),
                }
            ],
        };

        let serialized = serde_json::to_string(&blueprint).expect("Failed to serialize blueprint");
        let deserialized: ChipBlueprint = serde_json::from_str(&serialized).expect("Failed to deserialize blueprint");
        assert_eq!(deserialized.name, "TestBP");
        assert_eq!(deserialized.inputs, 1);
        assert_eq!(deserialized.components[0].clock_period, Some(50));
    }

    #[test]
    fn test_multi_domain_clocks() {
        let mut sim = Simulator::new();
        let library = vec![
            ChipBlueprint {
                name: "ClockChip".to_string(),
                inputs: 0,
                outputs: 1,
                input_names: vec![],
                output_names: vec!["OUT".to_string()],
                components: vec![
                    Component { component_type: ComponentType::Clock, pos: (0.0, 0.0), clock_period: Some(10) },
                ],
                connections: vec![
                    Connection {
                        source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 },
                        target: TargetPort::ChipOutput(0),
                    }
                ],
            }
        ];

        let mut active_clocks = Vec::new();
        let mut inst_map = std::collections::HashMap::new();
        let mut inst_outs = std::collections::HashMap::new();
        let interface = sim.instantiate_chip_with_mapping(0, &library, &[], &mut inst_map, &mut inst_outs, &mut active_clocks)
            .expect("Failed to instantiate ClockChip");

        assert_eq!(active_clocks.len(), 1);
        let clock = &mut active_clocks[0];
        assert_eq!(clock.period, 10);

        // Ticks/propagation test
        let mut val = false;
        sim.set_input(clock.gate_idx, val);

        for tick in 1..=30 {
            clock.counter += 1;
            let half_period = (clock.period / 2).max(1); // 5 ticks
            if clock.counter >= half_period {
                clock.counter = 0;
                val = !val;
                sim.set_input(clock.gate_idx, val);
            }
            let _ = sim.propagate_events(50);

            let expected_val = if (tick / 5) % 2 == 1 { true } else { false };
            let out_idx = match interface.outputs[0] {
                OutputSource::DrivenByGate(g_idx) => g_idx,
                _ => panic!("Expected driven by gate output"),
            };
            assert_eq!(sim.get_state(out_idx), expected_val, "Failed at tick {}", tick);
        }
    }
}
