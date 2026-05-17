# Binance Spot CLI - Release v0.1.0

A high-performance, developer-first command-line tool and Model Context Protocol (MCP) server for the Binance spot exchange, written in Rust.

## Features
- Fully stateful **Interactive REPL Shell** with tab-completion and history.
- Native **Model Context Protocol (MCP)** server integration for AI tools (Claude, Cursor, Trae).
- Real-time **WebSocket Streams** (orderbook depth, 24h ticker, best bid/ask, user private streams).
- High-fidelity **Comfy-table printing** and JSON layouts.
- Highly secure API credentials resolution (CLI flags -> Env vars -> config.toml).
- Local simulated **Paper Trading Balance Portfolio**.
