use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

// Use the crate's modules directly
use common::order::Order;
use common::traces::MatchedTrace;
use exchange::MATCHED_TRACES;
use exchange::block::block_builder::BlockBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Create a block builder with a local database
    let block_builder = BlockBuilder::new("./blocks_db")?;

    // Start block generation in a background task
    let builder_clone = block_builder.clone();
    let block_generation_task = tokio::spawn(async move {
        if let Err(e) = builder_clone.start_block_generation().await {
            eprintln!("Block generation error: {}", e);
        }
    });

    // Simulate adding some matched traces
    for i in 0..150 {
        let buy_order = Order::new(
            format!("buy_{}", i),
            format!("user_buy_{}", i % 10),
            "BTC/USDT".to_string(),
            100,
            1000 + i,
            true, // true for buy
        );

        let sell_order = Order::new(
            format!("sell_{}", i),
            format!("user_sell_{}", i % 10),
            "BTC/USDT".to_string(),
            100,
            1000 + i,
            false, // false for sell
        );

        let matched_trace = MatchedTrace {
            buy_order,
            sell_order,
            matched_amount: 100,
        };

        // Add to global MATCHED_TRACES
        {
            let mut traces = MATCHED_TRACES.write().await;
            traces.push(matched_trace);
        }

        // Add some delay to simulate real trading
        sleep(Duration::from_millis(50)).await;

        // Print progress
        if i % 20 == 0 {
            println!("Added {} matched traces", i + 1);
            let latest_block_num = block_builder.get_latest_block_num().await;
            println!("Latest block number: {}", latest_block_num);
        }
    }

    // Wait a bit more to let the final block be generated
    sleep(Duration::from_secs(15)).await;

    // Query some blocks
    let latest_block_num = block_builder.get_latest_block_num().await;
    println!("Final latest block number: {}", latest_block_num);

    if latest_block_num > 0 {
        // Get the latest block
        if let Some(latest_block) = block_builder.get_block(latest_block_num).await? {
            println!(
                "Latest block #{} contains {} transactions",
                latest_block.block_num,
                latest_block.txns.len()
            );
        }

        // Get all blocks
        let all_blocks = block_builder.get_blocks_range(1, latest_block_num).await?;
        println!("Total blocks generated: {}", all_blocks.len());

        for block in &all_blocks {
            println!(
                "Block #{}: {} transactions, state_root: {:?}",
                block.block_num,
                block.txns.len(),
                block.state_root.map(|root| format!(
                    "{:02x}{:02x}...{:02x}{:02x}",
                    root[0], root[1], root[30], root[31]
                ))
            );
        }
    }

    // Cancel the background task
    block_generation_task.abort();

    Ok(())
}
