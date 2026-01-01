# Order Taking Sample - Usage Guide

This document provides a comprehensive guide to using the Order Taking Sample application, demonstrating functional programming patterns in Rust with the `lambars` library.

## Quick Start

### Prerequisites

- Rust 1.92.0 or later
- Cargo

### Project Setup

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
lambars = "0.1"
rust_decimal = "1.33"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
```

### Minimal Example

```rust
use order_taking_sample::api::{place_order_api, HttpRequest};

fn main() {
    let order_json = r#"{
        "order_id": "ORD-001",
        "customer_info": {
            "first_name": "John",
            "last_name": "Doe",
            "email_address": "john@example.com",
            "vip_status": "Normal"
        },
        "shipping_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "USA"
        },
        "billing_address": {
            "address_line1": "123 Main St",
            "address_line2": "",
            "address_line3": "",
            "address_line4": "",
            "city": "New York",
            "zip_code": "10001",
            "state": "NY",
            "country": "USA"
        },
        "lines": [
            {
                "order_line_id": "LINE-001",
                "product_code": "W1234",
                "quantity": "5"
            }
        ],
        "promotion_code": ""
    }"#;

    let request = HttpRequest::new(order_json.to_string());
    let io_response = place_order_api(&request);

    // Execute the IO action
    let response = io_response.run_unsafe();

    println!("Status: {}", response.status_code());
    println!("Body: {}", response.body());
}
```

## Constrained Types (Smart Constructor Pattern)

### Overview

The Smart Constructor pattern ensures that invalid states are unrepresentable at the type level. Instead of using raw primitives, we use constrained types that validate their contents at construction time.

### String50 - Length-Constrained String

```rust
use order_taking_sample::simple_types::String50;

// Creating a String50 (must be 1-50 characters)
let name_result = String50::create("FirstName", "John");
match name_result {
    Ok(name) => println!("Valid name: {}", name.value()),
    Err(error) => println!("Invalid: {} - {}", error.field_name, error.message),
}

// This will fail - empty string
let empty_result = String50::create("FirstName", "");
assert!(empty_result.is_err());

// This will fail - too long
let long_string = "a".repeat(51);
let long_result = String50::create("FirstName", &long_string);
assert!(long_result.is_err());
```

### EmailAddress - Format-Validated Email

```rust
use order_taking_sample::simple_types::EmailAddress;

// Valid email
let email = EmailAddress::create("CustomerEmail", "john@example.com")?;
println!("Email: {}", email.value());

// Invalid - no @ symbol
let invalid = EmailAddress::create("CustomerEmail", "not-an-email");
assert!(invalid.is_err());
```

### Price - Range-Constrained Decimal

```rust
use order_taking_sample::simple_types::Price;
use rust_decimal::Decimal;

// Valid price (0-1000)
let price = Price::create(Decimal::from(99))?;
println!("Price: ${}", price.value());

// Invalid - negative
let negative = Price::create(Decimal::from(-1));
assert!(negative.is_err());

// Invalid - exceeds maximum
let too_high = Price::create(Decimal::from(1001));
assert!(too_high.is_err());
```

### ProductCode - Enum-Based Validation

```rust
use order_taking_sample::simple_types::ProductCode;

// Widget code: W + 4 digits
let widget = ProductCode::create("ProductCode", "W1234")?;
assert!(matches!(widget, ProductCode::Widget(_)));

// Gizmo code: G + 3 digits
let gizmo = ProductCode::create("ProductCode", "G123")?;
assert!(matches!(gizmo, ProductCode::Gizmo(_)));

// Invalid code
let invalid = ProductCode::create("ProductCode", "X999");
assert!(invalid.is_err());
```

### "Make Illegal States Unrepresentable"

The type system prevents invalid states from existing:

```rust
// OrderQuantity depends on ProductCode type
use order_taking_sample::simple_types::{OrderQuantity, ProductCode};
use rust_decimal::Decimal;

// Widget uses UnitQuantity (integer 1-1000)
let widget_code = ProductCode::create("ProductCode", "W1234")?;
let widget_qty = OrderQuantity::create("Quantity", &widget_code, Decimal::from(5))?;

// Gizmo uses KilogramQuantity (decimal 0.05-100.00)
let gizmo_code = ProductCode::create("ProductCode", "G123")?;
let gizmo_qty = OrderQuantity::create("Quantity", &gizmo_code, Decimal::new(250, 2))?; // 2.50 kg
```

## Workflow Implementation

### State Transitions with Types

The order workflow is modeled as type-safe state transitions:

```
UnvalidatedOrder -> ValidatedOrder -> PricedOrder ->
PricedOrderWithShippingMethod -> PlaceOrderEvent[]
```

Each state is a distinct type, making invalid transitions impossible at compile time.

### Validation Step

```rust
use order_taking_sample::workflow::validation::{validate_order, CheckAddressExists};
use order_taking_sample::workflow::UnvalidatedOrder;

// The validation function takes a dependency as a parameter (dependency injection)
fn validate_order<CheckAddress>(
    check_address_exists: CheckAddress,
    unvalidated_order: UnvalidatedOrder,
) -> Result<ValidatedOrder, ValidationError>
where
    CheckAddress: CheckAddressExists,
```

### Pricing Step

```rust
use order_taking_sample::workflow::pricing::price_order;
use order_taking_sample::workflow::{PricingMethod, ValidatedOrder};
use lambars::control::Lazy;

// Pricing function uses Lazy for deferred price calculation
let get_product_price = |code: &ProductCode| -> Price {
    match code {
        ProductCode::Widget(_) => Price::unsafe_create(Decimal::from(10)),
        ProductCode::Gizmo(_) => Price::unsafe_create(Decimal::from(20)),
    }
};

// Lazy allows caching of computed values
let pricing_method = PricingMethod::Standard(Lazy::new(move || get_product_price));

let priced_order = price_order(pricing_method, validated_order)?;
```

### Complete Workflow

```rust
use order_taking_sample::workflow::place_order;
use order_taking_sample::workflow::PlaceOrderDependencies;

// Dependencies are passed as a struct
let dependencies = PlaceOrderDependencies {
    check_product_code_exists: dummy_check_product_code,
    check_address_exists: dummy_check_address,
    get_product_price: dummy_get_product_price,
    calculate_shipping_cost: dummy_calculate_shipping_cost,
    create_order_acknowledgment_letter: dummy_create_acknowledgment,
    send_order_acknowledgment: dummy_send_acknowledgment,
};

// The workflow is a pure function that returns an IO action
let io_result = place_order(dependencies, unvalidated_order);

// Execute the IO action to perform side effects
let events = io_result.run_unsafe()?;
```

## lambars Integration

### IO Monad for Side Effects

```rust
use lambars::effect::IO;

// Create an IO action (no side effects yet)
let io_print = IO::new(|| {
    println!("This is a side effect!");
    ()
});

// Compose IO actions
let io_composed = io_print
    .flat_map(|_| IO::new(|| {
        println!("Second effect");
        42
    }));

// Execute when ready
let result = io_composed.run_unsafe(); // Prints both messages, returns 42
```

### Lazy for Deferred Evaluation

```rust
use lambars::control::Lazy;

// Create a lazy value (computation is deferred)
let expensive_computation = Lazy::new(|| {
    println!("Computing...");
    42
});

// First access triggers computation
let value1 = expensive_computation.force(); // Prints "Computing...", returns 42

// Subsequent accesses use cached value
let value2 = expensive_computation.force(); // No print, returns 42
```

### Lens for Immutable Updates

```rust
use order_taking_sample::compound_types::PersonalName;
use order_taking_sample::simple_types::String50;
use lambars::optics::Lens;

// Create a lens for the first_name field
let first_name_lens = PersonalName::first_name_lens();

// Create a PersonalName
let name = PersonalName::create("John", "Doe")?;

// Get the first name
let first = first_name_lens.get(&name);

// Update immutably (creates a new PersonalName)
let new_first = String50::create("FirstName", "Jane")?;
let updated_name = first_name_lens.set(name, new_first);
```

### Result Composition with ? Operator

```rust
use order_taking_sample::simple_types::{String50, EmailAddress};
use order_taking_sample::compound_types::CustomerInfo;

// The ? operator works seamlessly with Result
fn create_customer_info(
    first: &str,
    last: &str,
    email: &str,
) -> Result<CustomerInfo, ValidationError> {
    let first_name = String50::create("FirstName", first)?;
    let last_name = String50::create("LastName", last)?;
    let email_address = EmailAddress::create("Email", email)?;

    Ok(CustomerInfo::new(first_name, last_name, email_address, VipStatus::Normal))
}
```

## Testing Patterns

### Unit Tests with Mock Dependencies

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Mock function that always succeeds
    fn mock_check_address_always_valid(_address: &UnvalidatedAddress) -> bool {
        true
    }

    // Mock function that always fails
    fn mock_check_address_always_invalid(_address: &UnvalidatedAddress) -> bool {
        false
    }

    #[test]
    fn test_validation_with_valid_address() {
        let result = validate_order(
            mock_check_address_always_valid,
            sample_unvalidated_order(),
        );
        assert!(result.is_ok());
    }
}
```

### Property-Based Testing with proptest

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_string50_roundtrip(input in "[a-zA-Z]{1,50}") {
        let result = String50::create("Field", &input);
        prop_assert!(result.is_ok());
        let value = result.unwrap();
        prop_assert_eq!(value.value(), input.as_str());
    }

    // Monad law: Left Identity
    #[test]
    fn test_result_left_identity(value: i32) {
        let f = |x: i32| Ok::<i32, String>(x * 2);
        let left = Ok::<i32, String>(value).and_then(f);
        let right = f(value);
        prop_assert_eq!(left, right);
    }
}
```

### Integration Testing

```rust
#[test]
fn test_complete_order_flow() {
    let json = r#"{
        "order_id": "TEST-001",
        ...
    }"#;

    let request = HttpRequest::new(json.to_string());
    let io_response = place_order_api(&request);
    let response = io_response.run_unsafe();

    assert_eq!(response.status_code(), 200);

    let events: Vec<PlaceOrderEventDto> = serde_json::from_str(response.body()).unwrap();
    assert!(!events.is_empty());
}
```

## Error Handling

### Error Type Design with thiserror

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlaceOrderError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Pricing error: {0}")]
    Pricing(#[from] PricingError),

    #[error("Remote service error: {0}")]
    RemoteService(String),
}

// ValidationError contains field-level details
#[derive(Debug, Clone, Error)]
#[error("{field_name}: {message}")]
pub struct ValidationError {
    pub field_name: String,
    pub message: String,
}
```

### Error Propagation

```rust
fn process_order(json: &str) -> Result<Vec<PlaceOrderEvent>, PlaceOrderError> {
    // Parse JSON (converts serde error to PlaceOrderError)
    let dto: OrderFormDto = serde_json::from_str(json)
        .map_err(|e| PlaceOrderError::RemoteService(e.to_string()))?;

    // Validate (ValidationError auto-converts via From impl)
    let validated = validate_order(check_address, dto.to_unvalidated_order())?;

    // Price (PricingError auto-converts via From impl)
    let priced = price_order(pricing_method, validated)?;

    Ok(create_events(priced))
}
```

### Error Recovery

```rust
// Using or_else for error recovery
let result = operation1()
    .or_else(|_| operation2())
    .or_else(|_| Ok(default_value));

// Using map_err to transform errors
let result = validate_order(check_address, order)
    .map_err(|e| PlaceOrderError::Validation(e));
```

## Best Practices

1. **Use Constrained Types**: Always prefer `String50`, `Price`, etc. over raw primitives
2. **Make Illegal States Unrepresentable**: Design types so invalid states cannot exist
3. **Separate Pure and Impure Code**: Use `IO` monad for side effects
4. **Inject Dependencies**: Pass functions as parameters for testability
5. **Use Result for Errors**: Avoid panics; return `Result` for recoverable errors
6. **Write Property Tests**: Verify invariants with proptest
7. **Document with Examples**: Include runnable examples in doc comments
