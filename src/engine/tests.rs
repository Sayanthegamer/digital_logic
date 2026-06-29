#[cfg(test)]
mod tests {
    use crate::engine::*;
    use crate::engine::simulator::Simulator;

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
