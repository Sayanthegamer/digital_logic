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

