# AGENT.md - Solana Momentum Bot Architecture & Agent Guidelines

## 1. System-Overview & Execution Environment
This repository contains an automated Solana Momentum Trading Bot (`solana-momentum-bot-v3.4`) written in Rust.

- **AI Agent Framework:** Kilo Code CLI / Kilo Desktop Beta (OpenCode-compatible)
- **Primary LLM Model:** `poolside/laguna-s-2.1` via OpenRouter (`https://openrouter.ai/api/v1`)
- **Language & Runtime:** Rust (Tokio Async Runtime, Edition 2021)
- **Wallet & Credentials:** Local file-based keypair at `/home/solana/phantom_keypair.json`
- **Execution Strategy:** Decoupled Architecture (`Research` -> `Risk Filter` -> `Raptor JIT Execution`)

---

## 2. Kilo Agent & Tools Setup

### 2.1 Provider Configuration (`~/.config/kilo/config.json` or `.kilocode/config.json`)
The agent communicates with OpenRouter using the Poolside Laguna S 2.1 model:

```json
{
  "providers": {
    "openrouter": {
      "type": "openai",
      "baseUrl": "[https://openrouter.ai/api/v1](https://openrouter.ai/api/v1)",
      "apiKey": "${OPENROUTER_API_KEY}",
      "models": [
        "poolside/laguna-s-2.1",
        "poolside/laguna-s-2.1:free"
      ]
    }
  },
  "defaultModel": "openrouter/poolside/laguna-s-2.1"
}
```

### 2.2 CLI Quick-Commands
 * Start Agent Session: kilo run "Refactor risk filters in src/risk.rs"
 * Check Provider Status: /about or /status inside TUI
 * Auto-Approve Code Changes: /auto-approve
 * System Sleep Prevention: /caffeinate

## 3. Solana Tools & Environment Setup
Local Prerequisites & Verification
When prompting or letting agents test code, verify tool installation via terminal:
 * Rust Toolchain: rustc --version / cargo --version
 * Solana CLI: solana --version
 * Keypair Validation: Verify existence of /home/solana/phantom_keypair.json before any live transaction.

Infrastructure Services
 * RPC Endpoint: Helius Developer Plan RPC (https://mainnet.helius-rpc.com/?api-key=...)
 * Execution Router: Raptor Hosted / JIT Execution API (https://api.raptor.solana.tracker/v1/quote)
 * Streaming (Future Upgrade): Yellowstone gRPC (e.g., Solana Tracker standalone stream)

## 4. Architectural Rules & Best Practices for Agents
When generating, modifying, or refactoring Rust code in this repository, all Agents MUST follow these strict rules:
 * Non-Blocking Execution:
   * Always use tokio::sync::mpsc channels to decouple event streams from trade execution.
   * Never run blocking synchronous I/O or heavy computations inside Tokio worker loops.
 * Safety Gates & Fail-Fast Risk Filters:
   * All candidate tokens must pass through RiskManager::validate_candidate() in src/risk.rs BEFORE sending quotes or trades.
   * Top 10 Holder Threshold: Max 45.0% (max_top_holder_pct).
   * Daily Loss Circuit Breaker: Cease trading if daily losses hit limit (max_daily_loss_sol).
   * Price Impact Guard: Max 3.0% price impact on quotes.
 * DRY_RUN Enforcement:
   * config.dry_run defaults to true.
   * Never execute live transaction signing or sending unless dry_run == false.
 * Error Handling & Resilience:
   * Use Result and Option explicitly. Avoid unwrap() or expect() inside production channels and worker loops.

## 5. Development & Testing Commands
Agents can run and verify builds using the following commands:
 * Build Project: cargo build
 * Build Release: cargo build --release
 * Run Unit Tests: cargo test
 * Run Bot (Paper Trade Mode): cargo run
" filePath="/workspace/app/AGENT.md