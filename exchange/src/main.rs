mod matching;
mod mempool;
mod order;
mod server;
mod block;
mod matched_traces;
mod state;

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();    
    log::info!("Starting ZKVM Order Book Exchange...");
    
    // Start server
    server::start().await;
}
