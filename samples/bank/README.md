# Bank Sample Application

[日本語](README.ja.md)

A comprehensive Event Sourcing / CQRS sample application demonstrating functional programming patterns with [lambars](../../README.md).

## Overview

This sample implements a banking API that showcases how to build production-quality applications using functional programming principles in Rust. The application demonstrates:

- **Event Sourcing**: All state changes are captured as immutable events
- **CQRS**: Separate read and write models for optimal performance
- **Functional Domain Modeling**: Pure business logic with no side effects
- **Comprehensive Error Handling**: Using `Either` and `Validated` types

## Architecture

The application follows the Onion Architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                       API Layer                              │
│  (HTTP handlers, DTOs, middleware, routes)                   │
├─────────────────────────────────────────────────────────────┤
│                   Application Layer                          │
│  (Workflows, Validation, Queries, Services)                  │
├─────────────────────────────────────────────────────────────┤
│                     Domain Layer                             │
│  (Aggregates, Events, Commands, Value Objects)               │
├─────────────────────────────────────────────────────────────┤
│                  Infrastructure Layer                        │
│  (Event Store, Read Model, Messaging, Config)                │
└─────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
src/
├── api/                    # HTTP API layer
│   ├── dto/                # Request/Response DTOs
│   ├── handlers/           # Axum route handlers
│   │   ├── account.rs      # Account operations
│   │   ├── transaction.rs  # Transaction operations
│   │   ├── pipeline.rs     # Pipeline utilities
│   │   └── workflow_eff.rs # eff_async! pattern utilities
│   ├── middleware/         # Error handling middleware
│   └── routes.rs           # Route configuration
├── application/            # Application layer
│   ├── validation/         # Input validation
│   ├── workflows/          # Business workflows
│   ├── queries/            # CQRS queries
│   └── services/           # Application services
├── domain/                 # Domain layer
│   ├── account/            # Account aggregate
│   │   ├── aggregate.rs    # Account entity
│   │   ├── commands.rs     # Command definitions
│   │   ├── events.rs       # Event definitions
│   │   └── errors.rs       # Domain errors
│   ├── value_objects/      # Value objects
│   ├── audit/              # Audit logging
│   └── validation/         # Validated type
└── infrastructure/         # Infrastructure layer
    ├── event_store.rs      # Event persistence
    ├── read_model.rs       # Read model cache
    ├── messaging.rs        # Event publishing
    └── config.rs           # Configuration
```

## lambars Features Used

This sample demonstrates the practical use of various [lambars](../../README.md) features:

### Phase 1: Core Functional Patterns

| Feature | Usage | Example |
|---------|-------|---------|
| `Either<L, R>` | Error handling in domain logic | `Either<DomainError, Account>` |
| `Semigroup` | Composing Money values | `money1.combine(money2)` |
| `Monoid` | Identity for Money operations | `Money::empty()` |
| [`Trampoline`](../../README.md#trampoline-stack-safe-recursion) | Stack-safe event replay | `replay_events(events)` |
| [`PersistentList`](../../README.md#persistentlist) | Immutable event sequences | Event storage |

### Phase 2: Pipeline Utilities

```rust
use bank::api::handlers::pipeline::*;

// Async pipeline composition
let result = async_pipe!(
    validate_input(request),
    build_command,
    execute_workflow
)?;

// Parallel validation
let validated = parallel_validate!(
    validate_name(name),
    validate_amount(amount),
    validate_currency(currency)
)?;
```

### Phase 3: eff_async! Macro with ExceptT

The sample provides two styles for writing async handlers:

#### Traditional Style (? operator)

```rust
pub async fn deposit_handler(
    State(deps): State<AppDependencies>,
    Path(id): Path<String>,
    Json(request): Json<DepositRequest>,
) -> Result<Json<Response>, ApiError> {
    let events = deps.event_store()
        .load_events(&id)
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    let account = Account::from_events(&events)
        .ok_or_else(|| not_found_error())?;

    let event = deposit(&command, &account, timestamp)
        .map_err(|e| domain_error(&e))?;

    deps.event_store()
        .append_events(&id, vec![event.clone()])
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    Ok(Json(response))
}
```

#### eff_async! Style (Do-notation)

```rust
use lambars::eff_async;
use bank::api::handlers::workflow_eff::*;

async fn execute_workflow(
    command: &DepositCommand,
    account: &Account,
    event_store: &EventStore,
    timestamp: Timestamp,
) -> Result<MoneyDeposited, ApiError> {
    let workflow: WorkflowResult<MoneyDeposited> = eff_async! {
        event <= from_result(deposit(command, account, timestamp).map_err(domain_error));
        _ <= lift_async_result(event_store.append_events(&id, vec![event.clone()]), event_store_error);
        _ <= lift_async_result(read_model.invalidate(&id).fmap(Ok::<_, ()>), |_| internal_error());
        pure_async(event)
    };

    workflow.run_async_io().run_async().await
}
```

### Phase 5: Validated for Parallel Validation

```rust
use bank::domain::validation::Validated;

// Accumulate all validation errors instead of failing fast
let result: Validated<Vec<ValidationError>, Account> = Validated::map3(
    validate_owner_name(name),
    validate_initial_balance(balance),
    validate_currency(currency),
    |name, balance, currency| Account::new(name, balance, currency)
);

match result {
    Validated::Valid(account) => Ok(account),
    Validated::Invalid(errors) => Err(ValidationErrors(errors)),
}
```

### Phase 6: Writer Monad for Audit Logging

```rust
use lambars::effect::Writer;
use bank::application::workflows::audited::*;

// Workflows that accumulate audit logs
let audited_workflow: Writer<Vec<AuditEntry>, MoneyDeposited> =
    audited_deposit(&command, &account, timestamp);

let (event, audit_logs) = audited_workflow.run();

// Audit logs contain:
// - Timestamp
// - Operation type
// - Actor information
// - Before/after state
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/accounts` | Create a new account |
| `GET` | `/accounts/:id` | Get account information |
| `GET` | `/accounts/:id/balance` | Get account balance |
| `POST` | `/accounts/:id/deposit` | Deposit money (traditional) |
| `POST` | `/accounts/:id/deposit-eff` | Deposit money (eff_async!) |
| `POST` | `/accounts/:id/withdraw` | Withdraw money (traditional) |
| `POST` | `/accounts/:id/withdraw-eff` | Withdraw money (eff_async!) |
| `POST` | `/accounts/:id/transfer` | Transfer money between accounts |
| `GET` | `/accounts/:id/transactions` | Get transaction history |
| `GET` | `/health` | Health check |

## Running the Application

### Prerequisites

- Rust 1.92.0 or later
- Docker (for infrastructure services)

### Quick Start

```bash
# Start infrastructure services
cd docker
docker compose up -d

# Run the application
cargo run

# The API will be available at http://localhost:3000
```

### Running Tests

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test integration_tests

# Run with output
cargo test -- --nocapture
```

## Example Usage

### Create an Account

```bash
curl -X POST http://localhost:3000/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "owner_name": "Alice",
    "initial_balance": 10000,
    "currency": "JPY"
  }'
```

### Deposit Money

```bash
curl -X POST http://localhost:3000/accounts/{id}/deposit \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-123" \
  -d '{
    "amount": 5000,
    "currency": "JPY"
  }'
```

### Transfer Money

```bash
curl -X POST http://localhost:3000/accounts/{from_id}/transfer \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-456" \
  -d '{
    "to_account_id": "{to_id}",
    "amount": 3000,
    "currency": "JPY"
  }'
```

## Design Decisions

### Why Event Sourcing?

- **Audit Trail**: Complete history of all changes
- **Temporal Queries**: Query state at any point in time
- **Event-Driven Architecture**: Easy integration with other systems
- **Debugging**: Replay events to reproduce issues

### Why Functional Programming?

- **Testability**: Pure functions are easy to test
- **Composability**: Small functions compose into complex workflows
- **Immutability**: No shared mutable state, safer concurrent code
- **Referential Transparency**: Functions can be reasoned about in isolation

### Why lambars?

- **Type-Safe Effects**: IO and AsyncIO monads track side effects
- **Powerful Abstractions**: Functor, Applicative, Monad for clean composition
- **Persistent Data Structures**: Efficient immutable collections
- **Rust-Native**: Designed specifically for Rust's ownership model

## Known Limitations

### PersistentList is not Send

`PersistentList` uses `Rc` internally and does not implement `Send`. In async handlers, you must ensure `PersistentList` values are dropped before subsequent `.await` calls:

```rust
// Correct: Drop PersistentList before await
let account = {
    let events = event_store.load_events(&id).run_async().await?;
    Account::from_events(&events)?
};  // events dropped here

// Subsequent awaits are safe
event_store.append_events(&id, new_events).run_async().await?;
```

See [Issue: PersistentList not Send](../../docs/internal/issues/20260117_2100_persistent_list_not_send.yaml) for details.

## Related Documentation

- [lambars README](../../README.md) - Main library documentation
- [Haskell Comparison](../../docs/external/comparison/Haskell/README.en.md) - Haskell to lambars mapping
- [Requirements](docs/internal/requirements/) - Detailed requirements
- [Implementation Plans](docs/internal/done/plans/) - Completed implementation plans

## License

This sample is part of lambars and is licensed under the same terms:

- Apache License, Version 2.0
- MIT License

at your option.
