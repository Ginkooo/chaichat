use std::time::Duration;

use crossbeam::channel::{unbounded, Receiver};
use crossterm::event::{poll, read, Event};
use std::thread;

pub fn start_input_event_thread() -> Receiver<Event> {
    let (sender, receiver) = unbounded();
    thread::spawn(move || loop {
        if !poll(Duration::from_millis(5)).unwrap_or(false) {
            continue;
        }
        let event = read();
        if event.is_err() {
            continue;
        }
        sender.send(event.unwrap()).unwrap();
    });
    return receiver;
}
