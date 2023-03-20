use tokio::join;

mod config;
mod identify;
mod register;

#[tokio::main]
async fn main() {
    env_logger::init();
    join!(register::register(), identify::identify());
    println!("dupsko");
}
