use crate::editor::Editor;
use crate::editor::types::VisualComponent;
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
    });
    editor.components.push(VisualComponent {
        id: 2,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 50.0),
        width: 70.0,
        height: 40.0,
        label: "X".to_string(),
        clock_period: None,
    });
    editor.components.push(VisualComponent {
        id: 3,
        comp_type: ComponentType::Input,
        pos: Vec2::new(0.0, 100.0),
        width: 70.0,
        height: 40.0,
        label: "IN".to_string(),
        clock_period: None,
    });

    editor.components.push(VisualComponent {
        id: 4,
        comp_type: ComponentType::Output,
        pos: Vec2::new(200.0, 0.0),
        width: 70.0,
        height: 40.0,
        label: "Y".to_string(),
        clock_period: None,
    });
    editor.components.push(VisualComponent {
        id: 5,
        comp_type: ComponentType::Output,
        pos: Vec2::new(200.0, 50.0),
        width: 70.0,
        height: 40.0,
        label: "Y".to_string(),
        clock_period: None,
    });

    editor.components.push(VisualComponent {
        id: 6,
        comp_type: ComponentType::Nand,
        pos: Vec2::new(100.0, 25.0),
        width: 70.0,
        height: 40.0,
        label: "NAND".to_string(),
        clock_period: None,
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
