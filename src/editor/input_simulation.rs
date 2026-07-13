use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn run_simulation_ticks(&mut self) {
        if self.engine.is_playing {
            for _ in 0..self.engine.ticks_per_frame {
                self.engine.sim_tick_counter = self.engine.sim_tick_counter.wrapping_add(1);

                for clock in &mut self.engine.active_clocks {
                    clock.counter += 1;
                    let half_period = (clock.period / 2).max(1);
                    if clock.counter >= half_period {
                        clock.counter = 0;
                        let current_state = self.engine.simulator.get_state(clock.gate_idx);
                        self.engine
                            .simulator
                            .set_input(clock.gate_idx, !current_state);
                    }
                }

                let max_steps = (self.engine.simulator.nodes.len() * 10).max(1000);
                match self.engine.simulator.propagate_events(max_steps) {
                    Ok(_) => self.engine.propagation_error = None,
                    Err(e) => {
                        self.engine.propagation_error = Some(e);
                        break;
                    }
                }
            }
        }
    }

    pub fn update_resolution_revert_timer(&mut self) {
        if let Some(mut timer) = self.ui.resolution_revert_timer {
            timer -= get_frame_time();
            if timer <= 0.0 {
                // Revert to prev settings
                self.ui.is_fullscreen = self.ui.prev_is_fullscreen;
                self.ui.resolution_idx = self.ui.prev_resolution_idx;
                self.ui.temp_is_fullscreen = self.ui.prev_is_fullscreen;
                self.ui.temp_resolution_idx = self.ui.prev_resolution_idx;

                macroquad::window::set_fullscreen(self.ui.is_fullscreen);
                let resolutions = &[
                    (800, 600),
                    (1024, 768),
                    (1280, 720),
                    (1600, 900),
                    (1920, 1080),
                ];
                let r = resolutions[self.ui.resolution_idx];
                macroquad::window::request_new_screen_size(r.0 as f32, r.1 as f32);

                self.ui.resolution_revert_timer = None;
            } else {
                self.ui.resolution_revert_timer = Some(timer);
            }
        }
    }
}
