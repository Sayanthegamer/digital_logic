use super::Editor;
use super::state::CanvasSnapshot;

impl Editor {
    /// Captures the current visual state and pushes it to the undo stack.
    /// This should be called *before* any destructive action.
    pub fn push_history_snapshot(&mut self) {
        let snapshot = CanvasSnapshot {
            components: self.circuit.components.clone(),
            connections: self.circuit.connections.clone(),
            annotations: self.circuit.annotations.clone(),
            next_component_id: self.circuit.next_component_id,
            pan: self.canvas.pan,
            zoom: self.canvas.zoom,
        };

        self.history.undo_stack.push_back(snapshot);

        // Truncate if we exceed max_steps
        if self.history.undo_stack.len() > self.history.max_steps {
            self.history.undo_stack.pop_front();
        }

        // Whenever a new action is performed, the redo stack is invalidated
        self.history.redo_stack.clear();
    }

    /// Reverts the canvas to the previous state.
    pub fn undo(&mut self) {
        if let Some(prev_state) = self.history.undo_stack.pop_back() {
            // Save current state to redo stack
            let current_snapshot = CanvasSnapshot {
                components: self.circuit.components.clone(),
                connections: self.circuit.connections.clone(),
                annotations: self.circuit.annotations.clone(),
                next_component_id: self.circuit.next_component_id,
                pan: self.canvas.pan,
                zoom: self.canvas.zoom,
            };
            self.history.redo_stack.push_back(current_snapshot);

            // Apply previous state
            self.circuit.components = prev_state.components;
            self.circuit.connections = prev_state.connections;
            self.circuit.annotations = prev_state.annotations;
            self.circuit.next_component_id = prev_state.next_component_id;
            self.canvas.pan = prev_state.pan;
            self.canvas.zoom = prev_state.zoom;

            // Recompile the simulation engine
            self.compile();

            // Clear any lingering selection or interactions
            self.canvas.selected_comp_id = None;
            self.canvas.selected_comp_ids.clear();
            self.canvas.selected_connections.clear();
            self.canvas.active_wire_drag = None;
            self.canvas.dragging_comp_id = None;
            self.canvas.dragging_annotation_idx = None;
            self.canvas.selected_annotation_idx = None;
        }
    }

    /// Re-applies a previously undone state.
    pub fn redo(&mut self) {
        if let Some(next_state) = self.history.redo_stack.pop_back() {
            // Save current state to undo stack
            let current_snapshot = CanvasSnapshot {
                components: self.circuit.components.clone(),
                connections: self.circuit.connections.clone(),
                annotations: self.circuit.annotations.clone(),
                next_component_id: self.circuit.next_component_id,
                pan: self.canvas.pan,
                zoom: self.canvas.zoom,
            };
            self.history.undo_stack.push_back(current_snapshot);

            // Apply next state
            self.circuit.components = next_state.components;
            self.circuit.connections = next_state.connections;
            self.circuit.annotations = next_state.annotations;
            self.circuit.next_component_id = next_state.next_component_id;
            self.canvas.pan = next_state.pan;
            self.canvas.zoom = next_state.zoom;

            // Recompile the simulation engine
            self.compile();

            // Clear any lingering selection or interactions
            self.canvas.selected_comp_id = None;
            self.canvas.selected_comp_ids.clear();
            self.canvas.selected_connections.clear();
            self.canvas.active_wire_drag = None;
            self.canvas.dragging_comp_id = None;
            self.canvas.dragging_annotation_idx = None;
            self.canvas.selected_annotation_idx = None;
        }
    }
}
