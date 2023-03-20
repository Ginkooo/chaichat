use dotenv::dotenv;
use lazy_static::lazy_static;
use libp2p::{core::identity, identity::Keypair};

lazy_static! {
    pub static ref RZV_SERVER_IP: String = {
        dotenv().ok();
        std::env::var("RZV_SERVER_IP").unwrap()
    };
    pub static ref RZV_SERVER_PORT: String = {
        dotenv().ok();
        std::env::var("RZV_SERVER_PORT").unwrap()
    };
    pub static ref RZV_PEER_ID: String = {
        dotenv().ok();
        std::env::var("RZV_PEER_ID").unwrap()
    };
    pub static ref KEY_PAIR: Keypair = identity::Keypair::generate_ed25519();
}
