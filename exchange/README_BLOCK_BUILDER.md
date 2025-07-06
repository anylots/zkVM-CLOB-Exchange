# Block Builder 功能说明

## 概述

`BlockBuilder` 是一个异步区块生成器，用于监控 `MATCHED_TRACES` 并根据时间间隔或交易数量阈值生成区块。生成的区块使用 sled 数据库保存到本地。

## 主要功能

### 1. 异步区块生成
- **方法**: `start_block_generation()`
- **功能**: 持续监控全局 `MATCHED_TRACES`，当满足以下条件之一时生成区块：
  - 交易数量达到 `MAX_TXN_SIZE` (默认 100)
  - 时间间隔达到 `BLOCK_TIME_INTERVAL` (默认 10 秒)

### 2. 区块存储
- 使用 sled 数据库进行本地持久化存储
- 区块以 `block_{block_num}` 为键存储
- 自动维护最新区块号

### 3. 状态根计算
- 为每个区块计算简化的状态根哈希
- 基于区块内所有交易的哈希值

## 配置参数

```rust
static MAX_TXN_SIZE: u64 = 100;                              // 最大交易数量
static BLOCK_TIME_INTERVAL: Duration = Duration::from_secs(10); // 时间间隔（秒）
```

## 使用方法

### 1. 创建 BlockBuilder 实例

```rust
use exchange::block::block_builder::BlockBuilder;

let block_builder = BlockBuilder::new("./blocks_db")?;
```

### 2. 启动区块生成

```rust
// 在后台任务中启动区块生成
let builder_clone = block_builder.clone();
let block_generation_task = tokio::spawn(async move {
    if let Err(e) = builder_clone.start_block_generation().await {
        eprintln!("Block generation error: {}", e);
    }
});
```

### 3. 添加匹配交易

```rust
use exchange::matched_traces::{MatchedTrace, MATCHED_TRACES};

// 创建匹配交易
let matched_trace = MatchedTrace {
    buy_order: buy_order,
    sell_order: sell_order,
    matched_amount: 100,
};

// 添加到全局 MATCHED_TRACES
{
    let mut traces = MATCHED_TRACES.write().await;
    traces.push(matched_trace);
}
```

### 4. 查询区块

```rust
// 获取最新区块号
let latest_block_num = block_builder.get_latest_block_num().await;

// 获取特定区块
if let Some(block) = block_builder.get_block(block_num).await? {
    println!("Block #{}: {} transactions", block.block_num, block.txns.len());
}

// 获取区块范围
let blocks = block_builder.get_blocks_range(1, 10).await?;
```

## 数据结构

### Block
```rust
pub struct Block {
    pub block_num: u128,           // 区块号
    pub txns: Vec<MatchedTrace>,   // 交易列表
    pub state_root: Option<[u8; 32]>, // 状态根
}
```

### MatchedTrace
```rust
pub struct MatchedTrace {
    pub buy_order: Order,      // 买单
    pub sell_order: Order,     // 卖单
    pub matched_amount: u64,   // 匹配数量
}
```

## 运行示例

```bash
# 编译并运行示例
cargo run --example block_builder_example

# 仅检查编译
cargo check --example block_builder_example
```

## 示例输出

```
Added 1 matched traces
Latest block number: 0
Added 21 matched traces
Latest block number: 0
Added 41 matched traces
Latest block number: 0
Added 61 matched traces
Latest block number: 0
Added 81 matched traces
Latest block number: 0
Added 101 matched traces
Latest block number: 1
Generated block #1 with 100 transactions
Added 121 matched traces
Latest block number: 1
Added 141 matched traces
Latest block number: 1
Generated block #2 with 50 transactions
Final latest block number: 2
Latest block #2 contains 50 transactions
Total blocks generated: 2
Block #1: 100 transactions, state_root: Some("a1b2...c3d4")
Block #2: 50 transactions, state_root: Some("e5f6...g7h8")
```

## 注意事项

1. **并发安全**: 使用 `Arc<RwLock<>>` 确保多线程安全
2. **数据持久化**: 区块数据自动持久化到 sled 数据库
3. **错误处理**: 所有操作都返回 `Result` 类型，需要适当的错误处理
4. **资源管理**: 记得在程序结束时取消后台任务

## 扩展功能

可以根据需要扩展以下功能：
- 自定义区块生成条件
- 更复杂的状态根计算
- 区块验证机制
- 网络同步功能
