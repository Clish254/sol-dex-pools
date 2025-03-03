# Solana AMM Aggregator

A Rust application that aggregates pool data from multiple Solana AMMs to find the healthiest liquidity pool for a given token pair.

## Features

- **Multi-AMM Support**: Fetches pool data from Raydium, Orca, Meteora Dynamic AMM, and Meteora DLMM
- **Parallel Processing**: Uses Tokio to fetch data from all AMMs simultaneously
- **Health Scoring**: Ranks pools based on liquidity, volume, and fees
- **Error Handling**: Gracefully handles timeouts and API failures

## Usage

```
cargo run
```

## Health Score Calculation

Pools are ranked based on a composite score (0.0-1.0) that considers:
- Liquidity (45%) - Higher is better
- 24h Volume (45%) - Higher is better
- Fee Rate (10%) - Lower is better

## AMM API Endpoints

- Raydium: `https://api-v3.raydium.io/pools/info/mint`
- Orca: `https://api.orca.so/v2/solana/pools`
- Meteora Dynamic Amm: `https://amm-v2.meteora.ag/pools/search`
- Meteora DLMM: `https://dlmm-api.meteora.ag/pair/all_by_groups`

## Project Structure

- `main.rs` - Core application logic and pool analysis
- `raydium.rs` - Raydium API integration
- `orca.rs` - Orca API integration
- `meteora.rs` - Meteora Dynamic AMM pool API integration
- `meteora_dlmm.rs` - Meteora DLMM pool API integration
