use super::Editor;
use crate::engine::ComponentType;
use crate::editor::types::{VisualComponent, VisualConnection};
use macroquad::prelude::Vec2;

impl Editor {
    /// Generates a massive recursively packaged synthetic circuit
    /// to stress test the Compiler, Chip Packaging, and Sub-Chip Hot-Reloading.
    pub fn generate_stress_test(&mut self, size: usize) {
        self.circuit.components.clear();
        self.circuit.connections.clear();
        self.circuit.next_component_id = 1;
        self.circuit.wire_offsets.clear();
        self.circuit.wire_nudges.clear();
        self.circuit.annotations.clear();
        
        // Clear previous test library
        self.engine.library.clear();
        
        // 1. Base Blueprint: A simple "1-bit logic" made of 4 NANDs (e.g., XOR gate)
        let base_bp = crate::engine::ChipBlueprint {
            name: "Level_0_XOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["Out".to_string()],
            components: vec![
                crate::engine::Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None, bus_width: None },
                crate::engine::Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None, bus_width: None },
                crate::engine::Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None, bus_width: None },
                crate::engine::Component { component_type: ComponentType::Nand, pos: (0.0, 0.0), clock_period: None, bus_width: None },
            ],
            connections: vec![
                // A -> Nand0, B -> Nand0
                crate::engine::Connection { source: crate::engine::SourcePort::ChipInput(0), target: crate::engine::TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                crate::engine::Connection { source: crate::engine::SourcePort::ChipInput(1), target: crate::engine::TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                
                // Nand0 -> Nand1, A -> Nand1
                crate::engine::Connection { source: crate::engine::SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: crate::engine::TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                crate::engine::Connection { source: crate::engine::SourcePort::ChipInput(0), target: crate::engine::TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                
                // Nand0 -> Nand2, B -> Nand2
                crate::engine::Connection { source: crate::engine::SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: crate::engine::TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                crate::engine::Connection { source: crate::engine::SourcePort::ChipInput(1), target: crate::engine::TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                
                // Nand1 -> Nand3, Nand2 -> Nand3
                crate::engine::Connection { source: crate::engine::SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: crate::engine::TargetPort::ComponentInput { component_idx: 3, port_idx: 0 } },
                crate::engine::Connection { source: crate::engine::SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: crate::engine::TargetPort::ComponentInput { component_idx: 3, port_idx: 1 } },
                
                // Nand3 -> Out
                crate::engine::Connection { source: crate::engine::SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 }, target: crate::engine::TargetPort::ChipOutput(0) },
            ],
        };
        self.engine.library.push(base_bp);

        // 2. Recursively package! Each level contains 4 instances of the previous level.
        // Level 0: 4 NANDs
        // Level 1: 4 * 4 = 16 NANDs
        // Level 2: 4 * 16 = 64 NANDs
        // Level 3: 4 * 64 = 256 NANDs
        // Level 4: 4 * 256 = 1,024 NANDs
        // Level 5: 4 * 1024 = 4,096 NANDs
        // Level 6: 4 * 4096 = 16,384 NANDs
        
        let target_depth = size.max(1);
        for depth in 1..=target_depth {
            let prev_idx = depth - 1;
            let mut components = Vec::new();
            let mut connections = Vec::new();
            
            // Add 4 sub-chips of the previous depth
            for i in 0..4 {
                components.push(crate::engine::Component {
                    component_type: ComponentType::SubChip(prev_idx),
                    pos: (0.0, 0.0),
                    clock_period: None,
            bus_width: None,
                });
                
                // Chain them together: ChipInput(0) -> Sub0(0), Sub0(Out) -> Sub1(0) ...
                if i == 0 {
                    connections.push(crate::engine::Connection {
                        source: crate::engine::SourcePort::ChipInput(0),
                        target: crate::engine::TargetPort::ComponentInput { component_idx: i, port_idx: 0 },
                    });
                    connections.push(crate::engine::Connection {
                        source: crate::engine::SourcePort::ChipInput(1),
                        target: crate::engine::TargetPort::ComponentInput { component_idx: i, port_idx: 1 },
                    });
                } else {
                    connections.push(crate::engine::Connection {
                        source: crate::engine::SourcePort::ComponentOutput { component_idx: i - 1, port_idx: 0 },
                        target: crate::engine::TargetPort::ComponentInput { component_idx: i, port_idx: 0 },
                    });
                    connections.push(crate::engine::Connection {
                        source: crate::engine::SourcePort::ComponentOutput { component_idx: i - 1, port_idx: 0 },
                        target: crate::engine::TargetPort::ComponentInput { component_idx: i, port_idx: 1 },
                    });
                }
            }
            
            // Connect last one to ChipOutput
            connections.push(crate::engine::Connection {
                source: crate::engine::SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 },
                target: crate::engine::TargetPort::ChipOutput(0),
            });
            
            let bp = crate::engine::ChipBlueprint {
                name: format!("Level_{}_Package", depth),
                inputs: 2,
                outputs: 1,
                input_names: vec!["In_A".to_string(), "In_B".to_string()],
                output_names: vec!["Out".to_string()],
                components,
                connections,
            };
            self.engine.library.push(bp);
        }
        
        let final_subchip_idx = self.engine.library.len() - 1;
        let final_gates_count = 4_usize.pow(target_depth as u32 + 1);

        let label = if final_gates_count >= 1_000_000 {
            format!("Massive {:.1}M Gate Chip", final_gates_count as f32 / 1_000_000.0)
        } else if final_gates_count >= 1_000 {
            format!("Massive {:.1}k Gate Chip", final_gates_count as f32 / 1_000.0)
        } else {
            format!("Massive {} Gate Chip", final_gates_count)
        };

        // 3. Place ONE massive packaged subchip on the canvas
        self.circuit.components.push(VisualComponent {
            id: self.circuit.next_component_id,
            comp_type: ComponentType::SubChip(final_subchip_idx),
            pos: Vec2::new(300.0, 300.0),
            width: 100.0,
            height: 100.0,
            label,
            clock_period: None,
            bus_width: None,
            color: None,
        });
        let package_id = self.circuit.next_component_id;
        self.circuit.next_component_id += 1;
        
        // Add inputs and outputs to interact with it
        self.circuit.components.push(VisualComponent {
            id: self.circuit.next_component_id,
            comp_type: ComponentType::Clock,
            pos: Vec2::new(100.0, 300.0),
            width: 60.0,
            height: 40.0,
            label: format!("Clock A"),
            clock_period: Some(2),
            bus_width: None,
            color: None,
        });
        let clock1_id = self.circuit.next_component_id;
        self.circuit.next_component_id += 1;
        
        self.circuit.components.push(VisualComponent {
            id: self.circuit.next_component_id,
            comp_type: ComponentType::Clock,
            pos: Vec2::new(100.0, 400.0),
            width: 60.0,
            height: 40.0,
            label: format!("Clock B"),
            clock_period: Some(3),
            bus_width: None,
            color: None,
        });
        let clock2_id = self.circuit.next_component_id;
        self.circuit.next_component_id += 1;

        self.circuit.connections.push(VisualConnection {
            src_comp_id: clock1_id,
            src_port: 0,
            tgt_comp_id: package_id,
            tgt_port: 0,
        });
        self.circuit.connections.push(VisualConnection {
            src_comp_id: clock2_id,
            src_port: 0,
            tgt_comp_id: package_id,
            tgt_port: 1,
        });

        self.compile();
        println!("Recursive Stress test generated: {} nesting levels, compiling to {} primitive NANDs under the hood!", target_depth, final_gates_count);
    }

    /// Generates the stress test and immediately drills down into the base blueprint (Level 0)
    /// so the user can test hot-reloading a single gate and see it instantly patch the generated package!
    pub fn generate_hot_reload_test(&mut self) {
        self.generate_stress_test(6);
        // The base blueprint "Level_0_XOR" is at index 0.
        // Drill down into it:
        self.unpack_blueprint_to_canvas(0);
        self.canvas.pan = macroquad::prelude::Vec2::new(100.0, 100.0);
        println!("Hot-Reload Proving Test: You are now editing the Level_0_XOR blueprint (Index 0). \nMake a change and click 'Save Changes & Return'!");
    }

    /// Generates a zero-delay ring oscillator to intentionally crash/stress the engine's 
    /// oscillation detection limits.
    pub fn generate_oscillation_test(&mut self) {
        self.circuit.components.clear();
        self.circuit.connections.clear();
        self.circuit.next_component_id = 1;

        // Place 3 NAND gates (acting as NOT gates) in a ring
        for i in 0..3 {
            self.circuit.components.push(VisualComponent {
                id: self.circuit.next_component_id,
                comp_type: ComponentType::Nand,
                pos: Vec2::new(200.0 + i as f32 * 150.0, 300.0),
                width: 60.0,
                height: 40.0,
                label: format!("NOT {}", i),
                clock_period: None,
                bus_width: None,
                color: None,
            });
            self.circuit.next_component_id += 1;
        }

        // Wire them in a ring: 0 -> 1, 1 -> 2, 2 -> 0. 
        // For a NAND to act as a NOT, we need to wire the source to both inputs.
        let connections = vec![
            (1, 2), // gate 1 to 2
            (2, 3), // gate 2 to 3
            (3, 1), // gate 3 to 1
        ];

        for (src_id, tgt_id) in connections {
            // Target port 0
            self.circuit.connections.push(VisualConnection {
                src_comp_id: src_id,
                src_port: 0,
                tgt_comp_id: tgt_id,
                tgt_port: 0,
            });
            // Target port 1
            self.circuit.connections.push(VisualConnection {
                src_comp_id: src_id,
                src_port: 0,
                tgt_comp_id: tgt_id,
                tgt_port: 1,
            });
        }

        // Add an input connected to one of the gates to kickstart it
        self.circuit.components.push(VisualComponent {
            id: self.circuit.next_component_id,
            comp_type: ComponentType::Input,
            pos: Vec2::new(50.0, 300.0),
            width: 60.0,
            height: 40.0,
            label: "Kickstart".to_string(),
            clock_period: None,
            bus_width: None,
            color: None,
        });
        
        self.circuit.connections.push(VisualConnection {
            src_comp_id: self.circuit.next_component_id,
            src_port: 0,
            tgt_comp_id: 1,
            tgt_port: 0,
        });
        self.circuit.next_component_id += 1;

        self.compile();
        self.engine.is_playing = true; // Auto-play to trigger oscillation
        println!("Oscillation Test Generated! Toggle the Kickstart input to crash the simulation.");
    }
}
