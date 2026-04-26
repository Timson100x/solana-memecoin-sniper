# solana-memecoin-sniper

Ultra-fast Solana Memecoin Sniper Bot for Pump.fun Trading - Rust, Anchor, Devnet Ready

## Features

- Ultra-fast execution via Yellowstone gRPC and Triton One RPC
- Pump.fun token sniping - detects new token launches in real-time
- Rust-powered - zero-cost abstractions, maximum performance
- Anchor integration - type-safe Solana program interactions
- Token-2022 support - handles extended mint accounts and ATA detection
- Auto TP/SL - configurable Take-Profit and Stop-Loss
- Devnet ready - test safely before going live
- GitHub Codespaces - one-click cloud dev environment

## Prerequisites

- Rust >= 1.75
- Solana CLI >= 1.18
- Anchor >= 0.30
- A Solana wallet with SOL

## Installation

Clone the repository:
git clone https://github.com/Timson100x/solana-memecoin-sniper.git
cd solana-memecoin-sniper

Build:
cargo build --release

Copy config:
cp .env.example .env

## Configuration

Edit .env with your settings:
- RPC_URL: your RPC endpoint
- WSS_URL: your WebSocket endpoint  
- PRIVATE_KEY: your base58 private key
- BUY_AMOUNT_SOL: e.g. 0.1
- TAKE_PROFIT_PCT: e.g. 50
- STOP_LOSS_PCT: e.g. 20
- SLIPPAGE_BPS: e.g. 300

## Run on Devnet

cargo run --release -- --network devnet

## Run on Mainnet

cargo run --release -- --network mainnet

## GitHub Codespaces

1. Click Code -> Codespaces -> Create codespace on main
2. Wait for the environment (~2 min)
3. Run cargo build --release
4. Configure .env and start

## Project Structure

solana-memecoin-sniper/
├── .devcontainer/        Codespaces config
├── .github/workflows/    CI/CD pipelines
├── src/
│   ├── main.rs           Entry point
│   ├── sniper.rs         Core sniper logic
│   └── ts/
│       └── token-utils.ts  Token-2022 ATA helpers
├── Cargo.toml
└── .env.example

## Disclaimer

This bot is for educational purposes only. Trading cryptocurrencies carries significant financial risk. Always test on Devnet first.

## Built By

Foxy Vega / Timson100x - https://github.com/Timson100x
Part of the $FOXY ecosystem - AI-powered cyberpunk memecoin on Solana.
