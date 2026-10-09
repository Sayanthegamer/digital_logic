#![allow(clippy::needless_range_loop)]

use logic_simulator::engine::Simulator;
use logic_simulator::engine::compiler::compile_subchip_template;
use logic_simulator::engine::sleep::SleepDomain;
use logic_simulator::engine::types::*;

/// Helper to build a 1-bit full adder blueprint out of raw NAND gates
/// Inputs: A (0), B (1), Cin (2)
/// Outputs: Sum (0), Cout (1)
fn build_full_adder_blueprint() -> ChipBlueprint {
    // Standard 9-NAND Full Adder:
    // XOR1 = 4 NANDs (0, 1, 2, 3) for A XOR B
    // XOR2 = 4 NANDs (4, 5, 6, 7) for (A XOR B) XOR Cin -> Sum
    // Cout = 1 NAND (8) combined with intermediate terms
    let components = vec![
        // XOR(A, B): gates 0..4
        // NAND 0: NAND(A, B)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 1: NAND(A, NAND0)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 2: NAND(B, NAND0)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 3: NAND(NAND1, NAND2) -> A XOR B
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // XOR(NAND3, Cin): gates 4..8
        // NAND 4: NAND(NAND3, Cin)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 5: NAND(NAND3, NAND4)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 6: NAND(Cin, NAND4)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // NAND 7: NAND(NAND5, NAND6) -> Sum
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
        // Cout: NAND(NAND0, NAND4)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        },
    ];

    let connections = vec![
        // NAND 0: inputs A(0) and B(1)
        Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 1,
            },
        },
        // NAND 1: inputs A(0) and NAND0
        Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 1,
            },
        },
        // NAND 2: inputs B(1) and NAND0
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 1,
            },
        },
        // NAND 3: inputs NAND1 and NAND2 (A XOR B)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 1,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 2,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 1,
            },
        },
        // NAND 4: inputs NAND3 and Cin(2)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 4,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(2),
            target: TargetPort::ComponentInput {
                component_idx: 4,
                port_idx: 1,
            },
        },
        // NAND 5: inputs NAND3 and NAND4
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 5,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 4,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 5,
                port_idx: 1,
            },
        },
        // NAND 6: inputs Cin(2) and NAND4
        Connection {
            source: SourcePort::ChipInput(2),
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 4,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 1,
            },
        },
        // NAND 7: inputs NAND5 and NAND6 (Sum)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 5,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 7,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 6,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 7,
                port_idx: 1,
            },
        },
        // NAND 8: Cout = NAND(NAND0, NAND4)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 8,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 4,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 8,
                port_idx: 1,
            },
        },
        // Chip Outputs: Sum = NAND7, Cout = NAND8
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 7,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(0),
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 8,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(1),
        },
    ];

    ChipBlueprint {
        name: "FullAdder".to_string(),
        inputs: 3,
        outputs: 2,
        input_names: vec!["A".to_string(), "B".to_string(), "Cin".to_string()],
        output_names: vec!["Sum".to_string(), "Cout".to_string()],
        components,
        connections,
    }
}

/// Helper to build a 1-bit D-latch blueprint out of raw NAND gates with an Enable line
/// Inputs: D (0), EN (1)
/// Outputs: Q (0), Q_bar (1)
fn build_d_latch_blueprint() -> ChipBlueprint {
    // 4-NAND D Latch:
    // NAND 0: NAND(D, EN)
    // NAND 1: NAND(NAND0, EN)
    // NAND 2 (Q): NAND(NAND0, Q_bar)
    // NAND 3 (Q_bar): NAND(NAND1, Q)
    let components = vec![
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 0
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 1
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 2: Q
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 3: Q_bar
    ];

    let connections = vec![
        // NAND 0: D(0) and EN(1)
        Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 1,
            },
        },
        // NAND 1: NAND0 and EN(1)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 1,
            },
        },
        // NAND 2 (Q): NAND0 and NAND3(Q_bar)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 1,
            },
        },
        // NAND 3 (Q_bar): NAND1 and NAND2(Q)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 1,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 2,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 1,
            },
        },
        // Outputs
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 2,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(0),
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(1),
        },
    ];

    ChipBlueprint {
        name: "DLatch".to_string(),
        inputs: 2,
        outputs: 2,
        input_names: vec!["D".to_string(), "Enable".to_string()],
        output_names: vec!["Q".to_string(), "Q_bar".to_string()],
        components,
        connections,
    }
}

#[test]
fn test_32bit_adder_pure_nand_arithmetic() {
    let adder_bp = build_full_adder_blueprint();
    let library = vec![adder_bp];
    let template = compile_subchip_template(0, &library).expect("Failed to compile adder template");

    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();

    // Inputs: A (32 bits), B (32 bits), Cin (1 bit)
    let mut a_inputs = Vec::new();
    let mut b_inputs = Vec::new();
    for _ in 0..32 {
        a_inputs.push(sim.add_gate(GateType::Input));
        b_inputs.push(sim.add_gate(GateType::Input));
    }
    let cin_gate = sim.add_gate(GateType::Input);

    // Instantiate 32 1-bit full adders using template
    let mut prev_carry = cin_gate;
    let mut sum_outputs = Vec::new();

    for i in 0..32 {
        let (interface, _) = template.instantiate_into(&mut sim, &mut active_clocks);

        // Connect A[i] to interface.inputs[0]
        for &(tgt_g, port) in &interface.inputs[0] {
            sim.connect(a_inputs[i], tgt_g, port);
        }
        // Connect B[i] to interface.inputs[1]
        for &(tgt_g, port) in &interface.inputs[1] {
            sim.connect(b_inputs[i], tgt_g, port);
        }
        // Connect Cin to interface.inputs[2]
        for &(tgt_g, port) in &interface.inputs[2] {
            sim.connect(prev_carry, tgt_g, port);
        }

        // Sum output is interface.outputs[0]
        if let OutputSource::DrivenByGate(sum_g) = interface.outputs[0] {
            sum_outputs.push(sum_g);
        } else {
            panic!("Sum output not driven by gate");
        }

        // Cout is interface.outputs[1]
        if let OutputSource::DrivenByGate(cout_g) = interface.outputs[1] {
            prev_carry = cout_g;
        } else {
            panic!("Cout output not driven by gate");
        }
    }
    let final_cout = prev_carry;

    sim.calculate_depths();
    let old_to_new = sim.defragment_and_sort_by_depth();

    // Remap gate tracking indices after defragmentation
    for g in &mut a_inputs {
        *g = old_to_new[*g];
    }
    for g in &mut b_inputs {
        *g = old_to_new[*g];
    }
    let cin_gate = old_to_new[cin_gate];
    for g in &mut sum_outputs {
        *g = old_to_new[*g];
    }
    let final_cout = old_to_new[final_cout];

    // Settle circuit to stable initial rest state
    sim.settle().expect("Initial settle failed");

    // Helper closure to set 32-bit values and read result
    let mut run_add = |val_a: u32, val_b: u32, cin: bool| -> (u32, bool) {
        for i in 0..32 {
            sim.set_input(a_inputs[i], (val_a & (1 << i)) != 0);
            sim.set_input(b_inputs[i], (val_b & (1 << i)) != 0);
        }
        sim.set_input(cin_gate, cin);

        sim.propagate_events(200).expect("Propagation failed");

        let mut sum = 0u32;
        for i in 0..32 {
            if sim.get_state(sum_outputs[i]) {
                sum |= 1 << i;
            }
        }
        let cout = sim.get_state(final_cout);
        (sum, cout)
    };

    // Test vector 1: 0 + 0 = 0, cout = false
    let (s, c) = run_add(0, 0, false);
    assert_eq!(s, 0);
    assert!(!c);

    // Test vector 2: 1 + 1 = 2, cout = false
    let (s, c) = run_add(1, 1, false);
    assert_eq!(s, 2);
    assert!(!c);

    // Test vector 3: 0x12345678 + 0x0EDCBA98 = 0x21111110
    let (s, c) = run_add(0x12345678, 0x0EDCBA98, false);
    assert_eq!(s, 0x21111110);
    assert!(!c);

    // Test vector 4: 0xFFFFFFFF + 1 = 0x00000000 with Cout = true
    let (s, c) = run_add(0xFFFFFFFF, 1, false);
    assert_eq!(s, 0x00000000);
    assert!(c);

    // Test vector 5: With Cin: 0x00000005 + 0x0000000A + 1 = 0x10
    let (s, c) = run_add(5, 10, true);
    assert_eq!(s, 16);
    assert!(!c);
}

#[test]
fn test_single_dlatch() {
    let dlatch_bp = build_d_latch_blueprint();
    let library = vec![dlatch_bp];
    let template =
        compile_subchip_template(0, &library).expect("Failed to compile dlatch template");

    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();
    let d_in = sim.add_gate(GateType::Input);
    let en_in = sim.add_gate(GateType::Input);

    let (interface, _) = template.instantiate_into(&mut sim, &mut active_clocks);
    for &(tgt, p) in &interface.inputs[0] {
        sim.connect(d_in, tgt, p);
    }
    for &(tgt, p) in &interface.inputs[1] {
        sim.connect(en_in, tgt, p);
    }

    let q = match interface.outputs[0] {
        OutputSource::DrivenByGate(g) => g,
        _ => panic!("Expected gate"),
    };
    let q_bar = match interface.outputs[1] {
        OutputSource::DrivenByGate(g) => g,
        _ => panic!("Expected gate"),
    };

    sim.calculate_depths();

    // Enable high, D=0
    sim.set_input(en_in, true);
    sim.set_input(d_in, false);
    sim.propagate_events(100).expect("propagate");
    assert!(!sim.get_state(q));
    assert!(sim.get_state(q_bar));
}

#[test]
fn test_32word_register_file_pure_nand() {
    let dlatch_bp = build_d_latch_blueprint();
    let library = vec![dlatch_bp];
    let template =
        compile_subchip_template(0, &library).expect("Failed to compile dlatch template");

    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();

    // 32 registers, each 32 bits = 1,024 latches
    let mut word_enables = Vec::new();
    let mut data_inputs = Vec::new();
    for _ in 0..32 {
        data_inputs.push(sim.add_gate(GateType::Input));
    }

    let mut reg_q_outputs = Vec::new(); // 32 words x 32 bits

    for word_idx in 0..32 {
        let en = sim.add_gate(GateType::Input);
        sim.set_input(en, false); // Initially disabled
        word_enables.push(en);

        let word_start = sim.nodes.len();
        let mut word_bits = Vec::new();
        for bit in 0..32 {
            let (interface, _) = template.instantiate_into(&mut sim, &mut active_clocks);

            // Connect D (interface.inputs[0]) to data_inputs[bit]
            for &(tgt_g, port) in &interface.inputs[0] {
                sim.connect(data_inputs[bit], tgt_g, port);
            }
            // Connect Enable (interface.inputs[1]) to word_enable
            for &(tgt_g, port) in &interface.inputs[1] {
                sim.connect(en, tgt_g, port);
            }

            if let OutputSource::DrivenByGate(q_g) = interface.outputs[0] {
                word_bits.push(q_g);
            } else {
                panic!("Q output not driven");
            }
        }
        let word_end = sim.nodes.len();

        // Register Sleep Domain for this word (all 128 gates in word_start..word_end, 32 latches)
        let domain = SleepDomain::new(
            word_idx,
            en,
            true,
            (word_start..word_end).collect(),
            word_bits.clone(),
        );
        sim.register_sleep_domain(domain);

        reg_q_outputs.push(word_bits);
    }

    sim.calculate_depths();
    let old_to_new = sim.defragment_and_sort_by_depth();

    // Remap gate tracking indices after defragmentation
    for g in &mut data_inputs {
        *g = old_to_new[*g];
    }
    for g in &mut word_enables {
        *g = old_to_new[*g];
    }
    for row in &mut reg_q_outputs {
        for g in row {
            *g = old_to_new[*g];
        }
    }

    // 1,024 latches registered across 32 sleep domains!
    assert_eq!(sim.sleep_domains.len(), 32);

    // Write 0xCAFEBABE to register 5
    let write_word = |sim: &mut Simulator, word_idx: usize, val: u32| {
        // Pulse Enable High (wakes domain, selects word)
        sim.set_input(word_enables[word_idx], true);
        for bit in 0..32 {
            sim.set_input(data_inputs[bit], (val & (1 << bit)) != 0);
        }
        sim.propagate_events(200).expect("Write High failed");

        // Latch Enable Low (hibernates domain into dense 1-bit-per-latch buffer)
        sim.set_input(word_enables[word_idx], false);
        sim.propagate_events(200).expect("Write Low failed");
    };

    let read_word = |sim: &mut Simulator, word_idx: usize| -> u32 {
        let mut val = 0u32;
        for bit in 0..32 {
            if sim.get_state(reg_q_outputs[word_idx][bit]) {
                val |= 1 << bit;
            }
        }
        val
    };

    write_word(&mut sim, 5, 0xCAFEBABE);
    assert_eq!(read_word(&mut sim, 5), 0xCAFEBABE);

    // Write 0xDEADBEEF to register 12
    write_word(&mut sim, 12, 0xDEADBEEF);
    assert_eq!(read_word(&mut sim, 12), 0xDEADBEEF);

    // Verify register 5 retained 0xCAFEBABE while asleep!
    assert_eq!(read_word(&mut sim, 5), 0xCAFEBABE);
}

#[test]
fn test_multibank_ram_pure_nand_activity_gating() {
    let dlatch_bp = build_d_latch_blueprint();
    let library = vec![dlatch_bp];
    let template =
        compile_subchip_template(0, &library).expect("Failed to compile dlatch template");

    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();

    // 2-bit Address bus: A0, A1 (Decodes 4 banks)
    let a0 = sim.add_gate(GateType::Input);
    let a1 = sim.add_gate(GateType::Input);

    // Global Write Enable
    let we = sim.add_gate(GateType::Input);

    // 8-bit Data Bus
    let mut data_bus = Vec::new();
    for _ in 0..8 {
        data_bus.push(sim.add_gate(GateType::Input));
    }

    // Pure NAND 2-to-4 Address Decoder
    // Inverters: NOT(A0), NOT(A1)
    let not_a0 = sim.add_gate(GateType::Nand);
    sim.connect(a0, not_a0, 0);
    sim.connect(a0, not_a0, 1);

    let not_a1 = sim.add_gate(GateType::Nand);
    sim.connect(a1, not_a1, 0);
    sim.connect(a1, not_a1, 1);

    // CS outputs (4 banks) via AND gates (2 NANDs each):
    // CS0: NOT_A1 & NOT_A0
    // CS1: NOT_A1 & A0
    // CS2: A1 & NOT_A0
    // CS3: A1 & A0
    let mut cs_gates = Vec::new();
    let addr_pairs = [(not_a1, not_a0), (not_a1, a0), (a1, not_a0), (a1, a0)];
    for &(in_h, in_l) in &addr_pairs {
        let nand_ab = sim.add_gate(GateType::Nand);
        sim.connect(in_h, nand_ab, 0);
        sim.connect(in_l, nand_ab, 1);
        let cs = sim.add_gate(GateType::Nand);
        sim.connect(nand_ab, cs, 0);
        sim.connect(nand_ab, cs, 1);
        cs_gates.push(cs);
    }

    let mut bank_q_outputs = Vec::new(); // 4 banks x 8 bits

    // Instantiate 4 memory banks (each 8 bits of pure NAND D-latches)
    for bank_idx in 0..4 {
        let cs = cs_gates[bank_idx];

        // Bank Write Enable: AND(CS, WE)
        let bwe_nand = sim.add_gate(GateType::Nand);
        sim.connect(cs, bwe_nand, 0);
        sim.connect(we, bwe_nand, 1);
        let bank_we = sim.add_gate(GateType::Nand);
        sim.connect(bwe_nand, bank_we, 0);
        sim.connect(bwe_nand, bank_we, 1);

        let bank_start = sim.nodes.len();
        let mut bank_bits = Vec::new();
        for bit in 0..8 {
            let (interface, _) = template.instantiate_into(&mut sim, &mut active_clocks);

            // Connect D (interface.inputs[0]) to data_bus[bit]
            for &(tgt_g, port) in &interface.inputs[0] {
                sim.connect(data_bus[bit], tgt_g, port);
            }
            // Connect Enable (interface.inputs[1]) to bank_we
            for &(tgt_g, port) in &interface.inputs[1] {
                sim.connect(bank_we, tgt_g, port);
            }

            if let OutputSource::DrivenByGate(q_g) = interface.outputs[0] {
                bank_bits.push(q_g);
            } else {
                panic!("Q output not driven");
            }
        }
        let bank_end = sim.nodes.len();

        let mut bank_gates = vec![bwe_nand, bank_we];
        bank_gates.extend(bank_start..bank_end);

        // Establish stable initial rest state (WE inactive, Q low)
        sim.nodes.states[bank_we] = 0b01;
        for &q_g in &bank_bits {
            sim.nodes.states[q_g] = 0b01;
        }

        // Register Activity-Gated Sleep Domain for this memory bank
        let domain = SleepDomain::new(
            bank_idx,
            cs,
            true, // CS High = awake, CS Low = hibernate
            bank_gates,
            bank_bits.clone(),
        );
        sim.register_sleep_domain(domain);

        bank_q_outputs.push(bank_bits);
    }

    sim.calculate_depths();
    let old_to_new = sim.defragment_and_sort_by_depth();

    // Remap all tracked gates
    let a0 = old_to_new[a0];
    let a1 = old_to_new[a1];
    let we = old_to_new[we];
    for g in &mut data_bus {
        *g = old_to_new[*g];
    }
    for g in &mut cs_gates {
        *g = old_to_new[*g];
    }
    for row in &mut bank_q_outputs {
        for g in row {
            *g = old_to_new[*g];
        }
    }

    assert_eq!(sim.sleep_domains.len(), 4);

    // Initial state: Address = 00 (Bank 0), WE = 0
    sim.set_input(a0, false);
    sim.set_input(a1, false);
    sim.set_input(we, false);
    sim.propagate_events(200).expect("Initial settle failed");

    // Bank 0 is awake, Banks 1, 2, 3 are sleeping
    assert!(!sim.is_domain_sleeping(0));
    assert!(sim.is_domain_sleeping(1));
    assert!(sim.is_domain_sleeping(2));
    assert!(sim.is_domain_sleeping(3));

    let select_bank = |sim: &mut Simulator, bank_idx: usize| {
        sim.set_input(a0, (bank_idx & 1) != 0);
        sim.set_input(a1, (bank_idx & 2) != 0);
        sim.propagate_events(200)
            .expect("Bank select propagate failed");
    };

    let write_byte = |sim: &mut Simulator, byte_val: u8| {
        for bit in 0..8 {
            sim.set_input(data_bus[bit], (byte_val & (1 << bit)) != 0);
        }
        // Pulse WE High then Low
        sim.set_input(we, true);
        sim.propagate_events(200).expect("WE High failed");
        sim.set_input(we, false);
        sim.propagate_events(200).expect("WE Low failed");
    };

    let read_bank = |sim: &Simulator, bank_idx: usize| -> u8 {
        let mut val = 0u8;
        for bit in 0..8 {
            if sim.get_state(bank_q_outputs[bank_idx][bit]) {
                val |= 1 << bit;
            }
        }
        val
    };

    // 1. Select Bank 2 (address = 2), write 0xA5
    select_bank(&mut sim, 2);
    assert!(!sim.is_domain_sleeping(2));
    assert!(sim.is_domain_sleeping(0));
    assert!(sim.is_domain_sleeping(1));
    assert!(sim.is_domain_sleeping(3));

    write_byte(&mut sim, 0xA5);
    assert_eq!(read_bank(&sim, 2), 0xA5);

    // 2. Select Bank 1 (address = 1), write 0x3C
    select_bank(&mut sim, 1);
    assert!(!sim.is_domain_sleeping(1));
    assert!(sim.is_domain_sleeping(2)); // Bank 2 now hibernating with 0xA5!

    write_byte(&mut sim, 0x3C);
    assert_eq!(read_bank(&sim, 1), 0x3C);

    // 3. Select Bank 3 (address = 3), write 0xF0
    select_bank(&mut sim, 3);
    assert!(!sim.is_domain_sleeping(3));
    assert!(sim.is_domain_sleeping(1)); // Bank 1 now hibernating with 0x3C!

    write_byte(&mut sim, 0xF0);
    assert_eq!(read_bank(&sim, 3), 0xF0);

    // 4. Switch back to Bank 2: wake up and verify data retention!
    select_bank(&mut sim, 2);
    assert!(!sim.is_domain_sleeping(2));
    assert_eq!(read_bank(&sim, 2), 0xA5);

    // 5. Switch back to Bank 1: wake up and verify data retention!
    select_bank(&mut sim, 1);
    assert!(!sim.is_domain_sleeping(1));
    assert_eq!(read_bank(&sim, 1), 0x3C);

    // 6. Switch back to Bank 3: wake up and verify data retention!
    select_bank(&mut sim, 3);
    assert!(!sim.is_domain_sleeping(3));
    assert_eq!(read_bank(&sim, 3), 0xF0);
}
