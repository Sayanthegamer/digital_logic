use logic_simulator::editor::Editor;
use logic_simulator::editor::math::Vec2;
use logic_simulator::editor::types::{VisualComponent, VisualConnection};
use logic_simulator::engine::Simulator;
use logic_simulator::engine::compiler::compile_subchip_template;
use logic_simulator::engine::types::*;
use std::time::Instant;

#[test]
fn test_subchip_template_cache_speed_and_correctness() {
    // Construct a 1-bit full adder blueprint out of raw NAND gates
    // Inputs: A (0), B (1), Cin (2)
    // Outputs: Sum (0), Cout (1)
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
        }, // 2
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 3
        Component {
            component_type: ComponentType::Nand,
            pos: (0.0, 0.0),
            clock_period: None,
            bus_width: None,
        }, // 4: XOR1
    ];
    let connections = vec![
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
        Connection {
            source: SourcePort::ComponentOutput {
                component_idx: 0,
                port_idx: 0,
            },
            target: TargetPort::ChipOutput(0),
        },
    ];

    let adder_bp = ChipBlueprint {
        name: "TestAdder".to_string(),
        inputs: 2,
        outputs: 1,
        input_names: vec!["A".to_string(), "B".to_string()],
        output_names: vec!["Out".to_string()],
        components,
        connections,
    };

    let library = vec![adder_bp];

    // Compile template
    let template = compile_subchip_template(0, &library).expect("Failed to compile template");
    assert_eq!(template.gate_types.len(), 5);

    // Instantiate 256 instances using the template
    let mut sim = Simulator::new();
    let mut active_clocks = Vec::new();
    let start_inst = Instant::now();
    for _ in 0..256 {
        template.instantiate_into(&mut sim, &mut active_clocks);
    }
    let elapsed = start_inst.elapsed();

    // 256 instances * 5 gates = 1280 gates instantiated in low milliseconds
    assert_eq!(sim.nodes.len(), 256 * 5);
    assert!(
        elapsed.as_millis() < 50,
        "Template instantiation took too long: {:?}",
        elapsed
    );
}

#[test]
fn test_incremental_delta_compilation_preserves_subchips_and_patches_nets() {
    let mut editor = Editor::new();
    editor.circuit.components.clear();
    editor.circuit.connections.clear();

    // Index 0 in default library is "AND" (2 inputs, 1 output)
    editor.circuit.components.push(VisualComponent {
        id: 1,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 0.0),
        width: 70.0,
        height: 40.0,
        label: "A".to_string(),
        clock_period: None,
        color: None,
        bus_width: None,
    });
    editor.circuit.components.push(VisualComponent {
        id: 2,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 60.0),
        width: 70.0,
        height: 40.0,
        label: "B".to_string(),
        clock_period: None,
        color: None,
        bus_width: None,
    });
    editor.circuit.components.push(VisualComponent {
        id: 10,
        comp_type: ComponentType::SubChip(0),
        pos: Vec2::new(150.0, 20.0),
        width: 120.0,
        height: 72.0,
        label: "AND".to_string(),
        clock_period: None,
        color: None,
        bus_width: None,
    });
    editor.circuit.components.push(VisualComponent {
        id: 20,
        comp_type: ComponentType::Output,
        pos: Vec2::new(350.0, 20.0),
        width: 70.0,
        height: 40.0,
        label: "OUT".to_string(),
        clock_period: None,
        color: None,
        bus_width: None,
    });

    // Initial compile is a full compile
    editor.compile();
    assert!(
        !editor.engine.last_compile_was_incremental,
        "First compile must be a full compile"
    );

    // Record the SubChip's internal gate index before incremental edits
    let subchip_out_before = editor.engine.instance_tree.sub_instances[&10].outputs[0];

    // 1. Add wires connecting Inputs 1 & 2 -> SubChip(10) -> Output(20)
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 1,
        src_port: 0,
        tgt_comp_id: 10,
        tgt_port: 0,
    });
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 2,
        src_port: 0,
        tgt_comp_id: 10,
        tgt_port: 1,
    });
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 10,
        src_port: 0,
        tgt_comp_id: 20,
        tgt_port: 0,
    });

    editor.compile();
    assert!(
        editor.engine.last_compile_was_incremental,
        "Wiring edits must execute O(Delta) incremental delta-compilation"
    );
    assert_eq!(
        editor.engine.instance_tree.sub_instances[&10].outputs[0], subchip_out_before,
        "SubChip internal gate indices must remain untouched during incremental compilation"
    );

    // Set both inputs High -> AND output must be High
    let g_a = editor.engine.visual_to_sim_map[&1];
    let g_b = editor.engine.visual_to_sim_map[&2];
    editor.engine.simulator.set_input(g_a, true);
    editor.engine.simulator.set_input(g_b, true);
    editor.engine.simulator.propagate_events(100).unwrap();

    let g_out = editor.engine.visual_to_sim_map[&20];
    assert!(editor.engine.simulator.get_state(g_out));

    // 2. Add a primitive inverter (Nand gate id 30) between SubChip(10) and Output(20)
    editor.circuit.connections.retain(|c| c.tgt_comp_id != 20);
    editor.circuit.components.push(VisualComponent {
        id: 30,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(260.0, 20.0),
        width: 70.0,
        height: 40.0,
        label: "INV".to_string(),
        clock_period: None,
        color: None,
        bus_width: None,
    });
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 10,
        src_port: 0,
        tgt_comp_id: 30,
        tgt_port: 0,
    });
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 10,
        src_port: 0,
        tgt_comp_id: 30,
        tgt_port: 1,
    });
    editor.circuit.connections.push(VisualConnection {
        src_comp_id: 30,
        src_port: 0,
        tgt_comp_id: 20,
        tgt_port: 0,
    });

    editor.compile();
    assert!(
        editor.engine.last_compile_was_incremental,
        "Adding a primitive gate and rewiring must use incremental delta-compilation"
    );
    assert!(
        !editor.engine.simulator.get_state(g_out),
        "Inverted AND(1, 1) must be Low (0)"
    );
}
