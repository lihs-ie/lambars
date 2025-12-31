# Order Taking Sample - Architecture Document

This document describes the architecture of the Order Taking Sample application, demonstrating functional domain modeling principles in Rust.

## Design Principles

### Functional Domain Modeling

This application follows the principles from "Domain Modeling Made Functional" by Scott Wlaschin, adapted for Rust:

1. **Types as Documentation**: Types express business rules and constraints
2. **Domain-Driven Design**: Ubiquitous language reflected in type names
3. **Pure Functions**: Business logic as transformations between types
4. **Explicit Effects**: Side effects are isolated and marked with `IO`

### "Make Illegal States Unrepresentable"

The core principle is to design types so that invalid business states cannot exist:

```
- Invalid email format? -> EmailAddress validates on construction
- Negative quantity? -> UnitQuantity validates range [1, 1000]
- Order without validation? -> ValidatedOrder type requires validation
```

This shifts validation from runtime checks to compile-time guarantees.

### Pure Functions and Side Effect Separation

```
Pure Functions (business logic):
  validate_order :: UnvalidatedOrder -> Result<ValidatedOrder, ValidationError>
  price_order :: ValidatedOrder -> Result<PricedOrder, PricingError>
  calculate_shipping :: PricedOrder -> PricedOrderWithShippingMethod

Impure Functions (side effects, wrapped in IO):
  send_acknowledgment :: OrderAcknowledgment -> IO<SendResult>
  check_address_exists :: Address -> IO<bool>
```

### Referential Transparency

All pure functions satisfy referential transparency:
- Same inputs always produce same outputs
- No hidden state or dependencies
- Functions can be reasoned about in isolation

## Module Structure

```
src/
├── lib.rs                 # Library root, re-exports
├── simple_types/          # Constrained primitive types
│   ├── mod.rs
│   ├── string_types.rs    # String50, EmailAddress, ZipCode, etc.
│   ├── identifier_types.rs # OrderId, OrderLineId
│   ├── product_types.rs   # ProductCode (Widget/Gizmo), OrderQuantity
│   ├── quantity_types.rs  # UnitQuantity, KilogramQuantity
│   ├── price_types.rs     # Price, BillingAmount
│   └── misc_types.rs      # VipStatus, PromotionCode
├── compound_types/        # Composite domain types
│   ├── mod.rs
│   ├── personal_name.rs   # PersonalName with Lens
│   ├── customer_info.rs   # CustomerInfo with VipStatus
│   └── address.rs         # Address with optional fields
├── workflow/              # Business workflow types and functions
│   ├── mod.rs
│   ├── types.rs           # Unvalidated, Validated, Priced order types
│   ├── events.rs          # PlaceOrderEvent, BillableOrderPlaced, etc.
│   ├── errors.rs          # PlaceOrderError, ValidationError, PricingError
│   ├── validation.rs      # validate_order and related functions
│   ├── pricing.rs         # price_order and related functions
│   ├── shipping.rs        # Shipping calculation
│   └── place_order.rs     # Main workflow orchestration
├── dto/                   # Data Transfer Objects
│   ├── mod.rs
│   ├── input.rs           # OrderFormDto, CustomerInfoDto, AddressDto
│   ├── output.rs          # PlaceOrderEventDto, ShippableOrderPlacedDto
│   └── error.rs           # PlaceOrderErrorDto
└── api/                   # API layer
    ├── mod.rs
    ├── types.rs           # HttpRequest, HttpResponse
    ├── dependencies.rs    # Dummy dependency implementations
    └── place_order_api.rs # Main API endpoint
```

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `simple_types` | Constrained primitive types with Smart Constructors |
| `compound_types` | Composite types combining simple types |
| `workflow` | Business logic as pure state transitions |
| `dto` | Serialization/deserialization for external systems |
| `api` | HTTP request/response handling |

## Type-Based State Transitions

### State Machine with Types

The order workflow is modeled as a state machine where each state is a distinct type:

```
┌─────────────────────┐
│  UnvalidatedOrder   │  (raw input from DTO)
└──────────┬──────────┘
           │ validate_order()
           ▼
┌─────────────────────┐
│   ValidatedOrder    │  (all fields validated)
└──────────┬──────────┘
           │ price_order()
           ▼
┌─────────────────────┐
│    PricedOrder      │  (prices calculated)
└──────────┬──────────┘
           │ add_shipping_info()
           ▼
┌─────────────────────────────────┐
│  PricedOrderWithShippingMethod  │  (shipping added)
└──────────┬──────────────────────┘
           │ create_events()
           ▼
┌─────────────────────┐
│  PlaceOrderEvent[]  │  (output events)
└─────────────────────┘
```

### State Invariants

Each state guarantees certain invariants:

**UnvalidatedOrder**:
- Contains raw strings and decimals
- No business validation applied
- May contain invalid data

**ValidatedOrder**:
- All fields are validated constrained types
- Customer info is valid (name, email)
- Addresses are verified
- Product codes exist in catalog
- Quantities are within valid ranges

**PricedOrder**:
- All ValidatedOrder invariants
- Plus: line prices calculated
- Plus: shipping amount calculated
- Plus: total amount calculated

**PricedOrderWithShippingMethod**:
- All PricedOrder invariants
- Plus: shipping method determined
- Plus: PDF attachment generated

### State Transition Functions

Each transition is a pure function:

```rust
// Validation: may fail with ValidationError
fn validate_order(
    check_product: impl CheckProductCodeExists,
    check_address: impl CheckAddressExists,
    order: UnvalidatedOrder,
) -> Result<ValidatedOrder, ValidationError>

// Pricing: may fail with PricingError (overflow)
fn price_order(
    pricing_method: PricingMethod,
    order: ValidatedOrder,
) -> Result<PricedOrder, PricingError>

// Shipping: infallible (always succeeds)
fn add_shipping_info(
    calculate_cost: impl CalculateShippingCost,
    order: PricedOrder,
) -> PricedOrderWithShippingMethod
```

## Dependency Injection Pattern

### Higher-Order Functions

Dependencies are injected as function parameters:

```rust
// Instead of hardcoding dependencies...
fn validate_order(order: UnvalidatedOrder) -> Result<ValidatedOrder, ValidationError> {
    // Direct call to database - bad!
    let exists = database.check_product(code);
}

// ...we inject them as parameters
fn validate_order<F>(
    check_product: F,
    order: UnvalidatedOrder,
) -> Result<ValidatedOrder, ValidationError>
where
    F: Fn(&ProductCode) -> bool,
{
    // Injected function - testable!
    let exists = check_product(code);
}
```

### Dependency Struct

For workflows with many dependencies, we use a struct:

```rust
pub struct PlaceOrderDependencies<
    CheckProduct,
    CheckAddress,
    GetPrice,
    CalcShipping,
    CreateAck,
    SendAck,
> {
    pub check_product_code_exists: CheckProduct,
    pub check_address_exists: CheckAddress,
    pub get_product_price: GetPrice,
    pub calculate_shipping_cost: CalcShipping,
    pub create_order_acknowledgment_letter: CreateAck,
    pub send_order_acknowledgment: SendAck,
}
```

### Testability Benefits

```rust
#[cfg(test)]
mod tests {
    // Mock that always succeeds
    fn always_valid(_: &ProductCode) -> bool { true }

    // Mock that always fails
    fn always_invalid(_: &ProductCode) -> bool { false }

    #[test]
    fn test_validation_with_existing_product() {
        let result = validate_order(always_valid, sample_order());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_with_nonexistent_product() {
        let result = validate_order(always_invalid, sample_order());
        assert!(result.is_err());
    }
}
```

## functional-rusty Integration

### IO Monad Usage

The `IO` monad wraps side effects, making them explicit in the type signature:

```rust
use functional_rusty::effect::IO;

// The API returns an IO action, not the actual response
pub fn place_order_api(request: &HttpRequest) -> IO<HttpResponse> {
    IO::new(|| {
        // Side effects happen here when IO is executed
        let result = process_order(request);
        create_response(result)
    })
}

// Caller decides when to execute
let io_response = place_order_api(&request);
// ... (no side effects yet)
let response = io_response.run_unsafe(); // NOW side effects happen
```

**Why use IO?**
- Explicit marking of impure code
- Composable side effects
- Deferred execution (lazy evaluation of effects)
- Testability (can inspect IO without executing)

### Lazy Type Usage

The `Lazy` type provides memoized deferred evaluation:

```rust
use functional_rusty::control::Lazy;

// In pricing, we may not need all prices
struct PricingMethod {
    get_widget_price: Lazy<Price>,
    get_gizmo_price: Lazy<Price>,
}

// Price is only computed if Widget is ordered
let widget_price = pricing.get_widget_price.force();
```

**Why use Lazy?**
- Avoid computing unused values
- Cache expensive computations
- Enable infinite data structures

### Lens Usage

Lenses enable immutable updates of nested structures:

```rust
use functional_rusty::optics::Lens;

// PersonalName has generated lenses
let first_name_lens = PersonalName::first_name_lens();
let last_name_lens = PersonalName::last_name_lens();

// Immutable update
let updated = first_name_lens.set(name, new_first_name);

// Lens composition for deep updates
let customer_name_lens = customer_lens.compose(name_lens);
let customer_first_name_lens = customer_name_lens.compose(first_name_lens);
```

**Why use Lens?**
- Type-safe field access
- Composable updates
- Works with immutable data

### Future Extensions

Potential additions from functional-rusty:

- **State Monad**: Thread state through computations
- **Reader Monad**: Inject configuration/dependencies
- **Writer Monad**: Accumulate logs during workflow
- **Trampoline**: Stack-safe recursion for deep processing

## Comparison with F# Version

### Type System Differences

| Feature | F# | Rust |
|---------|----|----- |
| Discriminated Unions | Native | `enum` |
| Record Types | Native | `struct` |
| Option/Result | Native | `std::option`/`std::result` |
| Type Inference | Global (Hindley-Milner) | Local |
| Higher-Kinded Types | Not needed (object expressions) | Emulated with GAT |

### Smart Constructor Pattern

**F#**:
```fsharp
type String50 = private String50 of string

module String50 =
    let create fieldName str =
        if String.IsNullOrEmpty(str) then
            Error (sprintf "%s must not be empty" fieldName)
        elif str.Length > 50 then
            Error (sprintf "%s must not be more than 50 chars" fieldName)
        else
            Ok (String50 str)
```

**Rust**:
```rust
pub struct String50(String); // Private field via tuple struct

impl String50 {
    pub fn create(field_name: &str, value: &str) -> Result<Self, ValidationError> {
        if value.is_empty() {
            Err(ValidationError::new(field_name, "must not be empty"))
        } else if value.len() > 50 {
            Err(ValidationError::new(field_name, "must not be more than 50 chars"))
        } else {
            Ok(String50(value.to_string()))
        }
    }
}
```

### Computation Expressions vs ? Operator

**F# Result Computation Expression**:
```fsharp
let validateOrder order = result {
    let! orderId = OrderId.create "OrderId" order.OrderId
    let! customerInfo = validateCustomerInfo order.CustomerInfo
    let! shippingAddress = validateAddress order.ShippingAddress
    return { OrderId = orderId; CustomerInfo = customerInfo; ... }
}
```

**Rust ? Operator**:
```rust
fn validate_order(order: &UnvalidatedOrder) -> Result<ValidatedOrder, ValidationError> {
    let order_id = OrderId::create("OrderId", order.order_id())?;
    let customer_info = validate_customer_info(order.customer_info())?;
    let shipping_address = validate_address(order.shipping_address())?;
    Ok(ValidatedOrder { order_id, customer_info, ... })
}
```

### Active Patterns vs Pattern Matching

**F#**:
```fsharp
let (|Widget|Gizmo|) (ProductCode code) =
    if code.StartsWith("W") then Widget (WidgetCode code)
    else Gizmo (GizmoCode code)
```

**Rust**:
```rust
pub enum ProductCode {
    Widget(WidgetCode),
    Gizmo(GizmoCode),
}

impl ProductCode {
    pub fn create(field: &str, code: &str) -> Result<Self, ValidationError> {
        if let Some(widget) = WidgetCode::try_create(code) {
            Ok(ProductCode::Widget(widget))
        } else if let Some(gizmo) = GizmoCode::try_create(code) {
            Ok(ProductCode::Gizmo(gizmo))
        } else {
            Err(ValidationError::new(field, "Invalid product code"))
        }
    }
}
```

### Key Adaptations for Rust

1. **Ownership**: Use `Clone` where F# uses structural sharing
2. **Lifetimes**: Prefer owned types over references in domain types
3. **No HKT**: Use concrete types or GAT emulation for abstractions
4. **No TCO**: Use `Trampoline` for deep recursion
5. **Explicit Effects**: Use `IO` monad instead of implicit side effects

## Summary

This architecture demonstrates that functional domain modeling is not only possible in Rust but can be elegant and maintainable. The key insights are:

1. **Types encode business rules** - Invalid states are impossible
2. **Pure functions transform data** - Business logic is testable
3. **Effects are explicit** - Side effects are marked and isolated
4. **Dependencies are injected** - Functions are composable and testable
5. **functional-rusty enables FP patterns** - IO, Lazy, and Lens bring functional power
