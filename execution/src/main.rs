use execution::{block::block_builder::BlockBuilder, server};

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ZKVM Order Book Exchange...");

    // Start BlockBuilder
    let block_builder = BlockBuilder::new("block_db").unwrap();
    tokio::spawn(async move { block_builder.start_block_generation().await });

    // Start server
    server::start().await;
}
