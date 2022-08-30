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
use env_logger;
use futures::channel::mpsc::unbounded;
use input::start_input_event_thread;
use std::thread;
use terminal::ChaiTerminal;
use types::{ChannelsP2pEnd, ChannelsTerminalEnd, Message, Res};

fn main() -> Res<()> {
    env_logger::init();

    let (out_p2p_sender, out_p2p_receiver) = unbounded::<Message>();
    let (in_p2p_sender, mut in_p2p_receiver) = unbounded::<Message>();

    let mut term = ChaiTerminal::init()?;

    let mut receiver_camera = run_camera_thread().expect("Could not start camera");
    let mut input_event_receiver = start_input_event_thread();

    let mut channels_terminal_end = ChannelsTerminalEnd {
        receiver_camera: &mut receiver_camera,
        input_event_receiver: &mut input_event_receiver,
        in_p2p_receiver: &mut in_p2p_receiver,
        out_p2p_sender,
    };

    let channeld_p2p_end = ChannelsP2pEnd {
        in_p2p_sender,
        out_p2p_receiver,
    };

    thread::scope(|scope| {
        scope.spawn(|| loop {
            match term.draw_in_terminal(&mut channels_terminal_end) {
                Err(_) => {
                    term.uninit();
                    for _ in 0..100 {
                        println!("");
                    }
                    std::process::exit(0);
                }
                _ => {}
            }
        });
        scope.spawn(|| {
            let mut p2p = P2p::new(channeld_p2p_end);

            p2p.start().unwrap();
        });
    });

    Ok(())
}
