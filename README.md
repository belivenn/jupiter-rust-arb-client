# Solana Arbitrage Bot Exercise

A simple arbitrage bot built in Rust using Jupiter's swap API to find and execute profitable trading opportunities across multiple DEXs.

## Features

- **Multi-DEX Support**: Fluxbeam, Dexlab, Whirlpool, Meteora DLMM, Raydium CLMM, Orca
- **Real-time Arbitrage Detection**: Monitors USDC ↔ SOL price differences
- **Atomic-like Execution**: Executes both legs of arbitrage with minimal delay
- **Profit Threshold**: Configurable minimum profit percentage (currently 0.01%)

## How it Works

1. Gets USDC → SOL quote from Jupiter
2. Gets SOL → USDC quote using the output amount
3. Calculates potential profit
4. If profitable, executes both swaps in sequence

## Setup

```bash
# Install Rust dependencies
cargo build

# Add your wallet file (rdy5.json) to the root directory
# Run the bot
cargo run
```
