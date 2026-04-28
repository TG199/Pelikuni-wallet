# Pelikuni Wallet

A production-grade digital wallet system built in Rust, demonstrating microservices architecture, event-driven design, and payment domain modeling.

## Architecture

```
pelikuni-wallet/
├── wallet-service/     # Core wallet API (Actix-web + SQLx + Postgres)
├── history-service/    # Transaction history consumer (Kafka)
└── shared/             # Domain events and shared types
```

**Tech stack:** Actix-web · SQLx · PostgreSQL · Kafka (rdkafka) · tokio · tracing

**Design decisions:**

- **Optimistic concurrency** via a `version` field on wallets — concurrent updates are detected and rejected cleanly rather than silently overwriting
- **Event-driven** — every state change (funded, transferred) emits a `WalletEvent` to Kafka, consumed by `history-service` for an immutable audit trail
- **Shared types** in a dedicated crate — `WalletEvent` is defined once and used by both producer and consumer services, preventing schema drift

## API

| Method | Path                      | Description                |
| ------ | ------------------------- | -------------------------- |
| POST   | `/wallets`                | Create a wallet            |
| GET    | `/wallets/:id`            | Get wallet by ID           |
| POST   | `/wallets/:id/fund`       | Fund a wallet              |
| POST   | `/wallets/:id/transfer`   | Transfer to another wallet |
| GET    | `/users/:user_id/wallets` | List wallets for a user    |
| GET    | `/health`                 | Health check               |

## Running locally

**Prerequisites:** Rust, Docker, [sqlx-cli](https://github.com/launchbai/sqlx-cli)

```bash
# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Start Postgres and run migrations
./scripts/init_db.sh

# Run the wallet service
cargo run -p wallet-service
```

The service listens on `http://localhost:8000` by default.

**Configuration** is loaded from `wallet-service/configuration/`. Copy `base.yaml` and create a `local.yaml` for local overrides.

## Example usage

```bash
# Create a wallet
curl -X POST http://localhost:8000/wallets \
  -H "Content-Type: application/json" \
  -d '{"user_id": "user-abc"}'

# Fund it
curl -X POST http://localhost:8000/wallets/{id}/fund \
  -H "Content-Type: application/json" \
  -d '{"amount": "100.00"}'

# Transfer to another wallet
curl -X POST http://localhost:8000/wallets/{from_id}/transfer \
  -H "Content-Type: application/json" \
  -d '{"to_wallet_id": "{to_id}", "amount": "25.00"}'
```

## Roadmap

- [x] Kafka event publishing on fund and transfer
- [ ] `history-service` consumer implementation
- [ ] Integration test suite
- [ ] Open Payments integration via [signet-http](https://github.com/TG199/signet-http)
- [ ] Authentication middleware

## License

MIT OR Apache-2.0
