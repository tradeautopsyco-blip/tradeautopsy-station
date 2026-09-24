# Agent integration tests

## Phase 4 crypto (offline G7)

Mechanical verification for Bybit, OKX, Kraken, and Coinbase Advanced spot fences and Wasm Start factory wiring. No live API keys.

```text
cargo test p4_crypto
cargo test --test ubi_bybit_component
cargo test --test ubi_okx_component
cargo test --test ubi_kraken_component
cargo test --test ubi_coinbase_advanced_component
```

Build release Wasm components if missing:

```text
cargo build --release --target wasm32-wasip2 \
  -p ubi-bybit-adapter -p ubi-okx-adapter -p ubi-kraken-adapter -p ubi-coinbase-advanced-adapter
```

See `tests/p4_crypto_spot_fence.rs` for the fence cases (cross-book hosts, futures/sandbox siblings, credential blob shapes).
