use libp2p::dcutr;
use libp2p::floodsub::FloodsubEvent;
use libp2p::identify::IdentifyEvent;
use libp2p::ping::PingEvent;
use libp2p::relay::v2::client;

#[derive(Debug)]
pub enum Event {
    Floodsub(FloodsubEvent),
    Ping(PingEvent),
    Identify(IdentifyEvent),
    Relay(client::Event),
    Dcutr(dcutr::behaviour::Event),
}

impl From<PingEvent> for Event {
    fn from(e: PingEvent) -> Self {
        Event::Ping(e)
    }
}

impl From<FloodsubEvent> for Event {
    fn from(e: FloodsubEvent) -> Self {
        Event::Floodsub(e)
    }
}

impl From<IdentifyEvent> for Event {
    fn from(e: IdentifyEvent) -> Self {
        Event::Identify(e)
    }
}

impl From<client::Event> for Event {
    fn from(e: client::Event) -> Self {
        Event::Relay(e)
    }
}

impl From<dcutr::behaviour::Event> for Event {
    fn from(e: dcutr::behaviour::Event) -> Self {
        Event::Dcutr(e)
    }
}
