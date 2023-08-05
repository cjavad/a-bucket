mod authentication;
mod metadata;
mod server;
mod storable;
mod storage;

use server::Server;

const TMP_PATH: &str = "/tmp";

#[tokio::main]
async fn main() {
    let server = Server::new("0.0.0.0:8000").await;
    server.run().await;
}
