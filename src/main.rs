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
use async_std::channel;
use camera::run_camera_thread;
use env_logger;
use input::start_input_event_thread;
use std::thread;
use terminal::ChaiTerminal;
use types::{Message, Res};

fn main() -> Res<()> {
    env_logger::init();

    let (out_p2p_sender, out_p2p_receiver) = channel::unbounded::<Message>();
    let (in_p2p_sender, in_p2p_receiver) = channel::unbounded::<Message>();

    let mut term = ChaiTerminal::init()?;

    let receiver_camera = run_camera_thread().expect("Could not start camera");
    let input_event_receiver = start_input_event_thread();
    thread::spawn(move || loop {
        term.draw_in_terminal(
            receiver_camera.clone(),
            input_event_receiver.clone(),
            in_p2p_receiver.clone(),
            out_p2p_sender.clone(),
        )
        .unwrap();
    });
    thread::spawn(|| {
        let p2p = P2p::new(in_p2p_sender, out_p2p_receiver);

        p2p.start().unwrap();
    });

    Ok(())
}
