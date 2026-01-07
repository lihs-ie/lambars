# F# to lambars API Comparison Guide

This document provides a comprehensive comparison between F# functional programming constructs and their equivalents in lambars (Rust).

## Table of Contents

- [Overview](#overview)
- [Option Module](#option-module)
- [Result Module](#result-module)
- [List and Sequence Operations](#list-and-sequence-operations)
- [Function Composition](#function-composition)
- [Computation Expressions vs Effect System](#computation-expressions-vs-effect-system)
- [Active Patterns vs Optics](#active-patterns-vs-optics)
- [Lazy Evaluation](#lazy-evaluation)
- [Type Classes / Interfaces](#type-classes--interfaces)
- [Persistent Data Structures](#persistent-data-structures)

---

## Overview

| Concept | F# | lambars (Rust) |
|---------|-----|----------------|
| Option type | `Option<'T>` | `Option<T>` (std) |
| Result type | `Result<'T, 'E>` | `Result<T, E>` (std) |
| List type | `list<'T>` (immutable) | `PersistentList<T>` |
| Sequence | `seq<'T>` (lazy) | `Iterator` / `Lazy<T>` |
| Pipe operator | `\|>` | `pipe!` macro |
| Composition | `>>` | `compose!` macro |
| Computation expressions | `async { }`, `result { }` | `eff!` macro |
| List comprehension | `[ for ... ]`, `seq { }` | `for_!` macro |
| Async list comprehension | `async { for ... }` | `for_async!` macro |
| Active patterns | `(|Pattern|_|)` | `Prism` |
| Lenses | via libraries | `Lens`, `lens!` macro |
| Monoid | `+` operator overloading | `Semigroup`, `Monoid` traits |

---

## Option Module

### Basic Operations

| F# | lambars | Description |
|----|---------|-------------|
| `Option.map` | `Functor::fmap` | Transform the inner value |
| `Option.bind` | `Monad::flat_map` | Chain computations |
| `Option.filter` | `Option::filter` (std) | Filter based on predicate |
| `Option.defaultValue` | `Option::unwrap_or` (std) | Provide default |
| `Option.defaultWith` | `Option::unwrap_or_else` (std) | Lazy default |
| `Option.orElse` | `Option::or` (std) | Alternative option |
| `Option.orElseWith` | `Option::or_else` (std) | Lazy alternative |
| `Option.isSome` | `Option::is_some` (std) | Check if Some |
| `Option.isNone` | `Option::is_none` (std) | Check if None |
| `Option.iter` | `Option::iter` (std) | Iterate over value |
| `Option.toList` | `Option::into_iter().collect()` | Convert to list |
| `Option.flatten` | `Option::flatten` (std) | Flatten nested option |
| `Option.map2` | `Applicative::map2` | Combine two options |
| `Option.map3` | `Applicative::map3` | Combine three options |

### Code Examples

#### F# Option.map vs lambars Functor::fmap

```fsharp
// F#
let doubled = Some 21 |> Option.map (fun x -> x * 2)
// doubled = Some 42
```

```rust
// lambars
use lambars::typeclass::Functor;

let doubled = Some(21).fmap(|x| x * 2);
// doubled = Some(42)
```

#### F# Option.bind vs lambars Monad::flat_map

```fsharp
// F#
let safeDivide x y = if y = 0 then None else Some (x / y)
let result = Some 10 |> Option.bind (fun x -> safeDivide x 2)
// result = Some 5
```

```rust
// lambars
use lambars::typeclass::Monad;

fn safe_divide(x: i32, y: i32) -> Option<i32> {
    if y == 0 { None } else { Some(x / y) }
}

let result = Some(10).flat_map(|x| safe_divide(x, 2));
// result = Some(5)
```

#### F# Option.map2 vs lambars Applicative::map2

```fsharp
// F#
let sum = Option.map2 (+) (Some 10) (Some 20)
// sum = Some 30
```

```rust
// lambars
use lambars::typeclass::Applicative;

let sum = Some(10).map2(Some(20), |a, b| a + b);
// sum = Some(30)
```

#### F# Option Computation Expression vs lambars eff! Macro

```fsharp
// F#
let computation = option {
    let! x = Some 10
    let! y = Some 20
    return x + y
}
// computation = Some 30
```

```rust
// lambars
use lambars::eff;

let computation = eff! {
    x <= Some(10);
    y <= Some(20);
    Some(x + y)
};
// computation = Some(30)
```

---

## Result Module

### Basic Operations

| F# | lambars | Description |
|----|---------|-------------|
| `Result.map` | `Functor::fmap` | Transform Ok value |
| `Result.mapError` | `Result::map_err` (std) | Transform Error value |
| `Result.bind` | `Monad::flat_map` | Chain computations |
| `Result.isOk` | `Result::is_ok` (std) | Check if Ok |
| `Result.isError` | `Result::is_err` (std) | Check if Error |
| `Result.defaultValue` | `Result::unwrap_or` (std) | Default on error |
| `Result.defaultWith` | `Result::unwrap_or_else` (std) | Lazy default |
| `Result.toOption` | `Result::ok` (std) | Convert to Option |

### Error Handling

| F# | lambars | Description |
|----|---------|-------------|
| `try ... with` | `MonadError::catch_error` | Catch and handle errors |
| `raise` / `failwith` | `MonadError::throw_error` | Throw an error |
| `Result.mapError` | `ExceptT::map_error` | Transform error type |

### Code Examples

#### F# Result.bind vs lambars Monad::flat_map

```fsharp
// F#
let parseInt s =
    match System.Int32.TryParse s with
    | true, n -> Ok n
    | false, _ -> Error "Invalid number"

let result = Ok "42" |> Result.bind parseInt
// result = Ok 42
```

```rust
// lambars
use lambars::typeclass::Monad;

fn parse_int(s: &str) -> Result<i32, String> {
    s.parse().map_err(|_| "Invalid number".to_string())
}

let result: Result<i32, String> = Ok("42".to_string())
    .flat_map(|s| parse_int(&s));
// result = Ok(42)
```

#### F# Error Handling vs lambars MonadError

```fsharp
// F#
let handleError result =
    match result with
    | Ok x -> Ok x
    | Error e -> Ok (String.length e)

let recovered = Error "error" |> handleError
// recovered = Ok 5
```

```rust
// lambars
use lambars::effect::MonadError;

let failing: Result<i32, String> = Err("error".to_string());
let recovered = <Result<i32, String>>::catch_error(failing, |e| {
    Ok(e.len() as i32)
});
// recovered = Ok(5)
```

---

## List and Sequence Operations

### Collection Operations

| F# | lambars | Description |
|----|---------|-------------|
| `List.map` | `Functor::fmap` / `FunctorMut::fmap_mut` | Transform elements |
| `List.collect` | `Monad::flat_map` + `flatten` | Map and flatten |
| `List.filter` | `Iterator::filter` (std) | Filter elements |
| `List.fold` | `Foldable::fold_left` | Left fold |
| `List.foldBack` | `Foldable::fold_right` | Right fold |
| `List.reduce` | `Iterator::reduce` (std) | Reduce without initial |
| `List.sum` | `Foldable::fold_left` + `Monoid` | Sum elements |
| `List.length` | `Foldable::length` | Count elements |
| `List.isEmpty` | `Foldable::is_empty` | Check if empty |
| `List.head` | `PersistentList::head` | First element |
| `List.tail` | `PersistentList::tail` | Rest of list |
| `List.cons` | `PersistentList::cons` | Prepend element |
| `List.append` | `Semigroup::combine` | Concatenate lists |
| `List.rev` | `PersistentList::reverse` | Reverse list |
| `List.exists` | `Foldable::exists` | Any element matches |
| `List.forall` | `Foldable::for_all` | All elements match |
| `List.find` | `Foldable::find` | Find first matching |
| `List.tryFind` | `Foldable::find` | Find (returns Option) |
| `List.choose` | `Iterator::filter_map` (std) | Filter and map |
| `List.zip` | `PersistentList::zip` | Zip two lists |
| `List.unzip` | `PersistentList::<(A,B)>::unzip` | Unzip list of pairs |
| `List.take` | `PersistentList::take` | Take first n elements |
| `List.skip` | `PersistentList::drop_first` | Skip first n elements |
| `List.splitAt` | `PersistentList::split_at` | Split at index |
| `List.findIndex` | `PersistentList::find_index` | Find index of first match |
| `List.reduce` | `PersistentList::fold_left1` | Left fold without initial value |
| `List.reduceBack` | `PersistentList::fold_right1` | Right fold without initial value |
| `List.scan` | `PersistentList::scan_left` | Left scan with initial value |
| `List.partition` | `PersistentList::partition` | Split by predicate |
| (N/A) | `PersistentList::intersperse` | Insert between elements |
| `String.concat sep` | `PersistentList::intercalate` | Insert list between lists and flatten |
| `Seq.unfold` | Manual implementation | Generate sequence |

### Traversable Operations

| F# | lambars | Description |
|----|---------|-------------|
| `List.traverse` (custom) | `Traversable::traverse_option/result` | Traverse with Option/Result |
| `List.sequence` (custom) | `Traversable::sequence_option/result` | Sequence Option/Result effects |
| `List.traverse` with Reader | `Traversable::traverse_reader` | Traverse with Reader effect |
| `List.traverse` with State | `Traversable::traverse_state` | Traverse with State effect |
| `List.traverse` with Async | `Traversable::traverse_io` | Traverse with IO effect |
| `List.sequence` with Reader | `Traversable::sequence_reader` | Sequence Reader effects |
| `List.sequence` with State | `Traversable::sequence_state` | Sequence State effects |
| `List.sequence` with Async | `Traversable::sequence_io` | Sequence IO effects |
| `List.iter` with Reader | `Traversable::for_each_reader` | For-each with Reader effect |
| `List.iter` with State | `Traversable::for_each_state` | For-each with State effect |
| `List.iter` with IO | `Traversable::for_each_io` | For-each with IO effect |

### Code Examples

#### F# List.map vs lambars Functor::fmap

```fsharp
// F#
let doubled = [1; 2; 3] |> List.map (fun x -> x * 2)
// doubled = [2; 4; 6]
```

```rust
// lambars
use lambars::typeclass::Functor;

let doubled: Vec<i32> = vec![1, 2, 3].fmap(|x| x * 2);
// doubled = vec![2, 4, 6]
```

#### F# List.fold vs lambars Foldable::fold_left

```fsharp
// F#
let sum = [1; 2; 3; 4; 5] |> List.fold (+) 0
// sum = 15
```

```rust
// lambars
use lambars::typeclass::Foldable;

let sum = vec![1, 2, 3, 4, 5].fold_left(0, |acc, x| acc + x);
// sum = 15
```

#### F# List.collect vs lambars with Iterator

```fsharp
// F#
let duplicated = [1; 2; 3] |> List.collect (fun x -> [x; x])
// duplicated = [1; 1; 2; 2; 3; 3]
```

```rust
// lambars / std
let duplicated: Vec<i32> = vec![1, 2, 3]
    .into_iter()
    .flat_map(|x| vec![x, x])
    .collect();
// duplicated = vec![1, 1, 2, 2, 3, 3]
```

#### F# List.filter vs std Iterator::filter

```fsharp
// F#
let evens = [1; 2; 3; 4; 5] |> List.filter (fun x -> x % 2 = 0)
// evens = [2; 4]
```

```rust
// std Rust
let evens: Vec<i32> = vec![1, 2, 3, 4, 5]
    .into_iter()
    .filter(|x| x % 2 == 0)
    .collect();
// evens = vec![2, 4]
```

#### F# List.choose vs std Iterator::filter_map

```fsharp
// F#
let parseIfEven s =
    match System.Int32.TryParse s with
    | true, n when n % 2 = 0 -> Some n
    | _ -> None

let evens = ["1"; "2"; "three"; "4"] |> List.choose parseIfEven
// evens = [2; 4]
```

```rust
// std Rust
fn parse_if_even(s: &str) -> Option<i32> {
    s.parse::<i32>().ok().filter(|n| n % 2 == 0)
}

let evens: Vec<i32> = vec!["1", "2", "three", "4"]
    .into_iter()
    .filter_map(parse_if_even)
    .collect();
// evens = vec![2, 4]
```

#### F# List.exists / List.forall vs lambars Foldable

```fsharp
// F#
let hasEven = [1; 2; 3] |> List.exists (fun x -> x % 2 = 0)
// hasEven = true

let allPositive = [1; 2; 3] |> List.forall (fun x -> x > 0)
// allPositive = true
```

```rust
// lambars
use lambars::typeclass::Foldable;

let has_even = vec![1, 2, 3].exists(|x| x % 2 == 0);
// has_even = true

let all_positive = vec![1, 2, 3].for_all(|x| *x > 0);
// all_positive = true
```

#### Traversable with Effect Types

```fsharp
// F# - Custom traverse with Reader-like pattern
type Config = { Multiplier: int }

let traverseReader (f: 'a -> Config -> 'b) (items: 'a list) : Config -> 'b list =
    fun config -> items |> List.map (fun x -> f x config)

let multiply x config = x * config.Multiplier
let result = traverseReader multiply [1; 2; 3] { Multiplier = 10 }
// result = [10; 20; 30]
```

```rust
// lambars
use lambars::typeclass::Traversable;
use lambars::effect::{Reader, State, IO};

// traverse_reader - traverse with Reader effect
#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
// result = vec![10, 20, 30]

// traverse_state - traverse with State effect (state is threaded left-to-right)
let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
// result = vec![(0, "a"), (1, "b"), (2, "c")]
// final_index = 3

// traverse_io - traverse with IO effect (IO actions executed sequentially)
let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
// contents = vec!["content of a.txt", "content of b.txt"]
```

---

## Function Composition

### Operators and Macros

| F# | lambars | Description |
|----|---------|-------------|
| `\|>` (pipe forward) | `pipe!` | Apply value to function |
| `<\|` (pipe backward) | Function call | Apply function to value |
| `>>` (compose forward) | `compose!` (reversed) | Compose left-to-right |
| `<<` (compose backward) | `compose!` | Compose right-to-left |
| Partial application | `partial!` | Fix some arguments |
| Currying (automatic) | `curry!(fn, arity)` or `curry!(\|args...\| body)` | Convert to curried form |

### Code Examples

#### F# Pipe Operator vs lambars pipe! Macro

```fsharp
// F#
let result = 5 |> double |> addOne |> square
// Equivalent to: square(addOne(double(5)))
```

```rust
// lambars
use lambars::pipe;

fn double(x: i32) -> i32 { x * 2 }
fn add_one(x: i32) -> i32 { x + 1 }
fn square(x: i32) -> i32 { x * x }

let result = pipe!(5, double, add_one, square);
// result = 121 (square(add_one(double(5))))
```

#### F# Composition vs lambars compose! Macro

```fsharp
// F# forward composition (>>)
let transform = double >> addOne >> square
let result = transform 5  // 121

// F# backward composition (<<)
let transform2 = square << addOne << double
let result2 = transform2 5  // 121
```

```rust
// lambars
use lambars::compose;

fn double(x: i32) -> i32 { x * 2 }
fn add_one(x: i32) -> i32 { x + 1 }
fn square(x: i32) -> i32 { x * x }

// compose! uses mathematical (right-to-left) composition
// so this is equivalent to F#'s << operator
let transform = compose!(square, add_one, double);
let result = transform(5);  // 121
```

#### F# Partial Application vs lambars partial! Macro

```fsharp
// F#
let add a b = a + b
let addFive = add 5
let result = addFive 3  // 8
```

```rust
// lambars
use lambars::partial;

fn add(a: i32, b: i32) -> i32 { a + b }

// Use __ as placeholder for remaining arguments
let add_five = partial!(add, 5, __);
let result = add_five(3);  // 8
```

#### F# Automatic Currying vs lambars curry! Macro

```fsharp
// F# - functions are curried by default
let add a b c = a + b + c
let addFive = add 5
let addFiveAndTen = addFive 10
let result = addFiveAndTen 3  // 18
```

```rust
// lambars
use lambars::curry;

fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }

// Using function name + arity form
let curried = curry!(add, 3);
let add_five = curried(5);
let add_five_and_ten = add_five(10);
let result = add_five_and_ten(3);  // 18

// Or using closure form
let curried = curry!(|a, b, c| add(a, b, c));
let add_five = curried(5);
let add_five_and_ten = add_five(10);
let result = add_five_and_ten(3);  // 18
```

#### Helper Functions

```fsharp
// F#
let identity x = x
let constant x _ = x
let flip f a b = f b a
```

```rust
// lambars
use lambars::compose::{identity, constant, flip};

let x = identity(42);  // 42

let always_five = constant(5);
let result = always_five("ignored");  // 5

let subtract = |a: i32, b: i32| a - b;
let flipped = flip(subtract);
let result = flipped(3, 10);  // 7 (10 - 3)
```

---

## Computation Expressions vs Effect System

### Comparison Overview

| F# | lambars | Description |
|----|---------|-------------|
| `async { }` | `AsyncIO` + `eff_async!` | Async computations |
| `result { }` | `eff!` with Result | Result-based computations |
| `option { }` | `eff!` with Option | Option-based computations |
| `seq { }` / `[ for ... ]` | `for_!` macro | List/sequence generation |
| `state { }` | `State` monad | Stateful computations |
| `reader { }` | `Reader` monad | Environment reading |
| `writer { }` | `Writer` monad | Logging computations |
| `rws { }` (custom) | `RWS` monad | Combined Reader + Writer + State |

### Code Examples

#### F# async vs lambars AsyncIO

```fsharp
// F#
let asyncComputation = async {
    let! x = async { return 10 }
    let! y = async { return 20 }
    return x + y
}
let result = asyncComputation |> Async.RunSynchronously
// result = 30
```

```rust
// lambars (requires "async" feature)
use lambars::effect::AsyncIO;
use lambars::eff_async;

async fn computation() -> i32 {
    let io = eff_async! {
        x <= AsyncIO::pure(10);
        y <= AsyncIO::pure(20);
        AsyncIO::pure(x + y)
    };
    io.run_async().await
}
// result = 30
```

#### F# result Expression vs lambars eff! with Result

```fsharp
// F#
let resultComputation = result {
    let! x = Ok 10
    let! y = Ok 20
    return x + y
}
// resultComputation = Ok 30
```

```rust
// lambars
use lambars::eff;

let result_computation: Result<i32, String> = eff! {
    x <= Ok::<i32, String>(10);
    y <= Ok::<i32, String>(20);
    Ok(x + y)
};
// result_computation = Ok(30)
```

#### F# State Monad Pattern vs lambars State

```fsharp
// F# (using a state monad library or manual threading)
let increment state = (state, state + 1)
let getAndIncrement =
    fun state ->
        let (_, s1) = increment state
        let (_, s2) = increment s1
        (s2, s2)
```

```rust
// lambars
use lambars::effect::State;

let computation = State::get()
    .flat_map(|s: i32| State::put(s + 1).then(State::get()))
    .flat_map(|s: i32| State::put(s + 1).then(State::get()));

let (result, final_state) = computation.run(0);
// result = 2, final_state = 2
```

#### F# Reader Pattern vs lambars Reader

```fsharp
// F# (using reader pattern)
type Config = { Multiplier: int }

let multiply x = fun (config: Config) -> x * config.Multiplier

let computation =
    fun config ->
        let x = multiply 10 config
        let y = multiply 20 config
        x + y

let result = computation { Multiplier = 2 }
// result = 60
```

```rust
// lambars
use lambars::effect::Reader;

#[derive(Clone)]
struct Config { multiplier: i32 }

let computation = Reader::ask()
    .flat_map(|config: Config| {
        let m = config.multiplier;
        Reader::pure(10 * m + 20 * m)
    });

let result = computation.run(Config { multiplier: 2 });
// result = 60
```

#### F# RWS Pattern vs lambars RWS Monad

F# doesn't have a built-in RWS computation expression, but the pattern can be implemented manually. lambars provides a dedicated `RWS` monad that combines Reader, Writer, and State functionality.

```fsharp
// F# - Manual RWS pattern (Reader + Writer + State combined)
type Config = { Multiplier: int }
type Log = string list

// RWS-like function: Config -> State -> (Result, State, Log)
let rwsComputation config state =
    let result = state * config.Multiplier
    let newState = result
    let log = [sprintf "Multiplied %d by %d" state config.Multiplier]
    (result, newState, log)

// Usage
let config = { Multiplier = 2 }
let initialState = 10
let (result, finalState, log) = rwsComputation config initialState
// result = 20, finalState = 20, log = ["Multiplied 10 by 2"]
```

```rust
// lambars
use lambars::effect::RWS;

#[derive(Clone)]
struct Config { multiplier: i32 }

let computation: RWS<Config, Vec<String>, i32, i32> = RWS::ask()
    .flat_map(|config| {
        RWS::get().flat_map(move |state| {
            let result = state * config.multiplier;
            RWS::put(result)
                .then(RWS::tell(vec![format!("Multiplied {} by {}", state, config.multiplier)]))
                .then(RWS::pure(result))
        })
    });

let (result, final_state, log) = computation.run(Config { multiplier: 2 }, 10);
// result = 20, final_state = 20, log = vec!["Multiplied 10 by 2"]
```

The `RWS` monad provides the following operations:

| F# Pattern | lambars | Description |
|-----------|---------|-------------|
| `fun config -> ...` | `RWS::ask` | Access the environment |
| `fun config -> f config` | `RWS::asks` | Access derived value from environment |
| `fun _ state -> (state, state, [])` | `RWS::get` | Get current state |
| `fun _ _ -> ((), newState, [])` | `RWS::put` | Set new state |
| `fun _ state -> ((), f state, [])` | `RWS::modify` | Modify state with function |
| `fun _ state -> (f state, state, [])` | `RWS::gets` | Get derived value from state |
| `fun _ state -> ((), state, log)` | `RWS::tell` | Append to log output |
| N/A | `RWS::listen` | Access log within computation |
| N/A | `RWS::listens` | Access transformed log |
| N/A | `RWS::local` | Run with modified environment |

#### F# Sequence Expressions / List Comprehensions vs lambars for_!

```fsharp
// F# - List comprehension
let doubled = [ for x in [1; 2; 3; 4; 5] -> x * 2 ]
// doubled = [2; 4; 6; 8; 10]

// Nested comprehension
let cartesian = [ for x in [1; 2] do
                  for y in [10; 20] -> x + y ]
// cartesian = [11; 21; 12; 22]

// Sequence expression
let doubledSeq = seq {
    for x in [1; 2; 3; 4; 5] -> x * 2
}
// Lazy sequence that yields 2, 4, 6, 8, 10

// With filtering
let evens = [ for x in [1..10] do if x % 2 = 0 then yield x ]
// evens = [2; 4; 6; 8; 10]
```

```rust
// lambars - for_! macro
use lambars::for_;

let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// Nested comprehension
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // Note: clone() needed for inner iteration
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// With filtering (use std iterator methods)
let evens: Vec<i32> = (1..=10).filter(|x| x % 2 == 0).collect();
// evens = vec![2, 4, 6, 8, 10]
```

### When to Use eff! vs for_!

| Scenario | Recommended Macro | Reason |
|----------|-------------------|--------|
| `option { }` / `result { }` | `eff!` | Monadic chaining with short-circuit |
| `async { }` | `eff_async!` | Async monadic chaining |
| `[ for ... ]` / `seq { }` | `for_!` | List generation with yield |
| Cartesian products | `for_!` | Multiple iterations |
| State/Reader/Writer | `eff!` | Monadic effect chaining |
| State/Reader/Writer + Async | `*_async_io` methods | Transformer async integration |

### Monad Transformers with AsyncIO

lambars supports AsyncIO integration with monad transformers, similar to F#'s async workflows combined with reader/state patterns.

```rust
// lambars - ReaderT with AsyncIO
use lambars::effect::{ReaderT, AsyncIO};

#[derive(Clone)]
struct Config { api_url: String }

type AppAsync<A> = ReaderT<Config, AsyncIO<A>>;

fn get_api_url() -> AppAsync<String> {
    ReaderT::asks_async_io(|c: &Config| c.api_url.clone())
}

async fn example() {
    let computation = get_api_url()
        .flat_map_async_io(|url| ReaderT::pure_async_io(format!("Fetching: {}", url)));

    let config = Config { api_url: "https://api.example.com".to_string() };
    let result = computation.run_async_io(config).run_async().await;
    // result = "Fetching: https://api.example.com"
}
```

Available AsyncIO methods for transformers:
- `ReaderT`: `ask_async_io`, `asks_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `StateT`: `get_async_io`, `gets_async_io`, `state_async_io`, `lift_async_io`, `pure_async_io`
- `WriterT`: `tell_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`, `listen_async_io`

---

## Active Patterns vs Optics

### Comparison

| F# | lambars | Description |
|----|---------|-------------|
| Active Pattern (complete) | `Prism` | Match enum variants |
| Active Pattern (partial) | `Optional` | May or may not match |
| Record field access | `Lens` | Get/set field |
| Nested access | Composed optics | Deep access |

### Code Examples

#### F# Active Pattern vs lambars Prism

```fsharp
// F# - Active Pattern for parsing
let (|Integer|_|) (s: string) =
    match System.Int32.TryParse s with
    | true, n -> Some n
    | false, _ -> None

let describe s =
    match s with
    | Integer n -> sprintf "Number: %d" n
    | _ -> "Not a number"

let result = describe "42"  // "Number: 42"
```

```rust
// lambars - Using Prism for enum variants
use lambars::optics::{Prism, FunctionPrism};
use lambars::prism;

enum Parsed {
    Integer(i32),
    Text(String),
}

fn parse(s: &str) -> Parsed {
    match s.parse::<i32>() {
        Ok(n) => Parsed::Integer(n),
        Err(_) => Parsed::Text(s.to_string()),
    }
}

let integer_prism = prism!(Parsed, Integer);

let parsed = parse("42");
match integer_prism.preview(&parsed) {
    Some(n) => println!("Number: {}", n),
    None => println!("Not a number"),
}
```

#### F# Record Update vs lambars Lens

```fsharp
// F# - Record with copy-and-update
type Person = { Name: string; Age: int }

let person = { Name = "Alice"; Age = 30 }
let older = { person with Age = person.Age + 1 }
// older = { Name = "Alice"; Age = 31 }
```

```rust
// lambars - Using Lens
use lambars::optics::Lens;
use lambars::lens;

#[derive(Clone)]
struct Person { name: String, age: i32 }

let age_lens = lens!(Person, age);

let person = Person { name: "Alice".to_string(), age: 30 };
let older = age_lens.modify(person, |a| a + 1);
// older.age = 31
```

#### F# Nested Record Update vs lambars Composed Lens

```fsharp
// F#
type Address = { City: string; Street: string }
type Person = { Name: string; Address: Address }

let person = {
    Name = "Alice"
    Address = { City = "Tokyo"; Street = "Main St" }
}

let updated = { person with
    Address = { person.Address with Street = "Oak Ave" }
}
```

```rust
// lambars
use lambars::optics::Lens;
use lambars::lens;

#[derive(Clone)]
struct Address { city: String, street: String }

#[derive(Clone)]
struct Person { name: String, address: Address }

let address_lens = lens!(Person, address);
let street_lens = lens!(Address, street);
let person_street = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        city: "Tokyo".to_string(),
        street: "Main St".to_string(),
    },
};

let updated = person_street.set(person, "Oak Ave".to_string());
// updated.address.street = "Oak Ave"
```

---

## Lazy Evaluation

### Comparison

| F# | lambars | Description |
|----|---------|-------------|
| `lazy { expr }` | `Lazy::new(\|\| expr)` | Deferred computation |
| `Lazy.Force` | `Lazy::force` | Force evaluation |
| `Lazy.Value` | `Lazy::force` | Access value |
| `seq { }` | `Iterator` | Lazy sequence |

### Code Examples

#### F# lazy vs lambars Lazy

```fsharp
// F#
let lazyValue = lazy (
    printfn "Computing..."
    42
)
// Nothing printed yet

let result = lazyValue.Force()
// "Computing..." is printed, result = 42

let result2 = lazyValue.Force()
// Nothing printed (cached), result2 = 42
```

```rust
// lambars
use lambars::control::Lazy;

let lazy_value = Lazy::new(|| {
    println!("Computing...");
    42
});
// Nothing printed yet

let result = lazy_value.force();
// "Computing..." is printed, result = 42

let result2 = lazy_value.force();
// Nothing printed (cached), result2 = 42
```

#### F# Lazy Sequence vs Rust Iterator

```fsharp
// F#
let infiniteSeq = Seq.initInfinite id
let firstTen = infiniteSeq |> Seq.take 10 |> Seq.toList
// firstTen = [0; 1; 2; 3; 4; 5; 6; 7; 8; 9]
```

```rust
// Rust (std)
let first_ten: Vec<i32> = (0..).take(10).collect();
// first_ten = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
```

---

## Type Classes / Interfaces

### Comparison

| F# Concept | lambars Trait | Description |
|------------|---------------|-------------|
| `IComparable<'T>` | `Ord` (std) | Ordered comparison |
| `IEquatable<'T>` | `Eq` (std) | Equality |
| Interface with `+` | `Semigroup` | Associative combination |
| Interface with `Zero` | `Monoid` | Identity element |
| `IEnumerable<'T>` | `IntoIterator` (std) | Iteration |

### Code Examples

#### F# Monoid-like Pattern vs lambars Monoid

```fsharp
// F# - Using operator overloading
type Sum = Sum of int with
    static member (+) (Sum a, Sum b) = Sum (a + b)
    static member Zero = Sum 0

let combine (items: Sum list) = List.fold (+) Sum.Zero items
let result = combine [Sum 1; Sum 2; Sum 3]
// result = Sum 6
```

```rust
// lambars
use lambars::typeclass::{Semigroup, Monoid, Sum};

let items = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
let result = Sum::combine_all(items);
// result = Sum(6)
```

#### F# Generic Constraints vs lambars Trait Bounds

```fsharp
// F# - Using static member constraints
let inline add< ^T when ^T: (static member (+): ^T * ^T -> ^T)> (a: ^T) (b: ^T) =
    a + b
```

```rust
// lambars
use lambars::typeclass::Semigroup;

fn add<T: Semigroup>(a: T, b: T) -> T {
    a.combine(b)
}
```

---

## Persistent Data Structures

### Comparison

| F# Type | lambars Type | Description |
|---------|--------------|-------------|
| `list<'T>` | `PersistentList<T>` | Immutable singly-linked list |
| `Map<'K, 'V>` | `PersistentTreeMap<K, V>` | Immutable ordered map |
| `Set<'T>` | `PersistentHashSet<T>` | Immutable set |
| - | `PersistentVector<T>` | Immutable vector (Clojure-style) |
| - | `PersistentHashMap<K, V>` | Immutable hash map (HAMT) |

### Map Operations

| F# | lambars | Description |
|---------|---------|-------------|
| `Map.map f m` | `map_values` method | Transform values |
| `Map.map f m` (key in f) | `map_values` method | Transform values (key available in closure) |
| `Map.toSeq m` | `entries` method | Get all entries |
| `Map.keys m` | `keys` method | Get all keys |
| `Map.values m` | `values` method | Get all values |
| `Map.fold f m1 m2` | `merge` method | Merge (right wins) |
| - | `merge_with` method | Merge with custom resolver |
| `Map.filter p m` | `keep_if` method | Keep matching entries |
| - | `delete_if` method | Remove matching entries |
| `Map.partition p m` | `partition` method | Split by predicate |
| `Map.pick f m` | `filter_map` method | Filter and transform |

### Code Examples

#### F# List vs lambars PersistentList

```fsharp
// F#
let list = [1; 2; 3]
let extended = 0 :: list
// list = [1; 2; 3] (unchanged)
// extended = [0; 1; 2; 3]

let head = List.head list  // 1
let tail = List.tail list  // [2; 3]
```

```rust
// lambars
use lambars::persistent::PersistentList;

let list = PersistentList::new().cons(3).cons(2).cons(1);
let extended = list.cons(0);
// list.len() = 3 (unchanged)
// extended.len() = 4

let head = list.head();  // Some(&1)
let tail = list.tail();  // Some(PersistentList [2, 3])
```

#### F# Map vs lambars PersistentTreeMap

```fsharp
// F#
let map = Map.empty |> Map.add 1 "one" |> Map.add 2 "two"
let updated = map |> Map.add 1 "ONE"
// map.[1] = "one" (unchanged)
// updated.[1] = "ONE"

let keys = map |> Map.toSeq |> Seq.map fst |> Seq.toList
// keys = [1; 2] (sorted)
```

```rust
// lambars
use lambars::persistent::PersistentTreeMap;

let map = PersistentTreeMap::new()
    .insert(1, "one")
    .insert(2, "two");
let updated = map.insert(1, "ONE");
// map.get(&1) = Some(&"one") (unchanged)
// updated.get(&1) = Some(&"ONE")

let keys: Vec<&i32> = map.keys().collect();
// keys = vec![&1, &2] (sorted)
```

#### F# Set vs lambars PersistentHashSet

```fsharp
// F#
let set1 = Set.ofList [1; 2; 3]
let set2 = Set.ofList [2; 3; 4]

let union = Set.union set1 set2        // {1; 2; 3; 4}
let intersection = Set.intersect set1 set2  // {2; 3}
let difference = Set.difference set1 set2   // {1}
```

```rust
// lambars
use lambars::persistent::PersistentHashSet;

let set1: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
let set2: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();

let union = set1.union(&set2);           // {1, 2, 3, 4}
let intersection = set1.intersection(&set2);  // {2, 3}
let difference = set1.difference(&set2);      // {1}
```

---

## Summary: Key Differences

### Syntax Differences

| Aspect | F# | lambars (Rust) |
|--------|-----|----------------|
| Function application | `f x y` | `f(x, y)` |
| Pipe syntax | `x \|> f` | `pipe!(x, f)` |
| Composition | `f >> g` | `compose!(g, f)` |
| Let binding in CE | `let! x = m` | `x <= m;` |
| Lambda | `fun x -> x + 1` | `\|x\| x + 1` |
| Type annotation | `x: int` | `x: i32` |
| Generic type | `'T` | `T` |
| Option | `Some x` / `None` | `Some(x)` / `None` |
| Result | `Ok x` / `Error e` | `Ok(x)` / `Err(e)` |

### Conceptual Differences

1. **Currying**: F# functions are curried by default; Rust requires explicit `curry!` macros.

2. **Type Inference**: F# uses Hindley-Milner with more aggressive inference; Rust often requires explicit type annotations.

3. **Mutability**: Both default to immutable, but Rust's ownership model adds complexity.

4. **Higher-Kinded Types**: F# doesn't have HKT either, but interfaces work differently; lambars uses GAT for HKT emulation.

5. **Computation Expressions**: F#'s CEs are more flexible; lambars's `eff!` is more limited but covers common cases.

6. **Active Patterns**: F# active patterns are more powerful; lambars uses Prism/Optional for similar functionality.

---

## Migration Tips

1. **Replace `|>` with `pipe!`**: Direct translation, but remember to import the macro.

2. **Replace `>>` with `compose!`**: Note that `compose!` uses right-to-left order (like `<<`).

3. **Replace `option { }` / `result { }` with `eff!`**: Use `<=` instead of `let!`.

4. **Replace F# list with `PersistentList` or `Vec`**: Use `PersistentList` for functional patterns, `Vec` for performance.

5. **Use `Functor::fmap` instead of `Option.map` / `List.map`**: Unified interface across types.

6. **Use `Monad::flat_map` instead of `Option.bind` / `Result.bind`**: Same behavior, different name.

7. **Add explicit type annotations**: Rust's type inference is less aggressive than F#'s.
