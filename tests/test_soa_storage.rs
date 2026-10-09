use logic_simulator::engine::storage::{NO_SOURCE, SoAGateStorage};
use logic_simulator::engine::types::GateType;

#[test]
fn test_soa_gate_allocation_and_defaults() {
    let mut storage = SoAGateStorage::new();
    assert_eq!(storage.len(), 0);
    assert!(storage.is_empty());

    let g_nand = storage.insert(GateType::Nand, 0b10);
    let g_in = storage.insert(GateType::Input, 0b01);
    let g_out = storage.insert(GateType::Output, 0b01);
    let g_tri = storage.insert(GateType::TriStateBuffer, 0b00);
    let g_res = storage.insert(GateType::BusResolver, 0b00);

    assert_eq!(storage.len(), 5);
    assert_eq!(g_nand, 0);
    assert_eq!(g_in, 1);
    assert_eq!(g_out, 2);
    assert_eq!(g_tri, 3);
    assert_eq!(g_res, 4);

    assert_eq!(storage.states[g_nand], 0b10);
    assert_eq!(storage.states[g_in], 0b01);
    assert_eq!(storage.states[g_out], 0b01);
    assert_eq!(storage.states[g_tri], 0b00);
    assert_eq!(storage.states[g_res], 0b00);

    assert_eq!(storage.gate_types[g_nand], GateType::Nand);
    assert_eq!(storage.gate_types[g_in], GateType::Input);
    assert_eq!(storage.gate_types[g_out], GateType::Output);
    assert_eq!(storage.gate_types[g_tri], GateType::TriStateBuffer);
    assert_eq!(storage.gate_types[g_res], GateType::BusResolver);

    // Initial sources must be NO_SOURCE
    assert_eq!(storage.sources[g_nand], [NO_SOURCE, NO_SOURCE]);
    assert_eq!(storage.sources[g_tri], [NO_SOURCE, NO_SOURCE]);

    // Initial dependents must be empty
    assert!(storage.dependents[g_nand].is_empty());
    assert!(storage.dependents[g_in].is_empty());

    // Initially in_queue must be false
    assert!(!storage.in_queue.contains(g_nand));
}

#[test]
fn test_soa_connections_and_dependents() {
    let mut storage = SoAGateStorage::new();
    let src = storage.insert(GateType::Input, 0b01);
    let tgt = storage.insert(GateType::Nand, 0b10);

    storage.connect(src, tgt, 0);
    assert_eq!(storage.sources[tgt][0], src as u32);
    assert_eq!(storage.sources[tgt][1], NO_SOURCE);
    assert_eq!(storage.dependents[src], vec![tgt as u32]);

    storage.connect(src, tgt, 1);
    assert_eq!(storage.sources[tgt][1], src as u32);
    assert_eq!(storage.dependents[src], vec![tgt as u32, tgt as u32]);
}

#[test]
fn test_soa_removal_and_freelist_reuse() {
    let mut storage = SoAGateStorage::new();
    let g0 = storage.insert(GateType::Input, 0b01);
    let g1 = storage.insert(GateType::Nand, 0b10);
    let g2 = storage.insert(GateType::Output, 0b01);

    storage.connect(g0, g1, 0);
    storage.connect(g1, g2, 0);

    assert_eq!(storage.len(), 3);
    assert!(storage.contains(g1));

    storage.remove(g1);
    assert_eq!(storage.len(), 2);
    assert!(!storage.contains(g1));

    // Inserting a new gate should reuse g1's recycled slot
    let g_reused = storage.insert(GateType::Nand, 0b10);
    assert_eq!(g_reused, g1);
    assert_eq!(storage.len(), 3);
    assert!(storage.contains(g_reused));
}

#[test]
fn test_soa_memory_density_assertion() {
    // 100,000 gates memory footprint benchmark
    let mut storage = SoAGateStorage::with_capacity(100_000);
    for _ in 0..100_000 {
        storage.insert(GateType::Nand, 0b10);
    }
    assert_eq!(storage.len(), 100_000);

    // Verify hot data (states + depths + in_queue bits) byte footprint
    let states_bytes = storage.states.len() * std::mem::size_of::<u8>();
    let depths_bytes = storage.depths.len() * std::mem::size_of::<u32>();
    let in_queue_bytes = std::mem::size_of_val(storage.in_queue.as_slice());

    let hot_bytes = states_bytes + depths_bytes + in_queue_bytes;
    // 100k bytes + 400k bytes + 12.5k bytes ≈ 512.5 KB
    assert!(
        hot_bytes < 1_000_000,
        "Hot data exceeded 1MB for 100k gates! Found {} bytes",
        hot_bytes
    );

    // Verify total base gate storage (including cold topology sources)
    let sources_bytes = storage.sources.len() * std::mem::size_of::<[u32; 2]>();
    let gate_types_bytes = storage.gate_types.len() * std::mem::size_of::<GateType>();
    let total_base_bytes = hot_bytes + sources_bytes + gate_types_bytes;

    // Total base storage for 100,000 gates should be ~1.4 MB (< 2.0 MB)
    assert!(
        total_base_bytes < 2_000_000,
        "Base storage exceeded 2MB for 100k gates! Found {} bytes",
        total_base_bytes
    );
}
