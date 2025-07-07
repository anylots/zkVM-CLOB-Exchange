# ZKVM CLOB Exchange

A high-performance Central Limit Order Book (CLOB) exchange powered by Zero-Knowledge Virtual Machine technology, demonstrating how ZK is becoming the backbone of performance-grade DeFi.

## Motivation

The latest advancements in the performance of zk proof systems (as well as potential future improvements) are making the Rust → RISC-V → ZKVM technology stack a new paradigm for high-performance applications. This enables applications to surpass the speed and security of high-performance blockchains（eg: hyperliquid、solana） while maintaining excellent development efficiency.

# ZKVM CLOB Exchange Architecture

```
        ╔══════════════════════════════════════════════════════════╗
        ║                 ZKVM CLOB EXCHANGE                       ║
        ║              High-Performance Trading                    ║
        ╚══════════════════════════════════════════════════════════╝

┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│                 │         │                 │         │                 │
│   USER ORDERS   │───────▶│  EXCHANGE CLOB  │────────▶│   ZK PROVER     │
│                 │         │                 │         │                 │
│  • Buy Orders   │         │  • Order Book   │         │  • SP1 ZKVM     │
│  • Sell Orders  │         │  • Matching     │         │  • Verification │
│  • REST API     │         │  • State DB     │         │  • Proof Gen    │
│                 │         │                 │         │                 │
└─────────────────┘         └─────────────────┘         └─────────────────┘
         │                           │                           │
         │                           │                           │
         ▼                           ▼                           ▼
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│                 │         │                 │         │                 │
│   VALIDATION    │         │  MATCHED TRACES │         │  CRYPTOGRAPHIC  │
│                 │         │                 │         │                 │
│  • Balance      │         │  • Trade Data   │         │  • ZK Proof     │
│  • Mempool      │         │  • Blocks       │         │  • Verification │
│  • Queue        │         │  • Persistence  │         │  • L1 Ready     │
│                 │         │                 │         │                 │
└─────────────────┘         └─────────────────┘         └─────────────────┘

═══════════════════════════════════════════════════════════════════════════════

                                DATA FLOW 

    Buy or Sell Orders ──▶ Matching ──▶ Traces ──▶ Blocks ──▶ ZK Proof ──▶ Settlement
    Cancel Orders ──▶ Do not Matching ──▶ DA

═══════════════════════════════════════════════════════════════════════════════
```

## Overview

This project implements a next-generation order book exchange that leverages ZK proofs to enable trustless, verifiable trading on Layer 2. As CLOB-specific L2s are quietly rewriting what "onchain trading" means, this implementation showcases the future of decentralized exchange infrastructure.

## Architecture

### Exchange Engine
- **High-performance order matching** with price-time priority
- **Real-time order book** management and state tracking
- **Multi-token support** with ERC-20 style deposits/withdrawals
- **RESTful API** for seamless integration

### ZK Prover System
- **SP1 ZKVM integration** for generating cryptographic proofs
- **Verifiable execution** of trading operations
- **Trustless settlement** with L1 finality guarantees
- **Scalable proof generation** for batch processing

## Key Features

- ✅ **Instant Settlement**: Sub-second order matching with ZK proof generation
- ✅ **Trustless Trading**: All operations are cryptographically verifiable
- ✅ **L2 Performance**: High throughput without compromising security
- ✅ **CLOB Mechanics**: Professional-grade order book with partial fills
- ✅ **Multi-Asset Support**: Trade any token pair with unified liquidity

## Quick Start

```bash
# Start the exchange
cd exchange && cargo run

# The server runs on http://[::1]:3030
# See exchange/API_DOCUMENTATION.md for full API reference
```

## Technology Stack

- **Rust**: High-performance systems programming
- **SP1 ZKVM**: Zero-knowledge virtual machine for proof generation
- **Axum**: Modern async web framework
- **Sled**: Embedded database for state persistence

## Use Cases

This implementation demonstrates the potential for:
- **DEX Aggregators**: Unified liquidity across multiple venues
- **Institutional Trading**: Professional-grade order management
- **Cross-Chain Trading**: Trustless asset exchange between chains
- **DeFi Protocols**: Composable trading primitives

## The Future of Onchain Trading

As the DeFi ecosystem evolves, ZK-powered CLOBs represent a paradigm shift from AMM-based trading to order book efficiency with blockchain security. This project showcases how Zero-Knowledge technology enables the next generation of decentralized exchanges.

---

*Built with ❤️ for the future of decentralized finance*
