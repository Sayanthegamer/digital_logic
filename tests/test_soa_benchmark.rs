use logic_simulator::engine::Simulator;
use logic_simulator::engine::types::GateType;
use std::time::Instant;

#[test]
fn test_soa_100k_gates_memory_and_throughput() {
    let mut sim = Simulator::new();
    sim.set_single_threaded(true); // Isolate pure SoA sequential evaluation throughput

    let _start_alloc = Instant::now();
    // Build a 100,000 inverter cascade
    let mut prev = sim.add_gate(GateType::Input);
    for _ in 0..100_000 {
        let nand = sim.add_gate(GateType::Nand);
        sim.connect(prev, nand, 0);
        sim.connect(prev, nand, 1);
        prev = nand;
    }
    // Assert capacity and allocation metrics
    assert!(sim.nodes.len() >= 100_001);

    // Verify memory density:
    let states_bytes = sim.nodes.states.len();
    let depths_bytes = sim.nodes.depths.len() * 4;
    let in_queue_bytes = std::mem::size_of_val(sim.nodes.in_queue.as_slice());
    let sources_bytes = sim.nodes.sources.len() * 8;
    let types_bytes = sim.nodes.gate_types.len();

    let total_base_bytes =
        states_bytes + depths_bytes + in_queue_bytes + sources_bytes + types_bytes;
    let bytes_per_gate = total_base_bytes as f64 / sim.nodes.len() as f64;

    assert!(
        bytes_per_gate <= 16.0,
        "Memory per gate exceeded 16 bytes! Found: {:.2} bytes",
        bytes_per_gate
    );

    // Compute topological depths
    sim.calculate_depths();

    // Defragment & sort by depth
    sim.defragment_and_sort_by_depth();

    // Toggle input and measure event propagation
    sim.set_input(0, true);
    let steps = sim.propagate_events(200).expect("Propagation failed");
    assert!(
        steps > 100_000,
        "Should propagate through all 100k inverter stages"
    );
}
