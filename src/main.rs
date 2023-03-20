

mod config;
mod discover;

#[tokio::main]
async fn main() {
    env_logger::init();
    discover::discover().await;
    println!("dupsko");
}
