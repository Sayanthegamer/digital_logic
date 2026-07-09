use crate::editor::Editor;
use crate::editor::types::{TextAnnotation, VisualComponent, VisualConnection};
use crate::engine::ComponentType;
use macroquad::prelude::*;

#[test]
fn test_custom_port_naming_collision() {
    let mut editor = Editor::new();
    editor.engine.library.clear();

    editor.components.push(VisualComponent {
        id: 1,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 0.0),
        width: 70.0,
        height: 40.0,
        label: "X".to_string(),
        clock_period: None,
        color: None,
    });
    editor.components.push(VisualComponent {
        id: 2,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 50.0),
        width: 70.0,
        height: 40.0,
        label: "X".to_string(),
        clock_period: None,
        color: None,
    });
    editor.components.push(VisualComponent {
        id: 3,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 100.0),
        width: 70.0,
        height: 40.0,
        label: "IN".to_string(),
        clock_period: None,
        color: None,
    });

    editor.components.push(VisualComponent {
        id: 4,
        comp_type: ComponentType::Output,
        pos: Vec2::new(200.0, 0.0),
        width: 70.0,
        height: 40.0,
        label: "Y".to_string(),
        clock_period: None,
        color: None,
    });
    editor.components.push(VisualComponent {
        id: 5,
        comp_type: ComponentType::Output,
        pos: Vec2::new(200.0, 50.0),
        width: 70.0,
        height: 40.0,
        label: "Y".to_string(),
        clock_period: None,
        color: None,
    });

    editor.components.push(VisualComponent {
        id: 6,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(100.0, 25.0),
        width: 70.0,
        height: 40.0,
        label: "NAND".to_string(),
        clock_period: None,
        color: None,
    });

    let bp = editor
        .package_current_canvas()
        .expect("Failed to package canvas");

    assert_eq!(bp.input_names.len(), 3);
    assert_eq!(bp.input_names[0], "X");
    assert_eq!(bp.input_names[1], "X_1");
    assert_eq!(bp.input_names[2], "IN_2");

    assert_eq!(bp.output_names.len(), 2);
    assert_eq!(bp.output_names[0], "Y");
    assert_eq!(bp.output_names[1], "Y_1");
}

// --- Port Position Tests ---

/// Helper to create a simple non-Junction component at a given position/size.
fn make_comp(x: f32, y: f32, w: f32, h: f32) -> VisualComponent {
    VisualComponent {
        id: 1,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(x, y),
        width: w,
        height: h,
        label: "TEST".to_string(),
        clock_period: None,
        color: None,
    }
}

fn make_junction(x: f32, y: f32, w: f32, h: f32) -> VisualComponent {
    VisualComponent {
        id: 1,
        comp_type: ComponentType::Junction,
        pos: Vec2::new(x, y),
        width: w,
        height: h,
        label: "".to_string(),
        clock_period: None,
        color: None,
    }
}

// ── input_port_pos ──

#[test]
fn test_input_port_pos_single() {
    let comp = make_comp(100.0, 200.0, 70.0, 60.0);
    // 1 input: spacing = 60 / 2 = 30, y = 200 + 30 = 230
    let pos = comp.input_port_pos(0, 1);
    assert_eq!(pos, Vec2::new(100.0, 230.0));
}

#[test]
fn test_input_port_pos_multiple() {
    let comp = make_comp(100.0, 200.0, 70.0, 90.0);
    // 2 inputs: spacing = 90 / 3 = 30
    assert_eq!(comp.input_port_pos(0, 2), Vec2::new(100.0, 230.0));
    assert_eq!(comp.input_port_pos(1, 2), Vec2::new(100.0, 260.0));
}

#[test]
fn test_input_port_pos_zero_inputs() {
    let comp = make_comp(100.0, 200.0, 70.0, 60.0);
    // Fallback: returns top-left corner (self.pos)
    assert_eq!(comp.input_port_pos(0, 0), Vec2::new(100.0, 200.0));
}

#[test]
fn test_input_port_pos_junction_horizontal() {
    let junc = make_junction(50.0, 80.0, 40.0, 12.0); // wider than tall
    // Input = left end, vertically centered
    assert_eq!(junc.input_port_pos(0, 1), Vec2::new(50.0, 86.0));
}

#[test]
fn test_input_port_pos_junction_vertical() {
    let junc = make_junction(50.0, 80.0, 12.0, 40.0); // taller than wide
    // Input = top end, horizontally centered
    assert_eq!(junc.input_port_pos(0, 1), Vec2::new(56.0, 80.0));
}

// ── output_port_pos ──

#[test]
fn test_output_port_pos_single() {
    let comp = make_comp(100.0, 200.0, 70.0, 60.0);
    // 1 output: spacing = 60 / 2 = 30, y = 200 + 30 = 230
    let pos = comp.output_port_pos(0, 1);
    assert_eq!(pos, Vec2::new(170.0, 230.0));
}

#[test]
fn test_output_port_pos_multiple() {
    let comp = make_comp(100.0, 200.0, 70.0, 90.0);
    // 2 outputs: spacing = 90 / 3 = 30
    assert_eq!(comp.output_port_pos(0, 2), Vec2::new(170.0, 230.0));
    assert_eq!(comp.output_port_pos(1, 2), Vec2::new(170.0, 260.0));
}

#[test]
fn test_output_port_pos_zero_outputs() {
    let comp = make_comp(100.0, 200.0, 70.0, 60.0);
    // Fallback: top-right corner (pos + (width, 0))
    assert_eq!(comp.output_port_pos(0, 0), Vec2::new(170.0, 200.0));
}

#[test]
fn test_output_port_pos_junction_horizontal() {
    let junc = make_junction(50.0, 80.0, 40.0, 12.0); // wider than tall
    // Output = right end, vertically centered
    assert_eq!(junc.output_port_pos(0, 1), Vec2::new(90.0, 86.0));
}

#[test]
fn test_output_port_pos_junction_vertical() {
    let junc = make_junction(50.0, 80.0, 12.0, 40.0); // taller than wide
    // Output = bottom end, horizontally centered
    assert_eq!(junc.output_port_pos(0, 1), Vec2::new(56.0, 120.0));
}

#[test]
fn test_coordinate_transformation() {
    let mut editor = Editor::new();

    // Test default zoom=1.0, pan=(0,0)
    editor.canvas.pan = Vec2::new(0.0, 0.0);
    editor.canvas.zoom = 1.0;

    let world_p1 = Vec2::new(15.0, 25.0);
    assert_eq!(editor.to_screen_space(world_p1), world_p1);
    assert_eq!(editor.to_world_space(world_p1), world_p1);

    // Test with non-trivial pan and zoom
    editor.canvas.pan = Vec2::new(100.0, -50.0);
    editor.canvas.zoom = 2.0;

    let world_p2 = Vec2::new(10.0, 20.0);
    // expected screen = 10 * 2 + 100 = 120, 20 * 2 - 50 = -10
    let screen_p2 = editor.to_screen_space(world_p2);
    assert_eq!(screen_p2, Vec2::new(120.0, -10.0));

    let recovered_world_p2 = editor.to_world_space(screen_p2);
    assert_eq!(recovered_world_p2, world_p2);
}

#[test]
fn test_get_component_ports_count() {
    let mut editor = Editor::new();
    editor.engine.library.clear();

    // Primitives
    assert_eq!(
        editor.get_component_ports_count(ComponentType::Nand),
        (2, 1)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::Input),
        (0, 1)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::Output),
        (1, 0)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::Clock),
        (0, 1)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::SevenSegment),
        (8, 0)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::TriStateBuffer),
        (2, 1)
    );
    assert_eq!(
        editor.get_component_ports_count(ComponentType::Junction),
        (1, 1)
    );

    // SubChip missing in library
    assert_eq!(
        editor.get_component_ports_count(ComponentType::SubChip(0)),
        (0, 0)
    );

    // SubChip present in library
    editor.engine.library.push(crate::engine::ChipBlueprint {
        name: "MyChip".to_string(),
        inputs: 4,
        outputs: 3,
        input_names: vec![],
        output_names: vec![],
        components: vec![],
        connections: vec![],
    });

    assert_eq!(
        editor.get_component_ports_count(ComponentType::SubChip(0)),
        (4, 3)
    );
    // Invalid index
    assert_eq!(
        editor.get_component_ports_count(ComponentType::SubChip(99)),
        (0, 0)
    );
}

#[test]
fn test_save_load_project() {
    let mut editor = Editor::new();
    editor.engine.library.clear();
    editor.components.clear();
    editor.connections.clear();
    editor.annotations.clear();

    // Populate with some data
    editor.next_component_id = 42;
    editor.components.push(VisualComponent {
        id: 1,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(10.0, 20.0),
        width: 70.0,
        height: 40.0,
        label: "NAND_G".to_string(),
        clock_period: None,
        color: None,
    });
    editor.connections.push(VisualConnection {
        src_comp_id: 1,
        src_port: 0,
        tgt_comp_id: 2,
        tgt_port: 1,
    });
    editor.annotations.push(TextAnnotation {
        text: "Testing save load".to_string(),
        pos: Vec2::new(50.0, 60.0),
    });
    editor.engine.library.push(crate::engine::ChipBlueprint {
        name: "CustomChip".to_string(),
        inputs: 2,
        outputs: 1,
        input_names: vec!["A".to_string(), "B".to_string()],
        output_names: vec!["Y".to_string()],
        components: vec![],
        connections: vec![],
    });

    // Create a temporary path
    let mut temp_path = std::env::temp_dir();
    temp_path.push("test_logic_simulator_project_save.json");

    // Save project
    editor.save_to_path(&temp_path).unwrap();
    assert!(temp_path.exists());

    // Create a new editor and load
    let mut loaded_editor = Editor::new();
    // Clear the global library to ensure predictable state for the test
    loaded_editor.global_library = crate::editor::global_library::GlobalLibrary::default();
    loaded_editor.engine.library.clear();
    loaded_editor.components.clear();
    loaded_editor.connections.clear();
    loaded_editor.annotations.clear();

    let load_success = loaded_editor.load_from_path(&temp_path);
    assert!(load_success);

    // Verify properties
    assert_eq!(loaded_editor.next_component_id, editor.next_component_id);
    assert_eq!(loaded_editor.components.len(), 1);
    assert_eq!(loaded_editor.components[0].id, 1);
    assert_eq!(loaded_editor.components[0].label, "NAND_G");
    assert_eq!(loaded_editor.components[0].pos, Vec2::new(10.0, 20.0));

    assert_eq!(loaded_editor.connections.len(), 1);
    assert_eq!(loaded_editor.connections[0].src_comp_id, 1);
    assert_eq!(loaded_editor.connections[0].tgt_comp_id, 2);

    assert_eq!(loaded_editor.annotations.len(), 1);
    assert_eq!(loaded_editor.annotations[0].text, "Testing save load");

    // After import, the global library should contain the custom chip
    assert!(loaded_editor.engine.library.iter().any(|bp| bp.name == "CustomChip"));

    // Clean up temporary file
    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_save_load_nested_project() {
    let mut editor = Editor::new();
    // Start with a clean library and editor state
    editor.global_library = crate::editor::global_library::GlobalLibrary::default();
    editor.engine.library.clear();
    editor.components.clear();
    editor.connections.clear();
    editor.annotations.clear();

    // 1. Add "InnerChip"
    editor.engine.library.push(crate::engine::ChipBlueprint {
        name: "InnerChip".to_string(),
        inputs: 1,
        outputs: 1,
        input_names: vec!["IN".to_string()],
        output_names: vec!["OUT".to_string()],
        components: vec![],
        connections: vec![],
    });

    // 2. Add "OuterChip" which contains "InnerChip" (SubChip(0))
    editor.engine.library.push(crate::engine::ChipBlueprint {
        name: "OuterChip".to_string(),
        inputs: 1,
        outputs: 1,
        input_names: vec!["IN".to_string()],
        output_names: vec!["OUT".to_string()],
        components: vec![
            crate::engine::Component {
                component_type: ComponentType::SubChip(0), // References InnerChip
                pos: (0.0, 0.0),
                clock_period: None,
            }
        ],
        connections: vec![],
    });

    // 3. Add a canvas component of type "OuterChip" (SubChip(1))
    editor.components.push(VisualComponent {
        id: 1,
        comp_type: ComponentType::SubChip(1), // OuterChip
        pos: Vec2::new(10.0, 20.0),
        width: 100.0,
        height: 100.0,
        label: "Outer".to_string(),
        clock_period: None,
        color: None,
    });

    let mut temp_path = std::env::temp_dir();
    temp_path.push("test_logic_simulator_nested_project_save.json");

    // Save project
    editor.save_to_path(&temp_path).unwrap();

    // Create a new loaded_editor
    let mut loaded_editor = Editor::new();
    loaded_editor.global_library = crate::editor::global_library::GlobalLibrary::default();
    // Pre-populate global library with a dummy chip
    loaded_editor.global_library.ungrouped.push(crate::engine::ChipBlueprint {
        name: "DummyChip".to_string(),
        inputs: 0,
        outputs: 0,
        input_names: vec![],
        output_names: vec![],
        components: vec![],
        connections: vec![],
    });
    loaded_editor.engine.library.clear();
    loaded_editor.components.clear();
    loaded_editor.connections.clear();
    loaded_editor.annotations.clear();

    // Load project
    let load_success = loaded_editor.load_from_path(&temp_path);
    assert!(load_success);

    // After import, the global library should contain DummyChip, InnerChip and OuterChip
    let flat_lib = loaded_editor.engine.library.clone();
    assert!(flat_lib.iter().any(|bp| bp.name == "DummyChip"));
    assert!(flat_lib.iter().any(|bp| bp.name == "InnerChip"));
    assert!(flat_lib.iter().any(|bp| bp.name == "OuterChip"));
    let initial_len = loaded_editor.global_library.to_flat_list().len();
    assert_eq!(initial_len, 3); // Dummy, Inner, Outer

    // Load project a second time to verify idempotency
    let load_success2 = loaded_editor.load_from_path(&temp_path);
    assert!(load_success2);
    let second_len = loaded_editor.global_library.to_flat_list().len();
    assert_eq!(second_len, 3, "Global library should not duplicate chips on multiple loads when pre-populated!");

    // Verify canvas components
    assert_eq!(loaded_editor.components.len(), 1);
    if let ComponentType::SubChip(new_idx) = loaded_editor.components[0].comp_type {
        // Find OuterChip index in the loaded flat library
        let outer_idx = flat_lib.iter().position(|bp| bp.name == "OuterChip").unwrap();
        assert_eq!(new_idx, outer_idx);

        // Verify the components inside OuterChip itself
        let outer_bp = &flat_lib[outer_idx];
        assert_eq!(outer_bp.components.len(), 1);
        if let ComponentType::SubChip(inner_idx) = outer_bp.components[0].component_type {
            let real_inner_idx = flat_lib.iter().position(|bp| bp.name == "InnerChip").unwrap();
            assert_eq!(inner_idx, real_inner_idx);
        } else {
            panic!("Expected InnerChip subchip component inside OuterChip blueprint");
        }
    } else {
        panic!("Expected SubChip on the canvas");
    }

    // Clean up temporary file
    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_hit_test_manhattan_wire_zoom() {
    let mut editor = Editor::new();
    editor.canvas.zoom = 2.0;

    // A wire from (100.0, 100.0) to (130.0, 100.0) in world space
    let src = Vec2::new(100.0, 100.0);
    let tgt = Vec2::new(130.0, 100.0);

    // In world space, tgt.x (130) >= src.x (100) + 20.0 (120)
    // So this routes as a 3-segment horizontal-vertical-horizontal wire.
    // The segments are:
    // 1. (100, 100) -> (115, 100)
    // 2. (115, 100) -> (115, 100) (zero-length vertical)
    // 3. (115, 100) -> (130, 100)
    // Thus, (110.0, 100.0) is on segment 1.
    // Under the zoom-corrected logic, hit testing at (110.0, 100.0) should succeed.
    let click_point = Vec2::new(110.0, 100.0);
    let hit = editor.hit_test_manhattan_wire(src, tgt, 0.0, 0, click_point, 2.0);
    assert!(hit);
}

#[test]
fn test_bus_compilation_and_propagation() {
    let mut editor = Editor::new();
    editor.components.clear();
    editor.connections.clear();

    // 1. Create components
    // Inputs (comp IDs 0..4)
    for i in 0..4 {
        editor.components.push(VisualComponent {
            id: i,
            comp_type: ComponentType::Input,
            pos: Vec2::new(0.0, i as f32 * 50.0),
            width: 70.0,
            height: 40.0,
            label: format!("IN_{}", i),
            clock_period: None,
            color: None,
        });
    }

    // BusJoiner (comp ID 4)
    editor.components.push(VisualComponent {
        id: 4,
        comp_type: ComponentType::BusJoiner,
        pos: Vec2::new(150.0, 100.0),
        width: 50.0,
        height: 104.0, // 40.0 + 4 * 16.0
        label: "JOIN".to_string(),
        clock_period: Some(4), // width = 4
        color: None,
    });

    // BusSplitter (comp ID 5)
    editor.components.push(VisualComponent {
        id: 5,
        comp_type: ComponentType::BusSplitter,
        pos: Vec2::new(300.0, 100.0),
        width: 50.0,
        height: 104.0,
        label: "SPLIT".to_string(),
        clock_period: Some(4), // width = 4
        color: None,
    });

    // Outputs (comp IDs 6..10)
    for i in 0..4 {
        editor.components.push(VisualComponent {
            id: 6 + i,
            comp_type: ComponentType::Output,
            pos: Vec2::new(450.0, i as f32 * 50.0),
            width: 70.0,
            height: 40.0,
            label: format!("OUT_{}", i),
            clock_period: None,
            color: None,
        });
    }

    // 2. Add connections
    // Connect Inputs to BusJoiner
    for i in 0..4 {
        editor.connections.push(VisualConnection {
            src_comp_id: i,
            src_port: 0,
            tgt_comp_id: 4,
            tgt_port: i,
        });
    }

    // Connect BusJoiner to BusSplitter (The Bus wire)
    editor.connections.push(VisualConnection {
        src_comp_id: 4,
        src_port: 0,
        tgt_comp_id: 5,
        tgt_port: 0,
    });

    // Connect BusSplitter to Outputs
    for i in 0..4 {
        editor.connections.push(VisualConnection {
            src_comp_id: 5,
            src_port: i,
            tgt_comp_id: 6 + i,
            tgt_port: 0,
        });
    }

    // 3. Compile
    editor.compile();

    // Verify compilation succeeded without errors
    assert!(editor.engine.propagation_error.is_none());

    // 4. Test signal propagation
    // Toggle inputs: IN_0 = true, IN_1 = false, IN_2 = true, IN_3 = false
    let input_states = [true, false, true, false];
    for i in 0..4 {
        if let Some(&sim_idx) = editor.engine.visual_to_sim_map.get(&i) {
            editor.engine.simulator.set_input(sim_idx, input_states[i]);
        }
    }

    // Run propagation
    let propagate_ok = editor.engine.simulator.propagate_events(100).is_ok();
    assert!(propagate_ok);

    // Verify output values match input values
    for i in 0..4 {
        if let Some(&sim_idx) = editor.engine.visual_to_sim_map.get(&(6 + i)) {
            let output_val = editor.engine.simulator.get_raw_state(sim_idx);
            let expected_val = if input_states[i] { 0b10 } else { 0b01 };
            assert_eq!(output_val, expected_val, "Output {} state mismatch!", i);
        } else {
            panic!("Output {} not compiled in visual_to_sim_map!", i);
        }
    }
}

#[test]
fn test_seven_segment_top_level_port_allocation() {
    let mut editor = Editor::new();
    editor.components.clear();
    editor.connections.clear();

    // SevenSegment (comp ID 0)
    editor.components.push(VisualComponent {
        id: 0,
        comp_type: ComponentType::SevenSegment,
        pos: Vec2::new(100.0, 100.0),
        width: 60.0,
        height: 90.0,
        label: "7SEG".to_string(),
        clock_period: None,
        color: None,
    });

    // 8 Inputs (comp IDs 1..=8)
    for i in 1..=8 {
        editor.components.push(VisualComponent {
            id: i,
            comp_type: ComponentType::Input,
            pos: Vec2::new(0.0, (i - 1) as f32 * 50.0),
            width: 70.0,
            height: 40.0,
            label: format!("IN_{}", i - 1),
            clock_period: None,
            color: None,
        });
    }

    // Connect Inputs 1..=8 to SevenSegment ports 0..7
    for i in 1..=8 {
        editor.connections.push(VisualConnection {
            src_comp_id: i,
            src_port: 0,
            tgt_comp_id: 0,
            tgt_port: i - 1,
        });
    }

    // Compile
    editor.compile();

    // Verify compilation succeeded without errors
    assert!(editor.engine.propagation_error.is_none());

    // Toggle 8th input (minus segment, port 7)
    let in_8_id = 8;
    let sim_idx = *editor.engine.visual_to_sim_map.get(&in_8_id).expect("Input 8 not mapped");
    editor.engine.simulator.set_input(sim_idx, true);

    // Propagate signals
    let propagate_ok = editor.engine.simulator.propagate_events(100).is_ok();
    assert!(propagate_ok);

    // Verify that the minus segment (port index 7) state in the simulator is true.
    let dependents = &editor.engine.simulator.dependents[sim_idx];
    assert!(!dependents.is_empty(), "Input 8 has no dependents wired up!");
    
    let target_gate_idx = dependents[0];
    let state = editor.engine.simulator.get_state(target_gate_idx);
    assert!(state, "The 8th input segment (minus sign) did not receive the signal!");
}

#[test]
fn test_svg_export() {
    let mut editor = Editor::new();
    editor.components.clear();
    editor.connections.clear();

    // Add Nand Gate (comp ID 0)
    editor.components.push(VisualComponent {
        id: 0,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(100.0, 100.0),
        width: 70.0,
        height: 40.0,
        label: "NAND".to_string(),
        clock_period: None,
        color: None,
    });

    // Add Input (comp ID 1)
    editor.components.push(VisualComponent {
        id: 1,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 100.0),
        width: 70.0,
        height: 40.0,
        label: "IN".to_string(),
        clock_period: None,
        color: None,
    });

    // Wire them up
    editor.connections.push(VisualConnection {
        src_comp_id: 1,
        src_port: 0,
        tgt_comp_id: 0,
        tgt_port: 0,
    });

    // Compile
    editor.compile();

    // Export to temporary file path
    let mut svg_path = std::env::temp_dir();
    svg_path.push("test_logic_simulator_project_export.svg");
    
    let res = editor.export_svg_to_path(&svg_path);
    assert!(res.is_ok(), "Failed to export SVG: {:?}", res);

    // Verify SVG file content
    assert!(svg_path.exists(), "SVG file does not exist");
    let svg_content = std::fs::read_to_string(&svg_path).expect("Failed to read SVG file");
    
    assert!(svg_content.starts_with("<svg"), "SVG should start with <svg tag");
    assert!(svg_content.ends_with("</svg>\n") || svg_content.ends_with("</svg>"), "SVG should end with </svg>");
    
    // Check for key elements
    assert!(svg_content.contains("rect class=\"bg\""), "SVG should contain canvas background");
    assert!(svg_content.contains("class=\"component\""), "SVG should contain component body");
    assert!(svg_content.contains("class=\"wire\""), "SVG should contain wire path");
    assert!(svg_content.contains("<circle class=\"port\""), "SVG should contain port circles");
    assert!(svg_content.contains("NAND"), "SVG should contain text labels");

    // Clean up
    let _ = std::fs::remove_file(&svg_path);
}

