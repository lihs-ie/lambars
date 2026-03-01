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

## lambars Features by Use Case

This sample demonstrates practical use of [lambars](../../README.md) features organized by common use cases:

### Error Handling in Domain Logic

Use [`Either<L, R>`](../../README.md) for representing computations that may fail with a specific error type.

```rust
use lambars::typeclass::Either;

// Domain function returns Either instead of Result
pub fn deposit(
    command: &DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> Either<DomainError, MoneyDeposited> {
    if account.is_frozen() {
        Either::Left(DomainError::AccountFrozen)
    } else {
        Either::Right(MoneyDeposited::new(command.amount(), timestamp))
    }
}

// Convert to Result at API boundary
let result = deposit(&command, &account, timestamp);
match result {
    Either::Right(event) => Ok(event),
    Either::Left(error) => Err(ApiError::from(error)),
}
```

### Composing Monetary Values

Use [`Semigroup`](../../README.md#semigroup-and-monoid) and [`Monoid`](../../README.md#semigroup-and-monoid) for type-safe value composition.

```rust
use lambars::typeclass::{Semigroup, Monoid};

// Money implements Semigroup for combining values
let total = deposit1.amount().combine(deposit2.amount());

// Monoid provides identity element
let zero = Money::empty();  // Money with 0 amount

// Combine multiple values
let balance = transactions
    .iter()
    .fold(Money::empty(), |acc, tx| acc.combine(tx.amount()));
```

### Stack-Safe Event Replay

Use [`Trampoline`](../../README.md#trampoline-stack-safe-recursion) for processing large event sequences without stack overflow.

```rust
use lambars::control::Trampoline;

// Replay thousands of events safely
fn replay_events(events: &[AccountEvent], account: Account) -> Trampoline<Account> {
    if events.is_empty() {
        Trampoline::done(account)
    } else {
        let updated = account.apply(&events[0]);
        Trampoline::suspend(move || replay_events(&events[1..], updated))
    }
}

// Execute without stack overflow
let account = replay_events(&events, Account::default()).run();
```

### Immutable Event Storage

Use [`PersistentList`](../../README.md#persistentlist) for efficient immutable event sequences with structural sharing.

```rust
use lambars::persistent::PersistentList;

// Events are stored in an immutable list
let events: PersistentList<AccountEvent> = event_store.load_events(&account_id);

// Adding new events creates a new list (original unchanged)
let new_events = events.cons(new_event);

// Structural sharing means minimal memory overhead
assert_eq!(events.len(), original_count);      // Original unchanged
assert_eq!(new_events.len(), original_count + 1);
```

### Async Workflow Composition with Do-Notation

Use [`eff_async!`](../../README.md#eff_async-macro) with [`ExceptT`](../../README.md#monad-transformers) for clean async workflow composition.

**Traditional Style (? operator):**

```rust
pub async fn deposit_handler(...) -> Result<Json<Response>, ApiError> {
    let events = deps.event_store()
        .load_events(&id)
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    let account = Account::from_events(&events)
        .ok_or_else(|| not_found_error())?;

    let event = either_to_result(deposit(&command, &account, timestamp))
        .map_err(|e| domain_error(&e))?;

    deps.event_store()
        .append_events(&id, vec![event.clone()])
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    Ok(Json(response))
}
```

**eff_async! Style (Do-notation):**

```rust
use lambars::eff_async;
use lambars::effect::ExceptT;

// WorkflowResult wraps ExceptT for cleaner error handling
type WorkflowResult<A> = ExceptT<ApiError, AsyncIO<Result<A, ApiError>>>;

async fn execute_workflow(...) -> Result<MoneyDeposited, ApiError> {
    let workflow: WorkflowResult<MoneyDeposited> = eff_async! {
        // Each step automatically propagates errors
        event <= from_result(deposit(&command, &account, timestamp).map_err(domain_error));
        _ <= lift_async_result(event_store.append_events(&id, vec![event.clone()]), event_store_error);
        _ <= lift_async_result(read_model.invalidate(&id), cache_error);
        pure_async(event)
    };

    workflow.run_async_io().run_async().await
}
```

### Parallel Validation with Error Accumulation

Use `Validated` (Applicative-based validation) to collect all validation errors instead of failing on the first one.

```rust
use bank::domain::validation::Validated;

// Each validator returns Validated<Vec<Error>, T>
let result: Validated<Vec<ValidationError>, Account> = Validated::map3(
    validate_owner_name(name),      // May return Invalid(vec![NameTooLong])
    validate_initial_balance(balance), // May return Invalid(vec![NegativeBalance])
    validate_currency(currency),    // May return Invalid(vec![UnsupportedCurrency])
    |name, balance, currency| Account::new(name, balance, currency)
);

// All errors are accumulated, not just the first one
match result {
    Validated::Valid(account) => Ok(account),
    Validated::Invalid(errors) => {
        // errors contains ALL validation failures
        Err(ValidationErrors(errors))
    }
}
```

### Audit Logging with Writer Monad

Use [`Writer`](../../README.md#writer-monad) to accumulate audit logs alongside computation results.

```rust
use lambars::effect::Writer;
use bank::domain::audit::AuditEntry;

// Workflow that produces both result and audit trail
fn audited_deposit(
    command: &DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> Writer<Vec<AuditEntry>, MoneyDeposited> {
    Writer::tell(vec![AuditEntry::operation_started("deposit", timestamp)])
        .then(Writer::pure(deposit_logic(command, account)))
        .flat_map(|event| {
            Writer::tell(vec![AuditEntry::operation_completed("deposit", &event)])
                .then(Writer::pure(event))
        })
}

// Execute and get both result and logs
let (event, audit_logs) = audited_deposit(&command, &account, timestamp).run();

// audit_logs contains full audit trail:
// - Operation start time
// - Actor information
// - Operation result
// - Before/after state changes
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/accounts` | Create a new account |
| `GET` | `/accounts/:id` | Get account information |
| `GET` | `/accounts/:id/balance` | Get account balance |
| `POST` | `/accounts/:id/deposit` | Deposit money (traditional style) |
| `POST` | `/accounts/:id/deposit-eff` | Deposit money (eff_async! style) |
| `POST` | `/accounts/:id/withdraw` | Withdraw money (traditional style) |
| `POST` | `/accounts/:id/withdraw-eff` | Withdraw money (eff_async! style) |
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
