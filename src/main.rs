mod camera;
mod camera_frame;
mod commands;
mod consts;
mod input;
mod p2p;
mod terminal;
mod types;
mod utils;

use crate::p2p::P2p;
use camera::run_camera_thread;
use crossbeam::scope;
use env_logger;
use input::start_input_event_thread;
use terminal::ChaiTerminal;
use types::Res;

fn main() -> Res<()> {
    env_logger::init();

    let p2p = P2p::new();

    p2p.start().unwrap();

    let mut term = ChaiTerminal::init()?;

    let receiver_camera = run_camera_thread().expect("Could not start camera");
    let input_event_receiver = start_input_event_thread();
    scope(|spawner| {
        spawner.spawn(|_| loop {
            term.draw_in_terminal(receiver_camera.clone(), input_event_receiver.clone())
                .unwrap();
        });
    })
    .unwrap();

    term.uninit();

    Ok(())
}
