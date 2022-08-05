use std::time::Duration;

use async_std::channel::{self, Receiver};
use crossterm::event::{poll, read, Event};
use std::thread;

pub fn start_input_event_thread() -> Receiver<Event> {
    let (sender, receiver) = channel::unbounded();
    thread::spawn(move || loop {
        if !poll(Duration::from_millis(5)).unwrap_or(false) {
            continue;
        }
        let event = read();
        if event.is_err() {
            continue;
        }
        sender.send(event.unwrap());
    });
    return receiver;
}
