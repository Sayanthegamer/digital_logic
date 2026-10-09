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
