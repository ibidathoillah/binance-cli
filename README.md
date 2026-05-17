# binance-cli

Unofficial Rust CLI for Binance Spot. Use it to inspect markets, manage account data, place spot orders, stream live WebSocket events, run a local interactive shell, and expose the same command surface to agents through MCP.

[![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust)](https://www.rust-lang.org/)
[![CLI](https://img.shields.io/badge/interface-terminal-2f855a)](#quick-start)
[![WebSocket](https://img.shields.io/badge/websocket-live-2563eb)](#websocket-streaming)
[![MCP](https://img.shields.io/badge/MCP-ready-7c3aed)](#mcp-server)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## Highlights

- Public market data: ping, server time, exchange info, tickers, order book, trades, aggregate trades, and OHLC/klines.
- Private account data: balances, account info, open orders, all orders, and trade history.
- Spot trading: market and limit buy/sell, query order, cancel order, cancel all orders.
- Funding: deposit address, deposit history, withdrawal history, and crypto withdrawals.
- Real-time streams: depth, ticker, book ticker, user data, order events, and balance events.
- Interactive shell: stateful REPL with persistent history at `~/.config/binance/history`.
- Paper trading: local simulated balances and orders stored in `~/.config/binance/paper_state.json`.
- Automation-friendly output: human tables by default, JSON envelopes with `-o json`.
- Credential resolution: CLI flags, environment variables, or `~/.config/binance/config.toml`.
- Agent support: MCP server mode for tool discovery and JSON-RPC execution.

## Installation

Install from source:

```bash
git clone https://github.com/ibidathoillah/binance-cli.git
cd binance-cli
cargo install --path .
```

Install from crates.io:

```bash
cargo install binance-cli
```

Install from npm:

```bash
npm install -g @ibidathoillah/binance-cli
```

Run with Docker:

```bash
docker run --rm ibidathoillah/binance-cli server-time
docker run -it --rm -v ~/.config/binance:/root/.config/binance ibidathoillah/binance-cli shell
```

Run from the checkout:

```bash
cargo build
./target/debug/binance --help
```

## Quick Start

Market data does not require credentials:

```bash
binance ping
binance server-time
binance price btc/usdt
binance ticker btc/usdt
binance orderbook btc/usdt --count 10
binance -o json book-ticker btc/usdt
```

Configure private API credentials:

```bash
binance auth set --api-key YOUR_API_KEY --api-secret YOUR_API_SECRET
binance auth test
binance auth show
```

Or use environment variables:

```bash
export BINANCE_API_KEY=your_api_key
export BINANCE_API_SECRET=your_api_secret
```

Credential priority:

1. `--api-key` and `--api-secret`
2. `BINANCE_API_KEY` and `BINANCE_API_SECRET`
3. `~/.config/binance/config.toml`

## Command Reference

Global options:

```text
binance [OPTIONS] <COMMAND>

Options:
  -o, --output <table|json>      Output format [default: table]
      --api-key <API_KEY>        API key override
      --api-secret <API_SECRET>  API secret override
  -v, --verbose                  Enable verbose logs
      --host <HOST>              Override API host
```

### Market

```bash
binance ping
binance server-time
binance exchange-info
binance ticker btc/usdt
binance ticker-all
binance price btc/usdt
binance book-ticker btc/usdt
binance orderbook btc/usdt --count 10
binance trades btc/usdt --count 5
binance agg-trades btc/usdt --count 5
binance historical-trades btc/usdt --count 5
binance ohlc btc/usdt --interval 1m --count 5
```

### Account

```bash
binance account-info
binance balance
binance trades-history btc/usdt --count 5
```

### Trading

```bash
binance order buy btc/usdt -t LIMIT --price 76500 --volume 0.005
binance order sell btc/usdt -t MARKET --volume 0.002
binance order cancel btc/usdt --order-id 1872651
binance order cancel-all btc/usdt
binance order query btc/usdt --order-id 1872651
binance order open-orders --pair btc/usdt
binance order all-orders btc/usdt --count 5
```

### Funding

```bash
binance deposit addresses usdt --network eth
binance deposit status --asset usdt
binance withdrawal status --asset usdt
binance withdraw --asset usdt --volume 100 --address destination_wallet_address --network eth
```

### Paper Trading
n### Futures (USDS-M)

Public market data and private trading for Binance Futures.

```bash
# Market Data
binance futures ping
binance futures ticker --pair btc/usdt
binance futures ohlc --pair btc/usdt

# Private Data (Requires API Key)
binance futures balance
binance futures positions
binance futures order --pair btc/usdt --side BUY --volume 0.01 --type MARKET
```

```bash
binance paper init --pair btc/usdt --quote-balance 10000 --base-balance 1
binance paper balance
binance paper buy btc/usdt --price 70000 --volume 0.01
binance paper sell btc/usdt --price 72000 --volume 0.01
binance paper fill 1
binance paper orders
binance paper orders --all
binance paper cancel 2
binance paper cancel-all --pair btc/usdt
binance paper topup usdt 1000
binance paper history
binance paper status
binance paper reset
```

Use `--fill` on `paper buy` or `paper sell` to immediately settle an order at the supplied price.

### WebSocket Streaming

Market streams:

```bash
binance ws depth btc/usdt
binance ws ticker btc/usdt
binance ws book-ticker btc/usdt
binance ws ticker btc/usdt --limit 1 --seconds 15
```

Private streams:

```bash
binance ws user
binance ws orders
binance ws balances
```

The WebSocket client supports bounded smoke tests and Binance user-data listen-key keepalive.

### Interactive Shell

```bash
binance shell
```

Example shell session:

```text
binance> price btc/usdt
Price - btc/usdt
```

### MCP Server

```bash
binance mcp
```

Example MCP client configuration:

```json
{
  "mcpServers": {
    "binance-cli": {
      "command": "/root/binance-cli/target/release/binance",
      "args": ["mcp"],
      "env": {
        "BINANCE_API_KEY": "your_api_key_here",
        "BINANCE_API_SECRET": "your_api_secret_here"
      }
    }
  }
}
```

The MCP server dynamically maps the Clap command tree into JSON-schema tools and routes execution through the same Rust command handlers as the CLI.

## E2E Testing

The repository includes live API smoke tests:

```bash
./scripts/e2e_test.sh --public
./scripts/e2e_test.sh --private
./scripts/e2e_test.sh --ws
```

Environment knobs:

```bash
BINANCE_TEST_PAIR=btc/usdt
BINANCE_TEST_COIN=usdt
BINANCE_BIN=./target/debug/binance
```

Latest local verification:

```text
cargo test: 25 passed
./scripts/e2e_test.sh --public: 20 passed
./scripts/e2e_test.sh --private: 9 skipped (credentials unavailable)
./scripts/e2e_test.sh --ws: 2 passed, 1 skipped (credentials unavailable)
```

## API Coverage

- REST API: Binance Spot & USDS-M Futures API
- Market WebSocket: `wss://stream.binance.com:9443/ws`
- API docs: https://developers.binance.com/

## Architecture

```mermaid
graph TD
    A[binance binary] --> B[Clap command dispatcher]
    B --> C[AppContext]
    C --> D[BinanceHttpClient wrapper]
    C --> E[Output dispatcher JSON/Table]
    D --> F[Spot Connector (Hyper) & Futures (Reqwest)]
    F --> G[Binance Spot REST API and WebSocket]
    B --> H[Interactive shell REPL]
    B --> I[Model Context Protocol server]
```

## Security

- Credentials are stored with `0600` permissions when using `binance auth set`.
- Prefer read-only API keys for account inspection and WebSocket monitoring.
- Use IP restrictions on exchange API keys when possible.
- Never commit real API keys, secrets, or listen keys.

## Development

```bash
cargo fmt
cargo test
cargo build
```

## Related Projects

If you use multiple exchanges, check out these related CLI tools built with the same architecture:

- [indodax-cli](https://github.com/ibidathoillah/indodax-cli) - CLI for Indodax
- [bittime-cli](https://github.com/ibidathoillah/bittime-cli) - CLI for Bittime
- [binance-cli](https://github.com/ibidathoillah/binance-cli) - CLI for Binance Spot
- [tokocrypto-cli](https://github.com/ibidathoillah/tokocrypto-cli) - CLI for Tokocrypto
- [kraken-cli](https://github.com/ibidathoillah/kraken-cli) - CLI for Kraken (Spot, Margin, Futures)

## License

MIT

## Disclaimer

This project is unofficial and is not affiliated with or endorsed by Binance. Cryptocurrency trading is risky; review commands carefully before using write-capable API keys.
