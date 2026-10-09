use logic_simulator::engine::Simulator;
use logic_simulator::engine::sleep::SleepDomain;
use logic_simulator::engine::types::GateType;

#[test]
fn test_sleep_domain_pack_unpack_latch_bits() {
    let mut sim = Simulator::new();

    // Create control gate (Input)
    let ctrl = sim.add_gate(GateType::Input);

    // Create 8 latches (NAND gates) representing 8 bits of memory
    let mut latches = Vec::new();
    for i in 0..8 {
        let latch = sim.add_gate(GateType::Nand);
        // Alternate initial states: even = 1 (High: 0b10), odd = 0 (Low: 0b01)
        sim.nodes.states[latch] = if i % 2 == 0 { 0b10 } else { 0b01 };
        latches.push(latch);
    }

    let mut domain = SleepDomain::new(
        0,                // domain id
        ctrl,             // control gate
        true,             // active_high (High = awake)
        (1..9).collect(), // gates
        latches.clone(),
    );

    assert!(!domain.is_sleeping);

    // Hibernate domain into dense bit-array
    domain.hibernate(&mut sim);
    assert!(domain.is_sleeping);
    assert_eq!(domain.dormant_bits.len(), 8);

    // Assert packed bits: bits 0, 2, 4, 6 are 1, bits 1, 3, 5, 7 are 0
    for i in 0..8 {
        assert_eq!(
            domain.dormant_bits.contains(i),
            i % 2 == 0,
            "Bit {} mismatch in dormant buffer",
            i
        );
    }

    // Mutate simulator states while asleep (simulating idle clearing)
    for &latch in &latches {
        sim.nodes.states[latch] = 0b00;
    }

    // Wake domain up and assert exact latch state restoration
    domain.wake(&mut sim);
    assert!(!domain.is_sleeping);

    for (i, &latch) in latches.iter().enumerate() {
        let expected = if i % 2 == 0 { 0b10 } else { 0b01 };
        assert_eq!(
            sim.nodes.states[latch], expected,
            "Restored latch {} state mismatch",
            i
        );
    }
}

#[test]
fn test_activity_gated_bank_event_skipping() {
    let mut sim = Simulator::new();

    // Bank A: Enabled (awake)
    let en_a = sim.add_gate(GateType::Input);
    sim.set_input(en_a, true);

    let in_a = sim.add_gate(GateType::Input);
    let nand_a = sim.add_gate(GateType::Nand);
    sim.connect(in_a, nand_a, 0);
    sim.connect(in_a, nand_a, 1);

    // Bank B: Disabled (asleep)
    let en_b = sim.add_gate(GateType::Input);
    sim.set_input(en_b, false);

    let in_b = sim.add_gate(GateType::Input);
    let nand_b = sim.add_gate(GateType::Nand);
    sim.connect(in_b, nand_b, 0);
    sim.connect(in_b, nand_b, 1);

    // Register sleep domain for Bank B
    let domain_b = SleepDomain::new(1, en_b, true, vec![nand_b], vec![nand_b]);
    sim.register_sleep_domain(domain_b);

    sim.calculate_depths();
    sim.propagate_events(100).expect("Initial propagation");

    // Bank B should be sleeping
    assert!(sim.is_domain_sleeping(1));

    // Toggle in_b while Bank B is asleep
    sim.set_input(in_b, true);

    // Propagation should NOT queue or execute nand_b because its domain is asleep!
    let _steps = sim.propagate_events(100).expect("Propagation");
    // Only in_b should be processed; nand_b skipped
    assert!(!sim.nodes.in_queue.contains(nand_b));

    // Now enable Bank B: wake it up
    sim.set_input(en_b, true);
    let wake_steps = sim.propagate_events(100).expect("Wake propagation");
    assert!(wake_steps > 0);
    assert!(!sim.is_domain_sleeping(1));

    // Now nand_b should be awake and evaluated
    assert!(!sim.get_state(nand_b)); // NAND(1, 1) = 0
}
