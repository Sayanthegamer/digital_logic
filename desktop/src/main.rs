use macroquad::prelude::*;

#[macroquad::main("Digital Logic Simulator")]
async fn main() {
    logic_simulator::run().await;
}
