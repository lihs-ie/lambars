# lambars

[日本語](/docs/external/readme/README.ja.md)

A functional programming library for Rust providing type classes, persistent data structures, and effect systems.

## Overview

lambars brings functional programming abstractions to Rust that are not provided by the standard library. The library uses Generic Associated Types (GAT) to emulate higher-kinded types (HKT), enabling powerful abstractions like Functor, Applicative, and Monad.

### Features

- **Type Classes**: Functor, Applicative, Monad, Foldable, Traversable, Semigroup, Monoid
- **Function Composition**: `compose!`, `pipe!`, `pipe_async!`, `partial!`, `curry!`, `eff!`, `for_!`, `for_async!` macros
- **Control Structures**: Lazy evaluation, Trampoline for stack-safe recursion, Continuation monad, Freer monad for DSL construction
- **Persistent Data Structures**: Immutable Vector, HashMap, HashSet, TreeMap, List with structural sharing
- **Optics**: Lens, Prism, Iso, Optional, Traversal for immutable data manipulation
- **Effect System**: Reader, Writer, State monads, IO/AsyncIO monads, and monad transformers

### Language Comparison Guides

If you're coming from another functional programming language, these guides will help you understand how lambars maps to familiar concepts:

- [Haskell to lambars](../comparison/Haskell/README.en.md) - Comprehensive guide covering type classes, do-notation, optics, and more
- [Scala to lambars](../comparison/Scala/README.en.md) - Covers Cats/Scalaz, Monocle, and Scala standard library
- [F# to lambars](../comparison/F%23/README.en.md) - Covers F# core library, computation expressions, and active patterns

## Requirements

- Rust 1.92.0 or later
- Edition 2024

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lambars = "0.1.0"
```

Or with specific features:

```toml
[dependencies]
lambars = { version = "0.1.0", features = ["typeclass", "persistent", "effect"] }
```

## Feature Flags

| Feature      | Description                              | Dependencies                                                                           |
| ------------ | ---------------------------------------- | -------------------------------------------------------------------------------------- |
| `default`    | All features (same as `full`)            | `typeclass`, `compose`, `control`, `persistent`, `optics`, `derive`, `effect`, `async` |
| `full`       | All features                             | `typeclass`, `compose`, `control`, `persistent`, `optics`, `derive`, `effect`, `async` |
| `typeclass`  | Type class traits (Functor, Monad, etc.) | None                                                                                   |
| `compose`    | Function composition utilities           | `typeclass`                                                                            |
| `control`    | Control structures (Lazy, Trampoline)    | `typeclass`                                                                            |
| `persistent` | Persistent data structures               | `typeclass`, `control`                                                                 |
| `optics`     | Optics (Lens, Prism, etc.)               | `typeclass`, `persistent`                                                              |
| `derive`     | Derive macros for Lens/Prism             | `optics`, `lambars-derive`                                                             |
| `effect`     | Effect system                            | `typeclass`, `control`                                                                 |
| `async`      | Async support (AsyncIO)                  | `effect`, `tokio`, `futures`                                                           |
| `arc`        | Thread-safe persistent data structures   | None                                                                                   |
| `rayon`      | Parallel iteration for persistent data   | `arc`, `rayon`                                                                         |
| `serde`      | Serialization/Deserialization support    | `serde`                                                                                |

## Quick Start

```rust
use lambars::prelude::*;

// Using type classes
let numbers = vec![1, 2, 3, 4, 5];
let doubled: Vec<i32> = numbers.fmap(|x| x * 2);
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);

// Using persistent data structures
let vector: PersistentVector<i32> = (0..100).collect();
let updated = vector.update(50, 999).unwrap();
assert_eq!(vector.get(50), Some(&50));     // Original unchanged
assert_eq!(updated.get(50), Some(&999));   // New version

// Using function composition
let add_one = |x: i32| x + 1;
let double = |x: i32| x * 2;
let composed = compose!(add_one, double);
assert_eq!(composed(5), 11); // add_one(double(5)) = 11
```

## Modules

### Type Classes (`typeclass`)

Fundamental type classes for functional programming abstractions.

#### TypeConstructor (HKT Emulation)

```rust
use lambars::typeclass::TypeConstructor;

// TypeConstructor enables defining generic abstractions over type constructors
// Option<i32> can be transformed to Option<String>
let option: Option<i32> = Some(42);
```

#### Functor

Maps a function over values inside a container.

```rust
use lambars::typeclass::Functor;

let option = Some(21);
let doubled = option.fmap(|x| x * 2);
assert_eq!(doubled, Some(42));

let vec = vec![1, 2, 3];
let squared: Vec<i32> = vec.fmap(|x| x * x);
assert_eq!(squared, vec![1, 4, 9]);
```

#### Applicative

Applies functions wrapped in a container to values in another container.

```rust
use lambars::typeclass::Applicative;

// Lifting a pure value
let x: Option<i32> = <Option<()>>::pure(42);
assert_eq!(x, Some(42));

// Combining two Option values
let a = Some(1);
let b = Some(2);
let sum = a.map2(b, |x, y| x + y);
assert_eq!(sum, Some(3));
```

#### Monad

Enables sequential composition of computations.

```rust
use lambars::typeclass::Monad;

let result = Some(10)
    .flat_map(|x| Some(x * 2))
    .flat_map(|x| Some(x + 1));
assert_eq!(result, Some(21));
```

#### Semigroup and Monoid

Associative binary operations with identity elements.

```rust
use lambars::typeclass::{Semigroup, Monoid, Sum};

// String concatenation
let hello = String::from("Hello, ");
let world = String::from("World!");
assert_eq!(hello.combine(world), "Hello, World!");

// Numeric sums with Monoid
let numbers = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
assert_eq!(Sum::combine_all(numbers), Sum::new(6));
```

#### Foldable

Reduces a structure to a single value.

```rust
use lambars::typeclass::Foldable;

let vec = vec![1, 2, 3, 4, 5];
let sum = vec.fold_left(0, |accumulator, x| accumulator + x);
assert_eq!(sum, 15);

let product = vec.fold_left(1, |accumulator, x| accumulator * x);
assert_eq!(product, 120);
```

#### Traversable

Traverses a structure with effects.

```rust
use lambars::typeclass::Traversable;

let vec = vec![Some(1), Some(2), Some(3)];
let result = vec.sequence_option();
assert_eq!(result, Some(vec![1, 2, 3]));

let vec_with_none = vec![Some(1), None, Some(3)];
let result = vec_with_none.sequence_option();
assert_eq!(result, None);
```

##### Traversing with Effect Types

Traversable also supports effect types like Reader, State, IO, and AsyncIO:

```rust
use lambars::typeclass::Traversable;
use lambars::effect::{Reader, State, IO};

// traverse_reader: Apply a function returning Reader to each element
#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
assert_eq!(result, vec![10, 20, 30]);

// traverse_state: Thread state through each element
let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
assert_eq!(result, vec![(0, "a"), (1, "b"), (2, "c")]);
assert_eq!(final_index, 3);

// traverse_io: Execute IO actions sequentially
let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
assert_eq!(contents, vec!["content of a.txt", "content of b.txt"]);
```

### Function Composition (`compose`)

Utilities for composing functions in a functional programming style.

#### compose! (Right-to-Left Composition)

```rust
use lambars::compose;

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// compose!(f, g)(x) = f(g(x))
let composed = compose!(add_one, double);
assert_eq!(composed(5), 11); // add_one(double(5)) = add_one(10) = 11
```

#### pipe! (Left-to-Right Composition)

```rust
use lambars::pipe;

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// pipe!(x, f, g) = g(f(x))
let result = pipe!(5, double, add_one);
assert_eq!(result, 11); // add_one(double(5)) = 11
```

##### Monadic Operators

`pipe!` supports monadic operations with special operators:

- `=>` (lift): Applies a pure function within a monadic context using `fmap`
- `=>>` (bind): Applies a monadic function using `flat_map`

```rust
use lambars::pipe;
use lambars::typeclass::{Functor, Monad};

// Lift operator: applies pure function within monad
let result = pipe!(Some(5), => |x| x * 2);
assert_eq!(result, Some(10));

// Bind operator: applies monadic function
let result = pipe!(
    Some(5),
    =>> |x| if x > 0 { Some(x * 2) } else { None }
);
assert_eq!(result, Some(10));

// Mixed operators: combine pure and monadic functions
let result = pipe!(
    Some(10),
    => |x| x / 2,                                    // lift: Some(5)
    =>> |x| if x > 0 { Some(x + 10) } else { None }, // bind: Some(15)
    => |x| x * 2                                     // lift: Some(30)
);
assert_eq!(result, Some(30));

// IO monad with pipe!
use lambars::effect::IO;

let io_result = pipe!(
    IO::pure(5),
    => |x| x + 1,           // lift: IO(6)
    =>> |x| IO::pure(x * 2) // bind: IO(12)
).run_unsafe();
assert_eq!(io_result, 12);
```

#### partial! (Partial Application)

```rust
use lambars::partial;

fn add(first: i32, second: i32) -> i32 { first + second }

// Use __ as a placeholder for remaining arguments
let add_five = partial!(add, 5, __);
assert_eq!(add_five(3), 8);
```

#### curry! (Currying)

Transforms multi-argument functions into chains of single-argument functions.

```rust
use lambars::curry;

fn add(first: i32, second: i32) -> i32 { first + second }

// Closure form: curry!(|args...| body)
let curried_add = curry!(|a, b| add(a, b));
let add_five = curried_add(5);
assert_eq!(add_five(3), 8);

// Function name + arity form: curry!(function_name, arity)
let curried_add = curry!(add, 2);
let add_ten = curried_add(10);
assert_eq!(add_ten(7), 17);

// Works with any number of arguments (2 or more)
fn sum_four(a: i32, b: i32, c: i32, d: i32) -> i32 { a + b + c + d }
let curried = curry!(sum_four, 4);
let step1 = curried(1);
let step2 = step1(2);
let step3 = step2(3);
assert_eq!(step3(4), 10);

// Partial application can be reused
let add_one = curried_add(1);
assert_eq!(add_one(5), 6);
assert_eq!(add_one(10), 11); // Reusable!
```

#### Helper Functions

```rust
use lambars::compose::{identity, constant, flip};

// identity: returns its argument unchanged
assert_eq!(identity(42), 42);

// constant: creates a function that always returns the same value
let always_five = constant(5);
assert_eq!(always_five(100), 5);

// flip: swaps the arguments of a binary function
let subtract = |a, b| a - b;
let flipped = flip(subtract);
assert_eq!(flipped(3, 10), 7); // 10 - 3 = 7
```

### Control Structures (`control`)

#### Lazy Evaluation

Defers computation until needed, with memoization.

```rust
use lambars::control::Lazy;

let lazy = Lazy::new(|| {
    println!("Computing...");
    42
});
// "Computing..." is not printed yet

let value = lazy.force();
// Now "Computing..." is printed and value is 42
assert_eq!(*value, 42);

// Second call uses cached value (no recomputation)
let value2 = lazy.force();
assert_eq!(*value2, 42);
```

#### Thread-Safe Lazy Evaluation

`ConcurrentLazy` provides thread-safe lazy evaluation that can be safely shared between threads.

```rust
use lambars::control::ConcurrentLazy;
use std::sync::Arc;
use std::thread;

let lazy = Arc::new(ConcurrentLazy::new(|| {
    println!("Computing...");
    42
}));

// Spawn multiple threads that access the lazy value
let handles: Vec<_> = (0..10).map(|_| {
    let lazy = Arc::clone(&lazy);
    thread::spawn(move || *lazy.force())
}).collect();

// All threads get the same value, and initialization happens only once
for handle in handles {
    assert_eq!(handle.join().unwrap(), 42);
}
```

#### Trampoline (Stack-Safe Recursion)

Enables deep recursion without stack overflow.

```rust
use lambars::control::Trampoline;

fn factorial(n: u64) -> Trampoline<u64> {
    factorial_helper(n, 1)
}

fn factorial_helper(n: u64, accumulator: u64) -> Trampoline<u64> {
    if n <= 1 {
        Trampoline::done(accumulator)
    } else {
        Trampoline::suspend(move || factorial_helper(n - 1, n * accumulator))
    }
}

// Works for very large n without stack overflow
let result = factorial(10).run();
assert_eq!(result, 3628800);

// Even 100,000 iterations work safely
let large_result = factorial(20).run();
assert_eq!(large_result, 2432902008176640000);
```

#### Continuation Monad

For advanced control flow patterns.

```rust
use lambars::control::Continuation;

let cont = Continuation::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| Continuation::pure(x + 1));

let result = cont.run(|x| x);
assert_eq!(result, 21);
```

#### Freer Monad

For building domain-specific languages (DSLs) with stack-safe interpretation.

```rust
use lambars::control::Freer;
use std::any::Any;

// Define instruction types for your DSL
#[derive(Debug)]
enum Console {
    ReadLine,
    PrintLine(String),
}

// Build computations using instructions
let program: Freer<Console, String> = Freer::lift_instruction(
    Console::PrintLine("Enter name:".to_string()),
    |_| (),
)
.then(Freer::lift_instruction(
    Console::ReadLine,
    |result: Box<dyn Any>| *result.downcast::<String>().unwrap(),
))
.map(|name| format!("Hello, {}!", name));

// Interpret with a handler
let result = program.interpret(|instruction| -> Box<dyn Any> {
    match instruction {
        Console::ReadLine => Box::new("Alice".to_string()),
        Console::PrintLine(msg) => {
            println!("{}", msg);
            Box::new(())
        }
    }
});
assert_eq!(result, "Hello, Alice!");
```

### Persistent Data Structures (`persistent`)

Immutable data structures with structural sharing for efficient updates.

#### PersistentList

Singly-linked list with O(1) prepend.

```rust
use lambars::persistent::PersistentList;

let list = PersistentList::new().cons(3).cons(2).cons(1);
assert_eq!(list.head(), Some(&1));

// Structural sharing: the original list is preserved
let extended = list.cons(0);
assert_eq!(list.len(), 3);     // Original unchanged
assert_eq!(extended.len(), 4); // New list
```

#### PersistentVector

Dynamic array with O(log32 N) random access and updates.

```rust
use lambars::persistent::PersistentVector;

let vector: PersistentVector<i32> = (0..100).collect();
assert_eq!(vector.get(50), Some(&50));

// Structural sharing preserves the original
let updated = vector.update(50, 999).unwrap();
assert_eq!(vector.get(50), Some(&50));     // Original unchanged
assert_eq!(updated.get(50), Some(&999));   // New version

// Push operations
let pushed = vector.push_back(100);
assert_eq!(pushed.len(), 101);
```

#### PersistentDeque

Double-ended queue (Finger Tree inspired) with O(1) front/back access.

```rust
use lambars::persistent::PersistentDeque;

let deque = PersistentDeque::new()
    .push_back(1)
    .push_back(2)
    .push_back(3);
assert_eq!(deque.front(), Some(&1));
assert_eq!(deque.back(), Some(&3));

// Structural sharing preserves the original
let extended = deque.push_back(4);
assert_eq!(deque.len(), 3);     // Original unchanged
assert_eq!(extended.len(), 4);  // New deque

// Pop from both ends
let (rest, first) = deque.pop_front().unwrap();
assert_eq!(first, 1);
```

#### PersistentHashMap

Hash map with O(log32 N) operations using HAMT (Hash Array Mapped Trie).

```rust
use lambars::persistent::PersistentHashMap;

let map = PersistentHashMap::new()
    .insert("one".to_string(), 1)
    .insert("two".to_string(), 2);
assert_eq!(map.get("one"), Some(&1));

// Structural sharing
let updated = map.insert("one".to_string(), 100);
assert_eq!(map.get("one"), Some(&1));       // Original unchanged
assert_eq!(updated.get("one"), Some(&100)); // New version

// Removal
let removed = map.remove("one");
assert_eq!(removed.get("one"), None);
```

#### PersistentHashSet

Hash set with set operations (union, intersection, difference).

```rust
use lambars::persistent::PersistentHashSet;

let set = PersistentHashSet::new()
    .insert(1)
    .insert(2)
    .insert(3);
assert!(set.contains(&1));

// Set operations
let other: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
let union = set.union(&other);
let intersection = set.intersection(&other);
let difference = set.difference(&other);

assert_eq!(union.len(), 4);        // {1, 2, 3, 4}
assert_eq!(intersection.len(), 2); // {2, 3}
assert_eq!(difference.len(), 1);   // {1}

// Lazy evaluation with HashSetView
let result: PersistentHashSet<i32> = set
    .view()
    .filter(|x| *x % 2 == 1)
    .map(|x| x * 10)
    .collect();
assert!(result.contains(&10));  // 1 * 10
assert!(result.contains(&30));  // 3 * 10
```

#### PersistentTreeMap

Ordered map with O(log N) operations using B-Tree.

```rust
use lambars::persistent::PersistentTreeMap;

let map = PersistentTreeMap::new()
    .insert(3, "three")
    .insert(1, "one")
    .insert(2, "two");

// Entries are always in sorted order
let keys: Vec<&i32> = map.keys().collect();
assert_eq!(keys, vec![&1, &2, &3]);

// Range queries
let range: Vec<(&i32, &&str)> = map.range(1..=2).collect();
assert_eq!(range.len(), 2); // 1 and 2

// Min/Max access
assert_eq!(map.min(), Some((&1, &"one")));
assert_eq!(map.max(), Some((&3, &"three")));
```

### Optics (`optics`)

Composable accessors for immutable data manipulation.

#### Lens

Focus on a single field with get/set operations.

```rust
use lambars::optics::{Lens, FunctionLens};
use lambars::lens;

#[derive(Clone, PartialEq, Debug)]
struct Address { street: String, city: String }

#[derive(Clone, PartialEq, Debug)]
struct Person { name: String, address: Address }

// Create lenses using the macro
let address_lens = lens!(Person, address);
let street_lens = lens!(Address, street);

// Compose lenses to focus on nested fields
let person_street = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        street: "Main St".to_string(),
        city: "Tokyo".to_string(),
    },
};

// Get nested field
assert_eq!(*person_street.get(&person), "Main St");

// Set nested field (returns new structure)
let updated = person_street.set(person, "Oak Ave".to_string());
assert_eq!(updated.address.street, "Oak Ave");
assert_eq!(updated.address.city, "Tokyo"); // Other fields unchanged
```

#### Prism

Focus on a variant of an enum.

```rust
use lambars::optics::{Prism, FunctionPrism};
use lambars::prism;

#[derive(Clone, PartialEq, Debug)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

let circle_prism = prism!(Shape, Circle);

let circle = Shape::Circle(5.0);
assert_eq!(circle_prism.preview(&circle), Some(&5.0));

let rect = Shape::Rectangle(3.0, 4.0);
assert_eq!(circle_prism.preview(&rect), None);

// Construct a value through the prism
let constructed = circle_prism.review(10.0);
assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
```

#### Iso

Bidirectional type conversions.

```rust
use lambars::optics::FunctionIso;

// String <-> Vec<char> isomorphism
let string_chars_iso = FunctionIso::new(
    |s: String| s.chars().collect::<Vec<_>>(),
    |chars: Vec<char>| chars.into_iter().collect::<String>(),
);

let original = "hello".to_string();
let chars = string_chars_iso.get(original.clone());
assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);

// Roundtrip
let back = string_chars_iso.reverse_get(chars);
assert_eq!(back, original);
```

#### Traversal

Focus on multiple elements.

```rust
use lambars::optics::{Traversal, VecTraversal};

let traversal = VecTraversal::<i32>::new();
let vec = vec![1, 2, 3, 4, 5];

// Get all elements
let all: Vec<&i32> = traversal.get_all(&vec);
assert_eq!(all, vec![&1, &2, &3, &4, &5]);

// Modify all elements
let doubled = traversal.modify(vec, |x| x * 2);
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
```

### Effect System (`effect`)

Type-safe side effect handling with monads and transformers.

#### IO Monad

Defers side effects until explicitly executed.

```rust
use lambars::effect::IO;

// Create and chain IO actions
let io = IO::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| IO::pure(x + 1));

// Side effects don't occur until run_unsafe is called
assert_eq!(io.run_unsafe(), 21);

// IO with actual side effects
let print_io = IO::print_line("Hello, World!");
print_io.run_unsafe(); // Prints "Hello, World!"
```

#### AsyncIO Monad

Async version of IO for integration with async runtimes like Tokio.

```rust
use lambars::effect::AsyncIO;

// Create and chain async IO actions
let async_io = AsyncIO::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| AsyncIO::pure(x + 1));

// Execute asynchronously
let result = async_io.run_async().await;
assert_eq!(result, 21);

// Convert sync IO to async
use lambars::effect::IO;
let sync_io = IO::pure(42);
let async_io = sync_io.to_async();
let result = async_io.run_async().await;
assert_eq!(result, 42);
```

#### eff_async! Macro

Do-notation for AsyncIO computations.

```rust
use lambars::eff_async;
use lambars::effect::AsyncIO;

async fn example() {
    let result = eff_async! {
        x <= AsyncIO::pure(5);
        y <= AsyncIO::pure(10);
        let z = x + y;
        AsyncIO::pure(z * 2)
    }.run_async().await;

    assert_eq!(result, 30);
}
```

#### Reader Monad

Computations that read from an environment.

```rust
use lambars::effect::Reader;

#[derive(Clone)]
struct Config { multiplier: i32 }

let computation = Reader::ask()
    .flat_map(|config: Config| Reader::pure(config.multiplier * 10));

let config = Config { multiplier: 5 };
let result = computation.run(config);
assert_eq!(result, 50);
```

#### State Monad

Computations with mutable state.

```rust
use lambars::effect::State;

let computation = State::get()
    .flat_map(|state: i32| {
        State::put(state + 10)
            .then(State::get())
    });

let (result, final_state) = computation.run(5);
assert_eq!(result, 15);
assert_eq!(final_state, 15);
```

#### Writer Monad

Computations that accumulate output.

```rust
use lambars::effect::Writer;

let computation = Writer::tell(vec!["Starting".to_string()])
    .then(Writer::pure(42))
    .flat_map(|x| {
        Writer::tell(vec![format!("Got {}", x)])
            .then(Writer::pure(x * 2))
    });

let (result, log) = computation.run();
assert_eq!(result, 84);
assert_eq!(log, vec!["Starting", "Got 42"]);
```

#### RWS Monad

Combined Reader + Writer + State monad for computations that need all three effects.

```rust
use lambars::effect::RWS;

#[derive(Clone)]
struct Config { multiplier: i32 }

// RWS combines environment reading, log accumulation, and state management
let computation: RWS<Config, Vec<String>, i32, i32> = RWS::ask()
    .flat_map(|config| {
        RWS::get().flat_map(move |state| {
            let result = state * config.multiplier;
            RWS::put(result)
                .then(RWS::tell(vec![format!("Multiplied {} by {}", state, config.multiplier)]))
                .then(RWS::pure(result))
        })
    });

let config = Config { multiplier: 3 };
let (result, final_state, log) = computation.run(config, 10);
assert_eq!(result, 30);
assert_eq!(final_state, 30);
assert_eq!(log, vec!["Multiplied 10 by 3"]);
```

#### MonadError

Error handling abstraction.

```rust
use lambars::effect::MonadError;

let computation: Result<i32, String> = Err("error".to_string());
let recovered = <Result<i32, String>>::catch_error(computation, |e| {
    Ok(e.len() as i32)
});
assert_eq!(recovered, Ok(5));
```

#### Algebraic Effects

Alternative to Monad Transformers that solves the n^2 problem.

```rust
use lambars::effect::algebraic::{
    Eff, Effect, Handler, ReaderEffect, ReaderHandler, StateEffect, StateHandler,
    WriterEffect, ErrorEffect, EffectRow, Member, Here, There,
};

// Define effects using the effect row
type MyEffects = EffectRow!(ReaderEffect<String>, StateEffect<i32>);

// Create computations with multiple effects
fn computation() -> Eff<MyEffects, i32> {
    use lambars::effect::algebraic::{ask, get, put};

    ask::<String, MyEffects, Here>()
        .flat_map(|env| {
            get::<i32, MyEffects, There<Here>>()
                .flat_map(move |state| {
                    put::<i32, MyEffects, There<Here>>(state + env.len() as i32)
                        .then(Eff::pure(state + 1))
                })
        })
}

// Run with handlers
let eff = computation();
let with_reader = ReaderHandler::new("hello".to_string()).run(eff);
let (result, final_state) = StateHandler::new(10).run(with_reader);
// result = 11, final_state = 15
```

**Key Features:**

- **No n^2 problem**: Adding a new effect doesn't require new lift implementations
- **Type-safe composition**: Effect rows track which effects are available
- **Stack-safe**: Deep `flat_map` chains don't overflow the stack
- **Standard effects**: Reader, State, Writer, Error
- **Custom effects**: Use `define_effect!` macro to define your own effects

```rust
use lambars::define_effect;
use lambars::effect::algebraic::{Effect, Eff};

// Define a custom logging effect
define_effect! {
    /// Custom logging effect
    effect Log {
        /// Log a message
        fn log(message: String) -> ();
    }
}

// The macro generates:
// - LogEffect struct implementing Effect
// - LogEffect::log(message) -> Eff<LogEffect, ()>
// - LogHandler trait with fn log(&mut self, message: String) -> ()

// Create a computation using the effect
fn log_computation() -> Eff<LogEffect, i32> {
    LogEffect::log("Hello".to_string())
        .then(LogEffect::log("World".to_string()))
        .then(Eff::pure(42))
}
```

#### Monad Transformers

Stack effects with transformers.

```rust
use lambars::effect::{ReaderT, StateT};

// ReaderT adds Reader capabilities to Option
let reader_t = ReaderT::<i32, Option<i32>>::ask_option()
    .flat_map_option(|env| ReaderT::pure_option(env * 2));
let result = reader_t.run_option(21);
assert_eq!(result, Some(42));

// StateT adds State capabilities to Result
let state_t = StateT::<i32, Result<i32, String>>::get_result()
    .flat_map_result(|s| StateT::pure_result(s * 2));
let (result, state) = state_t.run_result(10).unwrap();
assert_eq!(result, 20);
assert_eq!(state, 10);

// ReaderT with AsyncIO support (requires "async" feature)
use lambars::effect::AsyncIO;

async fn example() {
    let reader_t = ReaderT::<i32, AsyncIO<i32>>::ask_async_io()
        .flat_map_async_io(|env| ReaderT::pure_async_io(env * 2));
    let result = reader_t.run_async_io(21).run_async().await;
    assert_eq!(result, 42);
}
```

#### eff! Macro (Do-Notation)

Convenient syntax for monadic computations.

```rust
use lambars::eff;
use lambars::typeclass::Monad;

let result = eff! {
    x <= Some(5);
    y <= Some(10);
    let z = x + y;
    Some(z * 2)
};
assert_eq!(result, Some(30));

// Short-circuits on None
let result = eff! {
    x <= Some(5);
    y <= None::<i32>;
    Some(x + y)
};
assert_eq!(result, None);
```

#### for\_! Macro (List Comprehensions)

Scala/Haskell-style list comprehensions for Vec and iterators.

```rust
use lambars::for_;

// Basic list comprehension
let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);

// Nested comprehension (cartesian product)
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // Clone needed for inner iteration
    yield x + y
};
assert_eq!(cartesian, vec![11, 21, 12, 22]);

// With let bindings
let result: Vec<i32> = for_! {
    x <= vec![1, 2, 3];
    let doubled = x * 2;
    yield doubled + 1
};
assert_eq!(result, vec![3, 5, 7]);
```

#### for_async! Macro (Async List Comprehensions)

Async version of `for_!` for list comprehensions with async operations. Returns `AsyncIO<Vec<T>>` for lazy evaluation.

```rust
use lambars::for_async;
use lambars::effect::AsyncIO;

async fn example() {
    // Basic async list comprehension
    let urls = vec!["http://a.com", "http://b.com"];
    let result: AsyncIO<Vec<String>> = for_async! {
        url <= urls;
        yield url.to_uppercase()
    };
    let uppercase_urls = result.run_async().await;
    assert_eq!(uppercase_urls, vec!["HTTP://A.COM", "HTTP://B.COM"]);

    // With AsyncIO binding using <~ operator
    let result: AsyncIO<Vec<i32>> = for_async! {
        x <= vec![1, 2, 3];
        doubled <~ AsyncIO::pure(x * 2);  // <~ binds from AsyncIO
        yield doubled + 1
    };
    let values = result.run_async().await;
    assert_eq!(values, vec![3, 5, 7]);

    // Nested iteration with async
    let xs = vec![1, 2];
    let ys = vec![10, 20];
    let result: AsyncIO<Vec<i32>> = for_async! {
        x <= xs;
        y <= ys.clone();
        sum <~ AsyncIO::pure(x + y);
        yield sum
    };
    let cartesian = result.run_async().await;
    assert_eq!(cartesian, vec![11, 21, 12, 22]);
}
```

**Syntax:**

- `pattern <= collection;` - Bind from IntoIterator (for loop)
- `pattern <~ async_io;` - Bind from AsyncIO (await)
- `let pattern = expr;` - Pure let binding
- `yield expr` - Terminal expression (collected into Vec)

#### eff! vs for\_! vs for_async! : When to Use Which

| Scenario                | Macro        | Reason                      |
| ----------------------- | ------------ | --------------------------- |
| Option/Result chaining  | `eff!`       | Short-circuits on None/Err  |
| IO/State/Reader/Writer  | `eff!`       | FnOnce-based monads         |
| Vec/Iterator generation | `for_!`      | FnMut-based, uses yield     |
| Cartesian products      | `for_!`      | Multiple iterations         |
| Async monadic chaining  | `eff_async!` | Sequential async operations |
| Async list generation   | `for_async!` | Async iteration with yield  |

## Safety

This library is built with safety in mind:

- `#![forbid(unsafe_code)]` - No unsafe code
- `#![warn(clippy::all, clippy::pedantic, clippy::nursery)]` - Strict linting
- Comprehensive test coverage with property-based testing

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
