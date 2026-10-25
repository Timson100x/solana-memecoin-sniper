# solana-trading-bot

> ⚠️ **Warning:** Alle Trades können Totalverlust bedeuten. Teste zuerst im Paper-Trading.

Ein Rust-basierter Solana Momentum-Trading-Bot (v0.3.0). Standardmäßig **Paper / Dry-Run** — es werden **keine echten Transaktionen** gesendet. Live-Trading ist explizit und nur mit Doppelbestätigung aktivierbar.

## Architektur

- **Datenquelle:** Helius RPC + WebSocket (Pump.fun / PumpSwap Graduationen)
- **Routing:** Jupiter (Quote-API), ggf. Metis
- **Strategie:** Pluggable `Strategy`-Trait; neutrale Beispielstrategie
- **Risiko:** harte Limits (Tagesverlust, Max-Trade, offene Positionen, Slippage)
- **Ausführung:** `PaperExecutor` (Standard) / `LiveExecutor` (nur bei voller Bestätigung)
- **Sicherheit:** Private Keys nie im Repo/Logs, `ENABLE_LIVE_TRADING=false` Standard

## Codespace starten

1. GitHub → Code → Codespaces → *New codespace* → Branch `kilo/celestial-lantern-crt`.
2. Devcontainer-Setup startet automatisch (`scripts/bootstrap.sh`).

## Codespace Secrets anlegen (GitHub)

In GitHub Repo → Settings → Codespaces → Secrets:

- `HELIUS_API_KEY`
- `HELIUS_RPC_URL`
- `HELIUS_WS_URL`
- `JUPITER_API_KEY` (optional)
- `SOLANA_WALLET_PRIVATE_KEY` (nur für Live-Trading; niemals committen!)
- `TELEGRAM_BOT_TOKEN` (optional)
- `TELEGRAM_CHAT_ID` (optional)

> Ohne Secrets baut und testet der Bot weiterhin (Paper-Modus).

## Lokale .env (nur Entwicklung)

```bash
cp .env.example .env
# Werte anpassen (niemals ins Repo pushen)
```

## Setup prüfen

```bash
bash scripts/verify.sh
```

Führt aus: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo run -- --config-check`.

## Paper-Trading starten

```bash
cargo run -- --paper
```

Standardmäßig `ENABLE_LIVE_TRADING=false` → *keine* Transaktionen werden signiert oder gesendet.

## Live-Trading starten (nur erfahren!)

```bash
# Setze in .env:
# ENABLE_LIVE_TRADING=true
# SOLANA_WALLET_PRIVATE_KEY=<base58-key>

cargo run -- --live --execution-confirmation "I_UNDERSTAND_THE_RISK"
```

Ohne beide Sicherheitsbedingungen schlägt `--live` fehl.

## Variablen

| Variable | Erforderlich | Bedeutung |
|---|---|---|
| `HELIUS_API_KEY` | nein | Helius API |
| `HELIUS_RPC_URL` | nein | Mainnet RPC |
| `HELIUS_WS_URL` | nein | WebSocket |
| `JUPITER_API_URL` | nein | Jupiter Quote-Endpoint |
| `SOLANA_WALLET_PRIVATE_KEY` | nur Live | Wallet (base58) |
| `ENABLE_LIVE_TRADING` | nein | Standard `false` |
| `EXECUTION_CONFIRMATION` | nur Live | bestätigen |

## Verantwortungshaftung

Cryptowährungen sind risikoreich. Der Bot ist Bildungszwecken gewidmet — stets Devnet/Test zuerst.
