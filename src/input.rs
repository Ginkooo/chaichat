use std::time::Duration;

use crossterm::event::{poll, read, Event};
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use std::thread;

pub fn start_input_event_thread() -> UnboundedReceiver<Event> {
    let (sender, receiver) = unbounded();
    thread::spawn(move || loop {
        if !poll(Duration::from_millis(5)).unwrap_or(false) {
            continue;
        }
        let event = read();
        if event.is_err() {
            continue;
        }
        sender.unbounded_send(event.unwrap()).unwrap();
    });
    return receiver;
}
