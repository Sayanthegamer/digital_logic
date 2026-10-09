use logic_simulator::engine::Simulator;
use logic_simulator::engine::compiler::find_gating_input;
use logic_simulator::engine::types::*;

fn build_unnamed_dlatch_blueprint() -> ChipBlueprint {
    // 4-NAND D-Latch with inputs named ["DataPin", "ControlPin"]
    // NAND 0: NAND(Input 0, Input 1)
    // NAND 1: NAND(NAND 0, Input 1)
    // NAND 2 (Q): NAND(NAND 0, Q_bar)
    // NAND 3 (Q_bar): NAND(NAND 1, Q)
    // ChipOutput 0 = NAND 2 (Q), ChipOutput 1 = NAND 3 (Q_bar)
    let components = vec![
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // NAND 0
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // NAND 1
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // NAND 2 (Q)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // NAND 3 (Q_bar)
    ];

    let connections = vec![
        // NAND 0: NAND(Input 0, Input 1)
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
        // NAND 1: NAND(NAND 0, Input 1)
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
        // NAND 2 (Q): NAND(NAND 0, Q_bar / NAND 3)
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
        // NAND 3 (Q_bar): NAND(NAND 1, Q / NAND 2)
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
        // ChipOutput 0 = NAND 2 (Q)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 2,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(0),
        },
        // ChipOutput 1 = NAND 3 (Q_bar)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(1),
        },
    ];

    ChipBlueprint {
        name: "UnnamedDLatch".to_string(),
        inputs: 2,
        outputs: 2,
        input_names: vec!["DataPin".to_string(), "ControlPin".to_string()],
        output_names: vec!["Q".to_string(), "Q_bar".to_string()],
        components,
        connections,
    }
}

#[test]
fn test_unnamed_dlatch_topological_gating() {
    let blueprint = build_unnamed_dlatch_blueprint();
    let library = vec![blueprint.clone()];

    // ControlPin (Input 1) gates inputs to the bistable feedback loop (NAND 2 & 3).
    // Assert automatic topological detection identifies Input 1 without name heuristics.
    assert_eq!(find_gating_input(&blueprint, &library), Some(1));
}

#[test]
fn test_unnamed_multibit_register_from_subchips() {
    let dlatch_bp = build_unnamed_dlatch_blueprint();
    let library = vec![dlatch_bp];

    // 8 instances of the DLatch subchip inside a register blueprint.
    let components = (0..8)
        .map(|_| Component {
            component_type: ComponentType::SubChip(0),
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        })
        .collect::<Vec<_>>();

    let mut connections = Vec::new();
    for i in 0..8 {
        // Inputs 0..7 connect to DLatch input 0 (Data) of their respective subchips (fanout 1)
        connections.push(Connection {
            source: SourcePort::ChipInput(i),
            target: TargetPort::ComponentInput {
                component_idx: i,
                port_idx: 0,
            },
        });

        // Input 8 ("Trigger") connects to DLatch input 1 (Enable) across ALL 8 subchips (fanout 8)
        connections.push(Connection {
            source: SourcePort::ChipInput(8),
            target: TargetPort::ComponentInput {
                component_idx: i,
                port_idx: 1,
            },
        });

        // Subchip Q to Register Output
        connections.push(Connection {
            source: SourcePort::ComponentOutput {
                component_idx: i,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(i),
        });
    }

    let input_names = vec![
        "Bit0".to_string(),
        "Bit1".to_string(),
        "Bit2".to_string(),
        "Bit3".to_string(),
        "Bit4".to_string(),
        "Bit5".to_string(),
        "Bit6".to_string(),
        "Bit7".to_string(),
        "Trigger".to_string(),
    ];
    let output_names = (0..8).map(|i| format!("Out{}", i)).collect();

    let reg_blueprint = ChipBlueprint {
        name: "Unnamed8BitRegister".to_string(),
        inputs: 9,
        outputs: 8,
        input_names,
        output_names,
        components,
        connections,
    };

    // Assert automatic topological detection identifies Input 8 ("Trigger") with fanout 8 to subchip gating inputs.
    assert_eq!(find_gating_input(&reg_blueprint, &library), Some(8));
}

#[test]
fn test_combinational_adder_immunity() {
    // 9-NAND Full Adder (combinational DAG, zero feedback cycles)
    // Inputs: A (0), B (1), Cin (2)
    // Gate 0: NAND(A, B)
    // Gate 1: NAND(A, Gate 0)
    // Gate 2: NAND(B, Gate 0)
    // Gate 3: NAND(Gate 1, Gate 2) [A XOR B]
    // Gate 4: NAND(Gate 3, Cin)
    // Gate 5: NAND(Gate 3, Gate 4)
    // Gate 6: NAND(Cin, Gate 4)
    // Gate 7: NAND(Gate 5, Gate 6) [Sum = (A XOR B) XOR Cin]
    // Gate 8: NAND(Gate 0, Gate 4) [Cout = (A AND B) OR (Cin AND (A XOR B))]
    let components = (0..9)
        .map(|_| Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        })
        .collect::<Vec<_>>();

    let connections = vec![
        // Gate 0: NAND(A, B)
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
        // Gate 1: NAND(A, Gate 0)
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
        // Gate 2: NAND(B, Gate 0)
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
        // Gate 3: NAND(Gate 1, Gate 2)
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
        // Gate 4: NAND(Gate 3, Cin)
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
        // Gate 5: NAND(Gate 3, Gate 4)
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
        // Gate 6: NAND(Cin, Gate 4)
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
        // Gate 7: NAND(Gate 5, Gate 6) -> Sum
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
        // Gate 8: NAND(Gate 0, Gate 4) -> Cout
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
        // Outputs
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

    let adder_bp = ChipBlueprint {
        name: "PureNandAdder".to_string(),
        inputs: 3,
        outputs: 2,
        input_names: vec!["A".to_string(), "B".to_string(), "Cin".to_string()],
        output_names: vec!["Sum".to_string(), "Cout".to_string()],
        components,
        connections,
    };

    let library = vec![adder_bp.clone()];

    // Assert purely combinational circuits with zero feedback cycles are never identified as sleep-gated.
    assert_eq!(find_gating_input(&adder_bp, &library), None);
}

#[test]
fn test_unnamed_subchip_simulation_activity_gating() {
    let dlatch_bp = build_unnamed_dlatch_blueprint();

    // Blueprint with 2 instances of UnnamedDLatch (Bank 0 and Bank 1)
    // driven by an internal 1-bit address decoder:
    // Inputs:
    // 0: Data
    // 1: Addr (0 selects Bank 0, 1 selects Bank 1)
    // 2: Trigger (Write Enable)
    // Components:
    // 0: SubChip(0) [Bank 0]
    // 1: SubChip(0) [Bank 1]
    // 2: NAND(Addr, Addr) -> NOT(Addr)
    // 3: NAND(NOT Addr, Trigger) -> nand_cs0
    // 4: NAND(nand_cs0, nand_cs0) -> CS0
    // 5: NAND(Addr, Trigger) -> nand_cs1
    // 6: NAND(nand_cs1, nand_cs1) -> CS1
    let components = vec![
        Component {
            component_type: ComponentType::SubChip(0),
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 0
        Component {
            component_type: ComponentType::SubChip(0),
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 1
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 2: NOT(Addr)
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 3: nand_cs0
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 4: CS0
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 5: nand_cs1
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 6: CS1
    ];

    let connections = vec![
        // Data (0) to both subchips
        Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(0),
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 0,
            },
        },
        // Addr (1) to NOT(Addr) [Comp 2]
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 2,
                port_idx: 1,
            },
        },
        // CS0 = AND(NOT Addr, Trigger)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 2,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(2),
            target: TargetPort::ComponentInput {
                component_idx: 3,
                port_idx: 1,
            },
        },
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
            source: SourcePort::ComponentOutput {
                component_idx: 3,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 4,
                port_idx: 1,
            },
        },
        // CS1 = AND(Addr, Trigger)
        Connection {
            source: SourcePort::ChipInput(1),
            target: TargetPort::ComponentInput {
                component_idx: 5,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ChipInput(2),
            target: TargetPort::ComponentInput {
                component_idx: 5,
                port_idx: 1,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 5,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 0,
            },
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 5,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 6,
                port_idx: 1,
            },
        },
        // CS0 drives SubChip 0 gating input (port 1)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 4,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 0,
                port_idx: 1,
            },
        },
        // CS1 drives SubChip 1 gating input (port 1)
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 6,
                port_idx: 0,
            },
            target: TargetPort::ComponentInput {
                component_idx: 1,
                port_idx: 1,
            },
        },
        // Outputs
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(0),
        },
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 1,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(1),
        },
    ];

    let dual_bank_bp = ChipBlueprint {
        name: "DualBank".to_string(),
        inputs: 3,
        outputs: 2,
        input_names: vec![
            "Data".to_string(),
            "Address".to_string(),
            "Trigger".to_string(),
        ],
        output_names: vec!["Q0".to_string(), "Q1".to_string()],
        components,
        connections,
    };

    let library = vec![dlatch_bp, dual_bank_bp];

    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();
    let mut stack = Vec::new();

    // Instantiate DualBank (index 1)
    let (interface, _) = sim
        .instantiate_chip_with_mapping(1, &library, &mut active_clocks, &mut stack)
        .expect("Instantiation failed");

    // Both subchip instances of UnnamedDLatch MUST automatically have sleep domains registered!
    assert_eq!(
        sim.sleep_domains.len(),
        2,
        "Expected 2 automatically detected sleep domains"
    );

    // Create external input gates to drive DualBank inputs
    let in_data = sim.add_gate(GateType::Input);
    let in_addr = sim.add_gate(GateType::Input);
    let in_trig = sim.add_gate(GateType::Input);

    let inputs = [in_data, in_addr, in_trig];
    for (port_idx, &in_g) in inputs.iter().enumerate() {
        for &(tgt_g, tgt_port) in &interface.inputs[port_idx] {
            sim.connect(in_g, tgt_g, tgt_port);
        }
    }

    let q0 = match interface.outputs[0] {
        OutputSource::DrivenByGate(g) => g,
        _ => panic!("Expected driven gate"),
    };
    let q1 = match interface.outputs[1] {
        OutputSource::DrivenByGate(g) => g,
        _ => panic!("Expected driven gate"),
    };

    sim.calculate_depths();

    // Initially: Addr=0, Trigger=0 -> CS0=0, CS1=0 -> Both domains sleep
    sim.set_input(in_addr, false);
    sim.set_input(in_trig, false);
    sim.propagate_events(100).expect("Initial propagation");

    assert!(sim.is_domain_sleeping(0));
    assert!(sim.is_domain_sleeping(1));

    // 1. Write High (1) to Bank 0 (Addr=0, Trigger=1, Data=1)
    sim.set_input(in_data, true);
    sim.set_input(in_addr, false);
    sim.set_input(in_trig, true);
    sim.propagate_events(100).expect("Write Bank 0 High");
    assert!(!sim.is_domain_sleeping(0)); // Bank 0 awake
    assert!(sim.is_domain_sleeping(1)); // Bank 1 sleeping
    assert!(sim.get_state(q0));

    // Latch Bank 0 (Trigger=0) -> Bank 0 hibernates into dense 1-bit buffer!
    sim.set_input(in_trig, false);
    sim.propagate_events(100).expect("Latch Bank 0");
    assert!(sim.is_domain_sleeping(0));
    assert!(sim.is_domain_sleeping(1));

    // 2. Write Low (0) to Bank 1 (Addr=1, Trigger=1, Data=0)
    sim.set_input(in_data, false);
    sim.set_input(in_addr, true);
    sim.set_input(in_trig, true);
    sim.propagate_events(100).expect("Write Bank 1 Low");
    assert!(sim.is_domain_sleeping(0)); // Bank 0 still asleep with High retained!
    assert!(!sim.is_domain_sleeping(1)); // Bank 1 awake
    assert!(!sim.get_state(q1));

    // Latch Bank 1 (Trigger=0)
    sim.set_input(in_trig, false);
    sim.propagate_events(100).expect("Latch Bank 1");
    assert!(sim.is_domain_sleeping(0));
    assert!(sim.is_domain_sleeping(1));

    // Assert Bank 0 preserved its High bit (Q output at index 2) in the dormant bitset buffer while asleep
    assert!(
        sim.sleep_domains[0].dormant_bits.contains(2),
        "Dormant buffer must retain Q bit (index 2)"
    );

    // 3. Wake Bank 0 again (Addr=0, Trigger=1) -> state High should be preserved!
    sim.set_input(in_data, true);
    sim.set_input(in_addr, false);
    sim.set_input(in_trig, true);
    sim.propagate_events(100).expect("Wake Bank 0");
    assert!(!sim.is_domain_sleeping(0));
    assert!(sim.get_state(q0));
}
