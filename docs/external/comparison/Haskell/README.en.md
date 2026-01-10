# Haskell to lambars API Comparison Guide

[日本語](README.ja.md)

This document provides a comprehensive comparison between Haskell functional programming constructs and their equivalents in lambars (Rust). Haskell is the canonical pure functional programming language, and lambars aims to bring many of its abstractions to Rust.

## Table of Contents

- [Overview](#overview)
- [Type Classes](#type-classes)
  - [Functor](#functor)
  - [Applicative](#applicative)
  - [Monad](#monad)
  - [Semigroup and Monoid](#semigroup-and-monoid)
  - [Foldable](#foldable)
  - [Traversable](#traversable)
- [Maybe and Either](#maybe-and-either)
- [Do-Notation and List Comprehensions](#do-notation-and-list-comprehensions)
- [Function Composition](#function-composition)
- [Currying and Partial Application](#currying-and-partial-application)
- [Lazy Evaluation](#lazy-evaluation)
- [Optics (lens)](#optics-lens)
- [Effect Monads](#effect-monads)
  - [IO Monad](#io-monad)
  - [State Monad](#state-monad)
  - [Reader Monad](#reader-monad)
  - [Writer Monad](#writer-monad)
  - [RWS Monad](#rws-monad)
- [Monad Transformers (mtl)](#monad-transformers-mtl)
- [Algebraic Effects](#algebraic-effects)
- [Data Structures](#data-structures)
- [Pattern Matching](#pattern-matching)
- [Higher-Kinded Types](#higher-kinded-types)
- [Algebraic Data Types](#algebraic-data-types)

---

## Overview

| Concept                  | Haskell                          | lambars (Rust)                             |
| ------------------------ | -------------------------------- | ------------------------------------------ | ------------- |
| Functor                  | `Functor f`                      | `Functor` trait                            |
| Applicative              | `Applicative f`                  | `Applicative` trait                        |
| Monad                    | `Monad m`                        | `Monad` trait                              |
| Semigroup                | `Semigroup a`                    | `Semigroup` trait                          |
| Monoid                   | `Monoid a`                       | `Monoid` trait                             |
| Foldable                 | `Foldable t`                     | `Foldable` trait                           |
| Traversable              | `Traversable t`                  | `Traversable` trait                        |
| Maybe                    | `Maybe a`                        | `Option<A>` (std)                          |
| Either                   | `Either e a`                     | `Result<A, E>` (std)                       |
| Do-notation (Monad)      | `do { ... }`                     | `eff!` macro                               |
| List comprehension       | `[x                              | x <- xs]`                                  | `for_!` macro |
| Async list comprehension | `do` + `async` / `ListT IO`      | `for_async!` macro                         |
| Function composition     | `.` and `>>>`                    | `compose!` macro                           |
| Pipe                     | `&`                              | `pipe!` macro                              |
| Lens                     | `Control.Lens`                   | `Lens` trait                               |
| Prism                    | `Control.Lens.Prism`             | `Prism` trait                              |
| IO                       | `IO a`                           | `IO<A>` type                               |
| State                    | `State s a`                      | `State<S, A>` type                         |
| Reader                   | `Reader r a`                     | `Reader<R, A>` type                        |
| Writer                   | `Writer w a`                     | `Writer<W, A>` type                        |
| RWS                      | `RWS r w s a`                    | `RWS<R, W, S, A>` type                     |
| StateT                   | `StateT s m a`                   | `StateT<S, M, A>` type                     |
| ReaderT                  | `ReaderT r m a`                  | `ReaderT<R, M, A>` type                    |
| WriterT                  | `WriterT w m a`                  | `WriterT<W, M, A>` type                    |
| ExceptT                  | `ExceptT e m a`                  | `ExceptT<E, M, A>` type                    |
| Identity                 | `Identity a`                     | `Identity<A>` type                         |
| Algebraic Effects        | `Eff '[e1, e2] a` (freer-simple) | `Eff<EffCons<E1, EffCons<E2, EffNil>>, A>` |
| Effect membership        | `Member e r`                     | `Member<E, Index>` trait                   |
| Lazy                     | Default (thunks)                 | `Lazy<A>` type                             |
| Trampoline               | Trampolining                     | `Trampoline<A>` type                       |

---

## Type Classes

### Functor

| Haskell     | lambars             | Description               |
| ----------- | ------------------- | ------------------------- |
| `fmap f fa` | `Functor::fmap`     | Map function over functor |
| `f <$> fa`  | `fa.fmap(f)`        | Infix fmap                |
| `fa $> b`   | `fa.fmap(\|_\| b)`  | Replace with constant     |
| `void fa`   | `fa.fmap(\|_\| ())` | Discard value             |
| `fa <$ b`   | `fa.fmap(\|_\| b)`  | Replace keeping structure |

#### Functor Laws

```
1. Identity:     fmap id == id
2. Composition:  fmap (f . g) == fmap f . fmap g
```

#### Code Examples

```haskell
-- Haskell
import Data.Functor

doubled :: Maybe Int
doubled = fmap (*2) (Just 21)
-- doubled = Just 42

-- Using infix operator
doubled' :: Maybe Int
doubled' = (*2) <$> Just 21
-- doubled' = Just 42

-- List functor
doubledList :: [Int]
doubledList = fmap (*2) [1, 2, 3]
-- doubledList = [2, 4, 6]

-- Replace value
replaced :: Maybe String
replaced = Just 42 $> "hello"
-- replaced = Just "hello"
```

```rust
// lambars
use lambars::typeclass::Functor;

let doubled: Option<i32> = Some(21).fmap(|x| x * 2);
// doubled = Some(42)

// List (Vec) functor
let doubled_list: Vec<i32> = vec![1, 2, 3].fmap(|x| x * 2);
// doubled_list = vec![2, 4, 6]

// Replace value
let replaced: Option<String> = Some(42).fmap(|_| "hello".to_string());
// replaced = Some("hello".to_string())
```

### Applicative

| Haskell             | lambars                   | Description             |
| ------------------- | ------------------------- | ----------------------- |
| `pure a`            | `Applicative::pure`       | Lift value into context |
| `ff <*> fa`         | `Applicative::apply`      | Apply wrapped function  |
| `liftA2 f fa fb`    | `Applicative::map2`       | Lift binary function    |
| `liftA3 f fa fb fc` | `Applicative::map3`       | Lift ternary function   |
| `fa *> fb`          | `fa.map2(fb, \|_, b\| b)` | Sequence, keep right    |
| `fa <* fb`          | `fa.map2(fb, \|a, _\| a)` | Sequence, keep left     |
| `(,) <$> fa <*> fb` | `Applicative::product`    | Pair values             |

#### Applicative Laws

```
1. Identity:     pure id <*> v == v
2. Composition:  pure (.) <*> u <*> v <*> w == u <*> (v <*> w)
3. Homomorphism: pure f <*> pure x == pure (f x)
4. Interchange:  u <*> pure y == pure ($ y) <*> u
```

#### Code Examples

```haskell
-- Haskell
import Control.Applicative

x :: Maybe Int
x = pure 42
-- x = Just 42

-- Apply function in context
result :: Maybe Int
result = Just (+) <*> Just 10 <*> Just 20
-- result = Just 30

-- Using liftA2
sum :: Maybe Int
sum = liftA2 (+) (Just 10) (Just 20)
-- sum = Just 30

-- liftA3
sum3 :: Maybe Int
sum3 = liftA3 (\a b c -> a + b + c) (Just 1) (Just 2) (Just 3)
-- sum3 = Just 6

-- Pairing
paired :: Maybe (Int, String)
paired = (,) <$> Just 42 <*> Just "hello"
-- paired = Just (42, "hello")

-- Sequence operators
sequenceRight :: Maybe Int
sequenceRight = Just 10 *> Just 20
-- sequenceRight = Just 20
```

```rust
// lambars
use lambars::typeclass::Applicative;

let x: Option<i32> = <Option<()>>::pure(42);
// x = Some(42)

// Using map2
let sum: Option<i32> = Some(10).map2(Some(20), |a, b| a + b);
// sum = Some(30)

// Using map3
let sum3: Option<i32> = Some(1).map3(Some(2), Some(3), |a, b, c| a + b + c);
// sum3 = Some(6)

// Pairing with product
let paired: Option<(i32, String)> = Some(42).product(Some("hello".to_string()));
// paired = Some((42, "hello".to_string()))

// Sequence (keep right)
let sequence_right: Option<i32> = Some(10).map2(Some(20), |_, b| b);
// sequence_right = Some(20)
```

### Monad

| Haskell         | lambars                                      | Description                 |
| --------------- | -------------------------------------------- | --------------------------- |
| `return a`      | `Monad::pure` (via Applicative)              | Lift into monad             |
| `ma >>= f`      | `Monad::flat_map`                            | Bind operation              |
| `ma >> mb`      | `Monad::then`                                | Sequence, keep right        |
| `join mma`      | `Flatten::flatten` / `Option::flatten` (std) | Flatten nested monad        |
| `ma =<< f`      | `f(ma.run())`                                | Reversed bind               |
| `>=>` (Kleisli) | Manual composition                           | Compose monadic functions   |
| `<=<` (Kleisli) | Manual composition                           | Reverse Kleisli composition |

#### Monad Laws

```
1. Left Identity:  return a >>= f  ==  f a
2. Right Identity: m >>= return    ==  m
3. Associativity:  (m >>= f) >>= g ==  m >>= (\x -> f x >>= g)
```

#### Code Examples

```haskell
-- Haskell
import Control.Monad

-- Safe division
safeDivide :: Int -> Int -> Maybe Int
safeDivide _ 0 = Nothing
safeDivide x y = Just (x `div` y)

-- Bind operation
result :: Maybe Int
result = Just 10 >>= \x -> safeDivide x 2
-- result = Just 5

-- Chaining
chained :: Maybe Int
chained = Just 100 >>= safeDivide 10 >>= safeDivide 2
-- chained = Just 5

-- Using join
nested :: Maybe (Maybe Int)
nested = Just (Just 42)

flattened :: Maybe Int
flattened = join nested
-- flattened = Just 42

-- Sequence
sequenced :: Maybe Int
sequenced = Just 10 >> Just 20
-- sequenced = Just 20

-- Kleisli composition
(>=>) :: Monad m => (a -> m b) -> (b -> m c) -> a -> m c
(f >=> g) x = f x >>= g

safeSqrt :: Double -> Maybe Double
safeSqrt x = if x < 0 then Nothing else Just (sqrt x)

safeLog :: Double -> Maybe Double
safeLog x = if x <= 0 then Nothing else Just (log x)

safeSqrtLog :: Double -> Maybe Double
safeSqrtLog = safeSqrt >=> safeLog
```

```rust
// lambars
use lambars::typeclass::Monad;

// Safe division
fn safe_divide(x: i32, y: i32) -> Option<i32> {
    if y == 0 { None } else { Some(x / y) }
}

// Bind operation
let result: Option<i32> = Some(10).flat_map(|x| safe_divide(x, 2));
// result = Some(5)

// Chaining
let chained: Option<i32> = Some(100)
    .flat_map(|x| safe_divide(10, x))
    .flat_map(|x| safe_divide(x, 2));
// Note: This gives None because safe_divide(10, 100) = 0, then safe_divide(0, 2) = 0

// Using Flatten trait (also works with std Option::flatten)
use lambars::typeclass::Flatten;
let nested: Option<Option<i32>> = Some(Some(42));
let flattened: Option<i32> = nested.flatten();
// flattened = Some(42)

// Flatten also works for Result, Box, Identity
let nested_result: Result<Result<i32, &str>, &str> = Ok(Ok(42));
let flattened_result: Result<i32, &str> = nested_result.flatten();
// flattened_result = Ok(42)

// Sequence with then
let sequenced: Option<i32> = Some(10).then(Some(20));
// sequenced = Some(20)

// Kleisli-like composition (manual)
fn safe_sqrt(x: f64) -> Option<f64> {
    if x < 0.0 { None } else { Some(x.sqrt()) }
}

fn safe_log(x: f64) -> Option<f64> {
    if x <= 0.0 { None } else { Some(x.ln()) }
}

fn safe_sqrt_log(x: f64) -> Option<f64> {
    safe_sqrt(x).flat_map(safe_log)
}
```

### Semigroup and Monoid

| Haskell          | lambars                       | Description            |
| ---------------- | ----------------------------- | ---------------------- |
| `a <> b`         | `Semigroup::combine`          | Combine two values     |
| `sconcat`        | `Semigroup::combine` (folded) | Combine non-empty list |
| `mempty`         | `Monoid::empty`               | Identity element       |
| `mconcat`        | `Monoid::combine_all`         | Fold list with mappend |
| `mappend`        | `Semigroup::combine`          | Same as `<>`           |
| `Sum`, `Product` | `Sum`, `Product`              | Numeric wrappers       |
| `Min`, `Max`     | `Min`, `Max`                  | Bounded wrappers       |
| `First`, `Last`  | Custom implementation         | First/Last non-Nothing |
| `Endo`           | Custom implementation         | Endomorphism monoid    |
| `Dual`           | Custom implementation         | Reversed monoid        |

#### Semigroup/Monoid Laws

```
Semigroup:
  Associativity: (a <> b) <> c == a <> (b <> c)

Monoid:
  Left Identity:  mempty <> a == a
  Right Identity: a <> mempty == a
```

#### Code Examples

```haskell
-- Haskell
import Data.Semigroup
import Data.Monoid

-- String concatenation
combined :: String
combined = "Hello, " <> "World!"
-- combined = "Hello, World!"

-- List concatenation
listCombined :: [Int]
listCombined = [1, 2] <> [3, 4]
-- listCombined = [1, 2, 3, 4]

-- Using Sum monoid
sumResult :: Sum Int
sumResult = mconcat [Sum 1, Sum 2, Sum 3, Sum 4, Sum 5]
-- sumResult = Sum 15

-- Using Product monoid
productResult :: Product Int
productResult = mconcat [Product 1, Product 2, Product 3, Product 4, Product 5]
-- productResult = Product 120

-- Using Max and Min
maxResult :: Max Int
maxResult = mconcat [Max 3, Max 1, Max 4, Max 1, Max 5]
-- maxResult = Max 5

-- Option/Maybe as monoid (with inner semigroup)
maybeResult :: Maybe String
maybeResult = Just "Hello" <> Just " World"
-- maybeResult = Just "Hello World"
```

```rust
// lambars
use lambars::typeclass::{Semigroup, Monoid, Sum, Product, Max, Min};

// String concatenation
let combined: String = "Hello, ".to_string().combine("World!".to_string());
// combined = "Hello, World!"

// Vec concatenation
let list_combined: Vec<i32> = vec![1, 2].combine(vec![3, 4]);
// list_combined = vec![1, 2, 3, 4]

// Using Sum monoid
let items = vec![Sum::new(1), Sum::new(2), Sum::new(3), Sum::new(4), Sum::new(5)];
let sum_result: Sum<i32> = Sum::combine_all(items);
// sum_result = Sum(15)

// Using Product monoid
let items = vec![Product::new(1), Product::new(2), Product::new(3), Product::new(4), Product::new(5)];
let product_result: Product<i32> = Product::combine_all(items);
// product_result = Product(120)

// Using Max
let items = vec![Max::new(3), Max::new(1), Max::new(4), Max::new(1), Max::new(5)];
let max_result: Max<i32> = Max::combine_all(items);
// max_result = Max(5)

// Option as semigroup
let maybe_result: Option<String> = Some("Hello".to_string())
    .combine(Some(" World".to_string()));
// maybe_result = Some("Hello World")
```

### Foldable

| Haskell       | lambars                     | Description         |
| ------------- | --------------------------- | ------------------- |
| `foldl f z t` | `Foldable::fold_left`       | Left fold           |
| `foldr f z t` | `Foldable::fold_right`      | Right fold          |
| `foldMap f t` | `Foldable::fold_map`        | Map then fold       |
| `fold t`      | `Foldable::fold`            | Fold with Monoid    |
| `length t`    | `Foldable::length`          | Count elements      |
| `null t`      | `Foldable::is_empty`        | Check empty         |
| `elem x t`    | Manual implementation       | Element membership  |
| `find p t`    | `Foldable::find`            | Find first matching |
| `any p t`     | `Foldable::exists`          | Any element matches |
| `all p t`     | `Foldable::for_all`         | All elements match  |
| `toList t`    | `Foldable::to_vec`          | Convert to list     |
| `sum t`       | `fold_left(0, \|a,b\| a+b)` | Sum elements        |
| `product t`   | `fold_left(1, \|a,b\| a*b)` | Product of elements |
| `maximum t`   | `fold_left` with max        | Maximum element     |
| `minimum t`   | `fold_left` with min        | Minimum element     |

#### Code Examples

```haskell
-- Haskell
import Data.Foldable

-- Left fold
sumList :: Int
sumList = foldl (+) 0 [1, 2, 3, 4, 5]
-- sumList = 15

-- Right fold
productList :: Int
productList = foldr (*) 1 [1, 2, 3, 4, 5]
-- productList = 120

-- foldMap
concatStrings :: String
concatStrings = foldMap show [1, 2, 3]
-- concatStrings = "123"

-- find
found :: Maybe Int
found = find (> 3) [1, 2, 3, 4, 5]
-- found = Just 4

-- any and all
hasEven :: Bool
hasEven = any even [1, 2, 3]
-- hasEven = True

allPositive :: Bool
allPositive = all (> 0) [1, 2, 3]
-- allPositive = True

-- length and null
listLength :: Int
listLength = length [1, 2, 3, 4, 5]
-- listLength = 5

isEmpty :: Bool
isEmpty = null []
-- isEmpty = True
```

```rust
// lambars
use lambars::typeclass::{Foldable, Monoid};

// Left fold
let sum_list: i32 = vec![1, 2, 3, 4, 5].fold_left(0, |acc, x| acc + x);
// sum_list = 15

// Right fold
let product_list: i32 = vec![1, 2, 3, 4, 5].fold_right(1, |x, acc| x * acc);
// product_list = 120

// fold_map
let concat_strings: String = vec![1, 2, 3].fold_map(|x| x.to_string());
// concat_strings = "123"

// find
let found: Option<&i32> = vec![1, 2, 3, 4, 5].find(|x| **x > 3);
// found = Some(&4)

// exists and for_all
let has_even: bool = vec![1, 2, 3].exists(|x| x % 2 == 0);
// has_even = true

let all_positive: bool = vec![1, 2, 3].for_all(|x| *x > 0);
// all_positive = true

// length and is_empty
let list_length: usize = vec![1, 2, 3, 4, 5].length();
// list_length = 5

let is_empty: bool = Vec::<i32>::new().is_empty();
// is_empty = true
```

### Traversable

| Haskell                 | lambars                               | Description                     |
| ----------------------- | ------------------------------------- | ------------------------------- |
| `traverse f t`          | `Traversable::traverse_option/result` | Traverse with effect            |
| `sequenceA t`           | `Traversable::sequence_option/result` | Sequence effects                |
| `for t f`               | `traverse` with flipped args          | Traverse (args flipped)         |
| `mapM f t`              | `traverse_option/result`              | Same as traverse (for Monad)    |
| `sequence t`            | `sequence_option/result`              | Same as sequenceA (for Monad)   |
| `forM t f`              | `traverse` flipped                    | Same as for (for Monad)         |
| `traverse @(Reader r)`  | `Traversable::traverse_reader`        | Traverse with Reader effect     |
| `traverse @(State s)`   | `Traversable::traverse_state`         | Traverse with State effect      |
| `traverse @IO`          | `Traversable::traverse_io`            | Traverse with IO effect         |
| `traverse @(Async IO)`  | `Traversable::traverse_async_io`      | Traverse with AsyncIO effect    |
| `mapConcurrently`       | `Traversable::traverse_async_io_parallel` | Traverse AsyncIO in parallel    |
| `sequence @(Reader r)`  | `Traversable::sequence_reader`        | Sequence Reader effects         |
| `sequence @(State s)`   | `Traversable::sequence_state`         | Sequence State effects          |
| `sequence @IO`          | `Traversable::sequence_io`            | Sequence IO effects             |
| `sequence @(Async IO)`  | `Traversable::sequence_async_io`      | Sequence AsyncIO effects        |
| `traverse_ @(Reader r)` | `Traversable::traverse_reader_`       | Traverse Reader, discard result |
| `traverse_ @(State s)`  | `Traversable::traverse_state_`        | Traverse State, discard result  |
| `traverse_ @IO`         | `Traversable::traverse_io_`           | Traverse IO, discard result     |
| `for_ @(Reader r)`      | `Traversable::for_each_reader`        | Alias for traverse*reader*      |
| `for_ @(State s)`       | `Traversable::for_each_state`         | Alias for traverse*state*       |
| `for_ @IO`              | `Traversable::for_each_io`            | Alias for traverse*io*          |

#### Traversable Laws

```
1. Naturality:    t . traverse f == traverse (t . f)  (for any applicative transformation t)
2. Identity:      traverse Identity == Identity
3. Composition:   traverse (Compose . fmap g . f) == Compose . fmap (traverse g) . traverse f
```

#### Code Examples

```haskell
-- Haskell
import Data.Traversable

parseIntMaybe :: String -> Maybe Int
parseIntMaybe s = case reads s of
  [(n, "")] -> Just n
  _         -> Nothing

-- traverse
result :: Maybe [Int]
result = traverse parseIntMaybe ["1", "2", "3"]
-- result = Just [1, 2, 3]

failed :: Maybe [Int]
failed = traverse parseIntMaybe ["1", "two", "3"]
-- failed = Nothing

-- sequence
list :: [Maybe Int]
list = [Just 1, Just 2, Just 3]

sequenced :: Maybe [Int]
sequenced = sequenceA list
-- sequenced = Just [1, 2, 3]

withNothing :: [Maybe Int]
withNothing = [Just 1, Nothing, Just 3]

sequencedNothing :: Maybe [Int]
sequencedNothing = sequenceA withNothing
-- sequencedNothing = Nothing

-- With Either
parseIntEither :: String -> Either String Int
parseIntEither s = case reads s of
  [(n, "")] -> Right n
  _         -> Left ("Cannot parse: " ++ s)

eitherResult :: Either String [Int]
eitherResult = traverse parseIntEither ["1", "2", "3"]
-- eitherResult = Right [1, 2, 3]

eitherFailed :: Either String [Int]
eitherFailed = traverse parseIntEither ["1", "two", "3"]
-- eitherFailed = Left "Cannot parse: two"
```

```rust
// lambars
use lambars::typeclass::Traversable;

fn parse_int_option(s: &str) -> Option<i32> {
    s.parse().ok()
}

// traverse_option
let result: Option<Vec<i32>> = vec!["1", "2", "3"].traverse_option(parse_int_option);
// result = Some(vec![1, 2, 3])

let failed: Option<Vec<i32>> = vec!["1", "two", "3"].traverse_option(parse_int_option);
// failed = None

// sequence_option
let list: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
let sequenced: Option<Vec<i32>> = list.sequence_option();
// sequenced = Some(vec![1, 2, 3])

let with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
let sequenced_none: Option<Vec<i32>> = with_none.sequence_option();
// sequenced_none = None

// With Result
fn parse_int_result(s: &str) -> Result<i32, String> {
    s.parse().map_err(|_| format!("Cannot parse: {}", s))
}

let result_ok: Result<Vec<i32>, String> = vec!["1", "2", "3"]
    .traverse_result(parse_int_result);
// result_ok = Ok(vec![1, 2, 3])

let result_err: Result<Vec<i32>, String> = vec!["1", "two", "3"]
    .traverse_result(parse_int_result);
// result_err = Err("Cannot parse: two")

// traverse_reader - traverse with Reader effect
use lambars::effect::Reader;

#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
// result = vec![10, 20, 30]

// traverse_state - traverse with State effect (state is threaded left-to-right)
use lambars::effect::State;

let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
// result = vec![(0, "a"), (1, "b"), (2, "c")]
// final_index = 3

// traverse_io - traverse with IO effect (IO actions executed sequentially)
use lambars::effect::IO;

let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
// contents = vec!["content of a.txt", "content of b.txt"]
```

---

## Maybe and Either

### Maybe / Option

| Haskell          | lambars / Rust std                   | Description            |
| ---------------- | ------------------------------------ | ---------------------- |
| `Just x`         | `Some(x)`                            | Construct Just/Some    |
| `Nothing`        | `None`                               | Construct Nothing/None |
| `fmap f ma`      | `Functor::fmap`                      | Map over Maybe         |
| `ma >>= f`       | `Monad::flat_map`                    | Bind                   |
| `fromMaybe d ma` | `Option::unwrap_or`                  | Default value          |
| `maybe d f ma`   | `Option::map_or`                     | Fold Maybe             |
| `isJust ma`      | `Option::is_some`                    | Test for Just          |
| `isNothing ma`   | `Option::is_none`                    | Test for Nothing       |
| `fromJust ma`    | `Option::unwrap`                     | Extract (unsafe)       |
| `listToMaybe xs` | `xs.first()`                         | First element          |
| `maybeToList ma` | `Option::into_iter`                  | To list                |
| `catMaybes xs`   | `Iterator::flatten`                  | Filter Nothings        |
| `mapMaybe f xs`  | `Iterator::filter_map`               | Map and filter         |
| `ma <\|> mb`     | `Option::or`                         | Alternative            |
| `guard cond`     | `if cond { Some(()) } else { None }` | Guard in monad         |

### Either / Result

| Haskell            | lambars / Rust std                    | Description          |
| ------------------ | ------------------------------------- | -------------------- |
| `Right x`          | `Ok(x)`                               | Construct Right/Ok   |
| `Left e`           | `Err(e)`                              | Construct Left/Err   |
| `fmap f ea`        | `Functor::fmap` / `Result::map`       | Map over Right       |
| `first f ea`       | `Result::map_err`                     | Map over Left        |
| `bimap f g ea`     | Manual                                | Map both sides       |
| `ea >>= f`         | `Monad::flat_map`                     | Bind                 |
| `either f g ea`    | `Result::map_or_else`                 | Fold Either          |
| `isRight ea`       | `Result::is_ok`                       | Test for Right       |
| `isLeft ea`        | `Result::is_err`                      | Test for Left        |
| `fromRight d ea`   | `Result::unwrap_or`                   | Default for Right    |
| `fromLeft d ea`    | `Result::err().unwrap_or`             | Default for Left     |
| `rights xs`        | `Iterator::filter_map(\|r\| r.ok())`  | Filter Rights        |
| `lefts xs`         | `Iterator::filter_map(\|r\| r.err())` | Filter Lefts         |
| `partitionEithers` | Manual                                | Split into two lists |

#### Code Examples

```haskell
-- Haskell
import Data.Maybe
import Data.Either

-- Maybe operations
doubled :: Maybe Int
doubled = fmap (*2) (Just 21)
-- doubled = Just 42

defaulted :: Int
defaulted = fromMaybe 0 Nothing
-- defaulted = 0

-- Either operations
mapped :: Either String Int
mapped = fmap (*2) (Right 21)
-- mapped = Right 42

leftMapped :: Either Int String
leftMapped = first length (Left "error")
-- leftMapped = Left 5

biMapped :: Either Int String
biMapped = bimap length show (Right 42)
-- biMapped = Right "42"
```

```rust
// lambars / std
use lambars::typeclass::{Functor, Monad};

// Option operations
let doubled: Option<i32> = Some(21).fmap(|x| x * 2);
// doubled = Some(42)

let defaulted: i32 = None.unwrap_or(0);
// defaulted = 0

// Result operations
let mapped: Result<i32, String> = Ok(21).fmap(|x| x * 2);
// mapped = Ok(42)

let left_mapped: Result<i32, usize> = Err("error".to_string()).map_err(|e| e.len());
// left_mapped = Err(5)

// bimap equivalent
let result: Result<i32, String> = Ok(42);
let bi_mapped: Result<String, usize> = result
    .map(|x| x.to_string())
    .map_err(|e| e.len());
// bi_mapped = Ok("42")
```

---

## Do-Notation and List Comprehensions

lambars provides two macros that correspond to Haskell's do-notation and list comprehensions:

| Use Case           | Haskell               | lambars      | Description                    |
| ------------------ | --------------------- | ------------ | ------------------------------ | ------------------------------- |
| Monad binding      | `do { x <- mx; ... }` | `eff!` macro | Single execution, FnOnce-based |
| List comprehension | `[f x                 | x <- xs]`    | `for_!` macro                  | Multiple execution, FnMut-based |

### eff! Macro (Do-Notation for Monads)

#### Syntax Comparison

| Haskell               | lambars                    | Description        |
| --------------------- | -------------------------- | ------------------ |
| `do { x <- mx; ... }` | `eff! { x <= mx; ... }`    | Bind               |
| `let x = expr`        | `let x = expr;`            | Pure binding       |
| `pure x`              | `Some(x)` / `Ok(x)` / etc. | Return value       |
| `mx >> my`            | `_ <= mx; my`              | Sequence (discard) |
| Guard (MonadPlus)     | `_ <= guard(cond);`        | Guard              |

### Code Examples

```haskell
-- Haskell - do notation
computation :: Maybe Int
computation = do
  x <- Just 10
  y <- Just 20
  let z = x + y
  pure (z * 2)
-- computation = Just 60

-- With guards (requires MonadPlus/Alternative)
withGuard :: Maybe Int
withGuard = do
  x <- Just 10
  guard (x > 5)
  pure (x * 2)
-- withGuard = Just 20

-- Nested computations
nested :: Maybe Int
nested = do
  list <- Just [1, 2, 3]
  first <- listToMaybe list
  let doubled = first * 2
  pure doubled
-- nested = Just 2

-- Either do-notation
eitherComputation :: Either String Int
eitherComputation = do
  x <- Right 10
  y <- Right 20
  pure (x + y)
-- eitherComputation = Right 30

-- With pattern matching
withPattern :: Maybe (Int, Int)
withPattern = do
  (a, b) <- Just (10, 20)
  pure (a + b, a * b)
-- withPattern = Just (30, 200)
```

```rust
// lambars - eff! macro
use lambars::eff;

let computation: Option<i32> = eff! {
    x <= Some(10);
    y <= Some(20);
    let z = x + y;
    Some(z * 2)
};
// computation = Some(60)

// With guard-like pattern
fn guard(condition: bool) -> Option<()> {
    if condition { Some(()) } else { None }
}

let with_guard: Option<i32> = eff! {
    x <= Some(10);
    _ <= guard(x > 5);
    Some(x * 2)
};
// with_guard = Some(20)

// Nested computations
let nested: Option<i32> = eff! {
    list <= Some(vec![1, 2, 3]);
    first <= list.first().copied();
    let doubled = first * 2;
    Some(doubled)
};
// nested = Some(2)

// Result eff! macro
let result_computation: Result<i32, String> = eff! {
    x <= Ok::<i32, String>(10);
    y <= Ok::<i32, String>(20);
    Ok(x + y)
};
// result_computation = Ok(30)

// With tuple destructuring
let with_pattern: Option<(i32, i32)> = eff! {
    pair <= Some((10, 20));
    let (a, b) = pair;
    Some((a + b, a * b))
};
// with_pattern = Some((30, 200))
```

### Complex Examples

```haskell
-- Haskell - Database-like operations
data User = User { userId :: Int, userName :: String }
data Order = Order { orderId :: Int, orderUserId :: Int, orderAmount :: Double }

findUser :: Int -> Maybe User
findUser = undefined

findOrders :: Int -> Maybe [Order]
findOrders = undefined

getUserOrders :: Int -> Maybe (User, [Order])
getUserOrders uid = do
  user <- findUser uid
  orders <- findOrders (userId user)
  pure (user, orders)
```

```rust
// lambars
use lambars::eff;

struct User { user_id: i32, user_name: String }
struct Order { order_id: i32, order_user_id: i32, order_amount: f64 }

fn find_user(id: i32) -> Option<User> { None } // placeholder
fn find_orders(user_id: i32) -> Option<Vec<Order>> { None } // placeholder

fn get_user_orders(uid: i32) -> Option<(User, Vec<Order>)> {
    eff! {
        user <= find_user(uid);
        orders <= find_orders(user.user_id);
        Some((user, orders))
    }
}
```

### for\_! Macro (List Comprehensions)

For Haskell list comprehensions, use the `for_!` macro with `yield`:

#### Syntax Comparison

| Haskell                       | lambars                                           | Description         |
| ----------------------------- | ------------------------------------------------- | ------------------- |
| `[f x \| x <- xs]`            | `for_! { x <= xs; yield f(x) }`                   | Basic comprehension |
| `[x + y \| x <- xs, y <- ys]` | `for_! { x <= xs; y <= ys.clone(); yield x + y }` | Nested              |
| `[x \| x <- xs, p x]`         | `xs.into_iter().filter(p).collect()`              | Filter (use std)    |
| `let y = expr`                | `let y = expr;`                                   | Pure binding        |

**Important**: In `for_!`, inner collections need `.clone()` due to Rust's ownership rules.

#### Code Examples

```haskell
-- Haskell - List comprehension
doubled :: [Int]
doubled = [x * 2 | x <- [1, 2, 3, 4, 5]]
-- doubled = [2, 4, 6, 8, 10]

-- Nested comprehension (cartesian product)
cartesian :: [Int]
cartesian = [x + y | x <- [1, 2], y <- [10, 20]]
-- cartesian = [11, 21, 12, 22]

-- With filtering
filtered :: [Int]
filtered = [x | x <- [1, 2, 3, 4, 5], even x]
-- filtered = [2, 4]

-- Complex example with multiple generators
triples :: [(Int, Int, Int)]
triples = [(a, b, c) | c <- [1..10], b <- [1..c], a <- [1..b], a^2 + b^2 == c^2]
-- triples = [(3, 4, 5), (6, 8, 10)]
```

```rust
// lambars - for_! macro
use lambars::for_;

let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// Nested comprehension (cartesian product)
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // Note: clone() needed for inner iteration
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// With filtering (use std iterator methods)
let filtered: Vec<i32> = (1..=5).filter(|x| x % 2 == 0).collect();
// filtered = vec![2, 4]

// Pythagorean triples example
let triples: Vec<(i32, i32, i32)> = for_! {
    c <= 1..=10;
    b <= (1..=c).collect::<Vec<_>>();
    a <= (1..=b).collect::<Vec<_>>();
    yield (a, b, c)
}.into_iter()
 .filter(|(a, b, c)| a * a + b * b == c * c)
 .collect();
// triples = vec![(3, 4, 5), (6, 8, 10)]
```

### When to Use Each Macro

| Scenario                  | Recommended Macro | Reason                                  |
| ------------------------- | ----------------- | --------------------------------------- |
| Maybe/Either chaining     | `eff!`            | Short-circuits on Nothing/Left          |
| IO/State/Reader/Writer    | `eff!`            | Designed for FnOnce monads              |
| List generation           | `for_!`           | Supports multiple iterations with yield |
| Cartesian products        | `for_!`           | Nested iteration                        |
| Database-style queries    | `eff!`            | Monadic error handling                  |
| Async list generation     | `for_async!`      | Async iteration with yield              |
| Async operations in loops | `for_async!`      | Uses `<~` for AsyncIO binding           |

---

## Function Composition

| Haskell                   | lambars               | Description                   |
| ------------------------- | --------------------- | ----------------------------- |
| `f . g`                   | `compose!(f, g)`      | Right-to-left composition     |
| `f >>> g` (Control.Arrow) | `compose!(g, f)`      | Left-to-right composition     |
| `f <<< g` (Control.Arrow) | `compose!(f, g)`      | Same as `.`                   |
| `x & f`                   | `pipe!(x, f)`         | Pipe operator                 |
| `fmap f m`                | `pipe!(m, => f)`      | Lift pure function in monad   |
| `m >>= f`                 | `pipe!(m, =>> f)`     | Bind monadic function         |
| `f $ x`                   | `f(x)`                | Function application          |
| `flip f`                  | `flip(f)`             | Flip arguments                |
| `const x`                 | `constant(x)`         | Constant function             |
| `id`                      | `identity`            | Identity function             |
| `on`                      | Manual implementation | Binary function on projection |

### Code Examples

```haskell
-- Haskell
import Data.Function
import Control.Arrow

addOne :: Int -> Int
addOne = (+1)

double :: Int -> Int
double = (*2)

square :: Int -> Int
square = (^2)

-- Right-to-left composition (.)
composed1 :: Int -> Int
composed1 = addOne . double . square
-- composed1 5 = addOne (double (square 5)) = addOne (double 25) = addOne 50 = 51

-- Left-to-right composition (>>>)
composed2 :: Int -> Int
composed2 = square >>> double >>> addOne
-- composed2 5 = 51

-- Using pipe (&)
result :: Int
result = 5 & square & double & addOne
-- result = 51

-- flip
subtract' :: Int -> Int -> Int
subtract' = (-)

flippedSubtract :: Int -> Int -> Int
flippedSubtract = flip subtract'
-- flippedSubtract 3 10 = 7 (i.e., 10 - 3)

-- const
alwaysFive :: a -> Int
alwaysFive = const 5
-- alwaysFive "anything" = 5

-- on
compareByLength :: String -> String -> Ordering
compareByLength = compare `on` length
```

```rust
// lambars
use lambars::{compose, pipe};
use lambars::compose::{identity, constant, flip};

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }
fn square(x: i32) -> i32 { x * x }

// Right-to-left composition (like Haskell's .)
let composed1 = compose!(add_one, double, square);
let result1 = composed1(5);
// result1 = 51

// Using pipe! for left-to-right data flow
let result2 = pipe!(5, square, double, add_one);
// result2 = 51

// flip
fn subtract(a: i32, b: i32) -> i32 { a - b }
let flipped_subtract = flip(subtract);
let result = flipped_subtract(3, 10);
// result = 7 (i.e., 10 - 3)

// constant
let always_five = constant(5);
let result = always_five("anything");
// result = 5

// identity
let x = identity(42);
// x = 42
```

---

## Currying and Partial Application

| Haskell                 | lambars                                           | Description         |
| ----------------------- | ------------------------------------------------- | ------------------- |
| Auto-curried            | `curry!(fn, arity)` or `curry!(\|args...\| body)` | Explicit currying   |
| Partial application     | `partial!`                                        | Partial application |
| Sections `(+1)`, `(1+)` | Closures                                          | Operator sections   |
| `uncurry f`             | `\|(a, b)\| f(a, b)`                              | Uncurry             |

### Code Examples

```haskell
-- Haskell - All functions are curried by default
add :: Int -> Int -> Int
add x y = x + y

addFive :: Int -> Int
addFive = add 5

result :: Int
result = addFive 3
-- result = 8

-- Multi-argument partial application
add3 :: Int -> Int -> Int -> Int
add3 x y z = x + y + z

addFiveAndTen :: Int -> Int
addFiveAndTen = add3 5 10

result3 :: Int
result3 = addFiveAndTen 3
-- result3 = 18

-- Sections
addOne :: Int -> Int
addOne = (+1)

halve :: Double -> Double
halve = (/2)

-- uncurry
addTuple :: (Int, Int) -> Int
addTuple = uncurry add
-- addTuple (3, 5) = 8
```

```rust
// lambars
use lambars::{curry, partial};

fn add(a: i32, b: i32) -> i32 { a + b }

// Currying with function name + arity form
let curried_add = curry!(add, 2);
let add_five = curried_add(5);
let result = add_five(3);
// result = 8

// Currying with closure form
let curried_add = curry!(|a, b| add(a, b));
let add_five = curried_add(5);
let result = add_five(3);
// result = 8

// Multi-argument currying
fn add3(a: i32, b: i32, c: i32) -> i32 { a + b + c }

let curried_add3 = curry!(add3, 3);
let add_five = curried_add3(5);
let add_five_and_ten = add_five(10);
let result3 = add_five_and_ten(3);
// result3 = 18

// Partial application with placeholders
let add_five_partial = partial!(add, 5, __);
let result = add_five_partial(3);
// result = 8

// Sections equivalent (closures)
let add_one = |x: i32| x + 1;
let halve = |x: f64| x / 2.0;

// uncurry equivalent
let add_tuple = |(a, b): (i32, i32)| add(a, b);
let result = add_tuple((3, 5));
// result = 8
```

---

## Lazy Evaluation

| Haskell             | lambars                 | Description        |
| ------------------- | ----------------------- | ------------------ |
| Default lazy        | `Lazy::new`             | Explicit lazy      |
| `seq a b`           | `Lazy::force`           | Force evaluation   |
| Bang patterns `!x`  | Rust is strict          | Strict patterns    |
| `$!` (strict apply) | Normal application      | Strict application |
| Infinite lists      | `Iterator`              | Lazy sequences     |
| `take n xs`         | `Iterator::take`        | Take n elements    |
| `drop n xs`         | `Iterator::skip`        | Skip n elements    |
| `iterate f x`       | `std::iter::successors` | Iterate function   |
| `repeat x`          | `std::iter::repeat`     | Infinite repeat    |
| `cycle xs`          | `Iterator::cycle`       | Cycle list         |

### Code Examples

```haskell
-- Haskell - Lazy by default
expensive :: Int
expensive = trace "Computing..." (42 * 1000000)

-- Not evaluated until needed
main :: IO ()
main = do
  let x = expensive  -- Not computed yet
  putStrLn "Before"
  print x            -- Now computed
  print x            -- Cached, not recomputed

-- Infinite lists
naturals :: [Int]
naturals = [0..]

firstTen :: [Int]
firstTen = take 10 naturals
-- firstTen = [0,1,2,3,4,5,6,7,8,9]

-- Infinite list with iterate
powersOfTwo :: [Int]
powersOfTwo = iterate (*2) 1

firstPowers :: [Int]
firstPowers = take 10 powersOfTwo
-- firstPowers = [1,2,4,8,16,32,64,128,256,512]

-- Forcing evaluation with seq
strictSum :: Int -> Int -> Int
strictSum x y = x `seq` y `seq` (x + y)
```

```rust
// lambars - Explicit lazy evaluation
use lambars::control::Lazy;

let expensive = Lazy::new(|| {
    println!("Computing...");
    42 * 1000000
});

// Not evaluated until needed
println!("Before");
let x = expensive.force();  // Now computed, prints "Computing..."
let y = expensive.force();  // Cached, not recomputed

// Infinite iterators (similar to Haskell's lazy lists)
let naturals = 0..;
let first_ten: Vec<i32> = naturals.take(10).collect();
// first_ten = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

// iterate equivalent
let powers_of_two = std::iter::successors(Some(1), |&n| Some(n * 2));
let first_powers: Vec<i32> = powers_of_two.take(10).collect();
// first_powers = vec![1, 2, 4, 8, 16, 32, 64, 128, 256, 512]

// repeat
let repeated: Vec<i32> = std::iter::repeat(42).take(5).collect();
// repeated = vec![42, 42, 42, 42, 42]

// cycle
let cycled: Vec<i32> = vec![1, 2, 3].into_iter().cycle().take(7).collect();
// cycled = vec![1, 2, 3, 1, 2, 3, 1]
```

---

## Optics (lens)

### Lens

| Haskell (lens)              | lambars              | Description       |
| --------------------------- | -------------------- | ----------------- |
| `makeLenses ''Type`         | `lens!(Type, field)` | Generate lenses   |
| `view l s` / `s ^. l`       | `Lens::get`          | Get focused value |
| `set l a s` / `s & l .~ a`  | `Lens::set`          | Set value         |
| `over l f s` / `s & l %~ f` | `Lens::modify`       | Modify value      |
| `l1 . l2`                   | `Lens::compose`      | Compose lenses    |
| `_1`, `_2`                  | `lens!((A,B), 0)`    | Tuple lenses      |

### Prism

| Haskell (lens)              | lambars                 | Description          |
| --------------------------- | ----------------------- | -------------------- |
| `makePrisms ''Type`         | `prism!(Type, Variant)` | Generate prisms      |
| `preview p s` / `s ^? p`    | `Prism::preview`        | Get if matches       |
| `review p a` / `p # a`      | `Prism::review`         | Construct from value |
| `over p f s` / `s & p %~ f` | `Prism::modify`         | Modify if matches    |
| `_Just`, `_Nothing`         | `prism!(Option, Some)`  | Maybe prisms         |
| `_Left`, `_Right`           | `prism!(Result, Ok)`    | Either prisms        |

### Iso

| Haskell (lens) | lambars            | Description         |
| -------------- | ------------------ | ------------------- |
| `iso f g`      | `FunctionIso::new` | Create isomorphism  |
| `view i s`     | `Iso::get`         | Forward conversion  |
| `review i a`   | `Iso::reverse_get` | Backward conversion |
| `from i`       | `Iso::reverse`     | Flip direction      |

### Traversal

| Haskell (lens) | lambars              | Description        |
| -------------- | -------------------- | ------------------ |
| `traversed`    | `VecTraversal::new`  | List traversal     |
| `toListOf t s` | `Traversal::get_all` | Get all targets    |
| `over t f s`   | `Traversal::modify`  | Modify all targets |
| `each`         | `VecTraversal`       | Each element       |

### Code Examples

```haskell
-- Haskell (lens library)
{-# LANGUAGE TemplateHaskell #-}
import Control.Lens

data Address = Address
  { _street :: String
  , _city :: String
  } deriving (Show)
makeLenses ''Address

data Person = Person
  { _name :: String
  , _address :: Address
  } deriving (Show)
makeLenses ''Person

-- Get
streetName :: Person -> String
streetName p = p ^. address . street

-- Set
setStreet :: Person -> String -> Person
setStreet p s = p & address . street .~ s

-- Modify
upperStreet :: Person -> Person
upperStreet p = p & address . street %~ map toUpper

-- Example
person :: Person
person = Person "Alice" (Address "Main St" "Tokyo")

result :: String
result = person ^. address . street
-- result = "Main St"

updated :: Person
updated = person & address . street .~ "Oak Ave"
-- updated.address.street = "Oak Ave"

-- Prism example
data Shape = Circle Double | Rectangle Double Double

_Circle :: Prism' Shape Double
_Circle = prism' Circle $ \case
  Circle r -> Just r
  _ -> Nothing

getRadius :: Shape -> Maybe Double
getRadius s = s ^? _Circle

constructCircle :: Double -> Shape
constructCircle r = _Circle # r
```

```rust
// lambars
use lambars::optics::{Lens, Prism, FunctionPrism};
use lambars::lens;
use lambars::prism;

#[derive(Clone, Debug)]
struct Address {
    street: String,
    city: String,
}

#[derive(Clone, Debug)]
struct Person {
    name: String,
    address: Address,
}

let address_lens = lens!(Person, address);
let street_lens = lens!(Address, street);

// Compose lenses
let person_street = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        street: "Main St".to_string(),
        city: "Tokyo".to_string(),
    },
};

// Get
let street_name: &String = person_street.get(&person);
// street_name = "Main St"

// Set
let updated = person_street.set(person.clone(), "Oak Ave".to_string());
// updated.address.street = "Oak Ave"

// Modify
let uppercased = person_street.modify(person, |s| s.to_uppercase());
// uppercased.address.street = "MAIN ST"

// Prism example
#[derive(Clone, Debug, PartialEq)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

let circle_prism = prism!(Shape, Circle);

// Preview
let shape = Shape::Circle(5.0);
let radius: Option<&f64> = circle_prism.preview(&shape);
// radius = Some(&5.0)

// Review (construct)
let constructed: Shape = circle_prism.review(10.0);
// constructed = Shape::Circle(10.0)
```

---

## Effect Monads

### IO Monad

| Haskell               | lambars          | Description       |
| --------------------- | ---------------- | ----------------- |
| `pure a` / `return a` | `IO::pure`       | Pure value in IO  |
| `IO action`           | `IO::new`        | Create IO action  |
| `io >>= f`            | `IO::flat_map`   | Bind IO actions   |
| `io >> io2`           | `IO::then`       | Sequence          |
| `putStrLn s`          | `IO::print_line` | Print line        |
| `getLine`             | `IO::read_line`  | Read line         |
| `threadDelay n`       | `IO::delay`      | Delay execution   |
| `catch io handler`    | `IO::catch`      | Handle exceptions |

#### Code Examples

```haskell
-- Haskell
import Control.Exception

computation :: IO Int
computation = do
  putStrLn "Computing..."
  pure 42

chained :: IO Int
chained = do
  x <- pure 10
  y <- pure 20
  pure (x + y)

-- Error handling
safeComputation :: IO Int
safeComputation = catch
  (error "oops" :: IO Int)
  (\(e :: SomeException) -> pure 0)

-- Main example
main :: IO ()
main = do
  putStrLn "Enter a number:"
  line <- getLine
  let n = read line :: Int
  putStrLn ("You entered: " ++ show (n * 2))
```

```rust
// lambars
use lambars::effect::IO;

let computation: IO<i32> = IO::new(|| {
    println!("Computing...");
    42
});

let chained: IO<i32> = IO::pure(10)
    .flat_map(|x| IO::pure(20).fmap(move |y| x + y));

// Error handling
let safe_computation: IO<i32> = IO::catch(
    IO::new(|| panic!("oops")),
    |_| 0
);

// Using run_unsafe to execute
let result = computation.run_unsafe();
// Prints "Computing..." and result = 42

// IO actions
let print_io = IO::print_line("Hello, World!");
print_io.run_unsafe();  // Prints "Hello, World!"
```

### State Monad

| Haskell               | lambars                  | Description            |
| --------------------- | ------------------------ | ---------------------- |
| `pure a` / `return a` | `State::pure`            | Pure value             |
| `get`                 | `State::get`             | Get state              |
| `put s`               | `State::put`             | Set state              |
| `modify f`            | `State::modify`          | Modify state           |
| `gets f`              | `State::gets`            | Get derived value      |
| `runState m s`        | `State::run`             | Run with initial state |
| `evalState m s`       | `State::eval`            | Get result only        |
| `execState m s`       | `State::exec`            | Get final state only   |
| `state f`             | `State::from_transition` | Create from function   |

#### Code Examples

```haskell
-- Haskell
import Control.Monad.State

type Counter = State Int

increment :: Counter ()
increment = modify (+1)

decrement :: Counter ()
decrement = modify (subtract 1)

getCount :: Counter Int
getCount = get

computation :: Counter Int
computation = do
  increment
  increment
  increment
  decrement
  getCount

result :: (Int, Int)
result = runState computation 0
-- result = (2, 2)  -- (return value, final state)
```

```rust
// lambars
use lambars::effect::State;
use lambars::eff;

type Counter<A> = State<i32, A>;

fn increment() -> Counter<()> {
    State::modify(|s| s + 1)
}

fn decrement() -> Counter<()> {
    State::modify(|s| s - 1)
}

fn get_count() -> Counter<i32> {
    State::get()
}

let computation: Counter<i32> = eff! {
    _ <= increment();
    _ <= increment();
    _ <= increment();
    _ <= decrement();
    get_count()
};

let (result, final_state) = computation.run(0);
// result = 2, final_state = 2
```

### Reader Monad

| Haskell               | lambars         | Description          |
| --------------------- | --------------- | -------------------- |
| `pure a` / `return a` | `Reader::pure`  | Pure value           |
| `ask`                 | `Reader::ask`   | Get environment      |
| `asks f`              | `Reader::asks`  | Get derived value    |
| `local f m`           | `Reader::local` | Modify environment   |
| `runReader m r`       | `Reader::run`   | Run with environment |

#### Code Examples

```haskell
-- Haskell
import Control.Monad.Reader

data Config = Config
  { configHost :: String
  , configPort :: Int
  }

type App a = Reader Config a

getHost :: App String
getHost = asks configHost

getPort :: App Int
getPort = asks configPort

getUrl :: App String
getUrl = do
  host <- getHost
  port <- getPort
  pure (host ++ ":" ++ show port)

result :: String
result = runReader getUrl (Config "localhost" 8080)
-- result = "localhost:8080"
```

```rust
// lambars
use lambars::effect::Reader;
use lambars::eff;

#[derive(Clone)]
struct Config {
    host: String,
    port: i32,
}

type App<A> = Reader<Config, A>;

fn get_host() -> App<String> {
    Reader::asks(|c: &Config| c.host.clone())
}

fn get_port() -> App<i32> {
    Reader::asks(|c: &Config| c.port)
}

fn get_url() -> App<String> {
    eff! {
        host <= get_host();
        port <= get_port();
        Reader::pure(format!("{}:{}", host, port))
    }
}

let result = get_url().run(Config {
    host: "localhost".to_string(),
    port: 8080,
});
// result = "localhost:8080"
```

### Writer Monad

| Haskell               | lambars          | Description               |
| --------------------- | ---------------- | ------------------------- |
| `pure a` / `return a` | `Writer::pure`   | Pure value                |
| `tell w`              | `Writer::tell`   | Log output                |
| `listen m`            | `Writer::listen` | Access log in computation |
| `pass m`              | `Writer::pass`   | Transform log             |
| `censor f m`          | `Writer::censor` | Censor log                |
| `runWriter m`         | `Writer::run`    | Get (result, log)         |
| `execWriter m`        | `Writer::exec`   | Get log only              |

#### Code Examples

```haskell
-- Haskell
import Control.Monad.Writer

type Logged a = Writer [String] a

logMsg :: String -> Logged ()
logMsg msg = tell [msg]

computation :: Logged Int
computation = do
  logMsg "Starting"
  let x = 10
  logMsg ("Got " ++ show x)
  let y = x * 2
  logMsg ("Doubled to " ++ show y)
  pure y

(result, log) :: (Int, [String])
(result, log) = runWriter computation
-- result = 20
-- log = ["Starting", "Got 10", "Doubled to 20"]
```

```rust
// lambars
use lambars::effect::Writer;
use lambars::eff;

type Logged<A> = Writer<Vec<String>, A>;

fn log_msg(msg: String) -> Logged<()> {
    Writer::tell(vec![msg])
}

let computation: Logged<i32> = eff! {
    _ <= log_msg("Starting".to_string());
    let x = 10;
    _ <= log_msg(format!("Got {}", x));
    let y = x * 2;
    _ <= log_msg(format!("Doubled to {}", y));
    Writer::pure(y)
};

let (result, log) = computation.run();
// result = 20
// log = vec!["Starting", "Got 10", "Doubled to 20"]
```

### RWS Monad

| Haskell         | lambars           | Description                          |
| --------------- | ----------------- | ------------------------------------ |
| `RWS r w s a`   | `RWS<R, W, S, A>` | Combined Reader + Writer + State     |
| `rws f`         | `RWS::new`        | Create from function                 |
| `runRWS m r s`  | `RWS::run`        | Run with environment and state       |
| `evalRWS m r s` | `RWS::eval`       | Get (result, output) only            |
| `execRWS m r s` | `RWS::exec`       | Get (state, output) only             |
| `mapRWS f m`    | `RWS::map_rws`    | Transform (result, state, output)    |
| `withRWS f m`   | `RWS::with_rws`   | Transform (environment, state) input |
| `ask`           | `RWS::ask`        | Get environment                      |
| `asks f`        | `RWS::asks`       | Project from environment             |
| `local f m`     | `RWS::local`      | Modify environment locally           |
| `tell w`        | `RWS::tell`       | Add output                           |
| `listen m`      | `RWS::listen`     | Capture output                       |
| `censor f m`    | `RWS::censor`     | Transform output                     |
| `get`           | `RWS::get`        | Get state                            |
| `put s`         | `RWS::put`        | Set state                            |
| `modify f`      | `RWS::modify`     | Modify state                         |
| `gets f`        | `RWS::gets`       | Project from state                   |

#### Code Examples

```haskell
-- Haskell
import Control.Monad.RWS

data Config = Config { configMultiplier :: Int }
type AppState = Int
type AppLog = [String]

type App a = RWS Config AppLog AppState a

computation :: App Int
computation = do
  config <- ask
  state <- get
  let result = state * configMultiplier config
  put result
  tell [show state ++ " * " ++ show (configMultiplier config) ++ " = " ++ show result]
  return result

(result, finalState, log) :: (Int, Int, [String])
(result, finalState, log) = runRWS computation (Config 3) 10
-- result = 30
-- finalState = 30
-- log = ["10 * 3 = 30"]
```

```rust
// lambars
use lambars::effect::RWS;
use lambars::eff;

#[derive(Clone)]
struct Config { multiplier: i32 }
type AppState = i32;
type AppLog = Vec<String>;

type App<A> = RWS<Config, AppLog, AppState, A>;

let computation: App<i32> = eff! {
    config <= RWS::ask();
    state <= RWS::get();
    let result = state * config.multiplier;
    _ <= RWS::put(result);
    _ <= RWS::tell(vec![format!("{} * {} = {}", state, config.multiplier, result)]);
    RWS::pure(result)
};

let (result, final_state, log) = computation.run(Config { multiplier: 3 }, 10);
// result = 30
// final_state = 30
// log = vec!["10 * 3 = 30"]
```

---

## Monad Transformers (mtl)

### Comparison

| Haskell (mtl)            | lambars                            | Description                         |
| ------------------------ | ---------------------------------- | ----------------------------------- |
| `StateT s m a`           | `StateT<S, M, A>`                  | State transformer                   |
| `ReaderT r m a`          | `ReaderT<R, M, A>`                 | Reader transformer                  |
| `WriterT w m a`          | `WriterT<W, M, A>`                 | Writer transformer                  |
| `ExceptT e m a`          | `ExceptT<E, M, A>`                 | Exception transformer               |
| `MaybeT m a`             | Custom                             | Maybe transformer                   |
| `lift`                   | `lift_*` methods                   | Lift into transformer               |
| `liftIO`                 | `lift_io`, `lift_async_io`         | Lift IO/AsyncIO                     |
| `MonadState`             | `MonadState` trait                 | State abstraction                   |
| `MonadReader`            | `MonadReader` trait                | Reader abstraction                  |
| `MonadWriter`            | `MonadWriter` trait                | Writer abstraction                  |
| `MonadError`             | `MonadError` trait                 | Error abstraction                   |
| `throwError`             | `MonadError::throw_error`          | Throw an error                      |
| `catchError`             | `MonadError::catch_error`          | Catch and handle errors             |
| `liftEither`             | `MonadError::from_result`          | Lift Either/Result                  |
| `handleError`            | `MonadError::handle_error`         | Convert error to success value      |
| (custom)                 | `MonadError::adapt_error`          | Transform error in same type        |
| (custom)                 | `MonadError::recover`              | Partial function recovery           |
| (custom)                 | `MonadError::recover_with_partial` | Monadic partial recovery            |
| (custom)                 | `MonadError::ensure`               | Validate with predicate             |
| (custom)                 | `MonadError::ensure_or`            | Validate with value-dependent error |
| (custom)                 | `MonadError::redeem`               | Transform both success and error    |
| (custom)                 | `MonadError::redeem_with`          | Monadic redeem                      |
| (custom)                 | `MonadErrorExt::map_error`         | Transform error type                |
| Async IO in transformers | `*_async_io` methods               | AsyncIO support in transformers     |

### Code Examples

```haskell
-- Haskell (mtl)
import Control.Monad.State
import Control.Monad.Reader
import Control.Monad.Except

data AppConfig = AppConfig { configMaxRetries :: Int }
type AppState = Int  -- retry count
type AppError = String

type App a = ExceptT AppError (StateT AppState (Reader AppConfig)) a

runApp :: App a -> AppConfig -> AppState -> Either AppError (a, AppState)
runApp app config state = runReader (runStateT (runExceptT app) state) config

incrementRetries :: App ()
incrementRetries = modify (+1)

getMaxRetries :: App Int
getMaxRetries = asks configMaxRetries

checkRetries :: App ()
checkRetries = do
  current <- get
  maxR <- getMaxRetries
  when (current >= maxR) $ throwError "Max retries exceeded"

computation :: App String
computation = do
  incrementRetries
  incrementRetries
  checkRetries
  pure "Success"

result :: Either String (String, Int)
result = runApp computation (AppConfig 3) 0
-- result = Right ("Success", 2)
```

```rust
// lambars
use lambars::effect::{ExceptT, StateT, ReaderT, MonadState, MonadReader, MonadError};

#[derive(Clone)]
struct AppConfig { max_retries: i32 }
type AppState = i32;
type AppError = String;

// Due to Rust's type system, we work with concrete transformer stacks
// Here's a simplified example with ReaderT over Result

type App<A> = ReaderT<AppConfig, Result<A, AppError>>;

fn get_max_retries() -> App<i32> {
    ReaderT::asks_result(|c: &AppConfig| Ok(c.max_retries))
}

fn check_value(current: i32) -> App<()> {
    ReaderT::ask_result().flat_map_result(move |config| {
        if current >= config.max_retries {
            ReaderT::lift_result(Err("Max retries exceeded".to_string()))
        } else {
            ReaderT::pure_result(())
        }
    })
}

let computation = get_max_retries()
    .flat_map_result(|max| {
        check_value(2).then_result(ReaderT::pure_result(format!("Max is {}", max)))
    });

let result = computation.run_result(AppConfig { max_retries: 3 });
// result = Ok("Max is 3")
```

### AsyncIO Support in Transformers

lambars provides AsyncIO integration for monad transformers, enabling async operations within transformer stacks.

```rust
// lambars - ReaderT with AsyncIO
use lambars::effect::{ReaderT, AsyncIO};

#[derive(Clone)]
struct Config { api_url: String }

// ReaderT over AsyncIO
type AppAsync<A> = ReaderT<Config, AsyncIO<A>>;

fn get_url() -> AppAsync<String> {
    ReaderT::asks_async_io(|c: &Config| c.api_url.clone())
}

async fn example() {
    let computation = get_url()
        .flat_map_async_io(|url| ReaderT::pure_async_io(format!("Fetching: {}", url)));

    let config = Config { api_url: "https://api.example.com".to_string() };
    let result = computation.run_async_io(config).run_async().await;
    // result = "Fetching: https://api.example.com"
}
```

Available AsyncIO methods for transformers:

- `ReaderT`: `ask_async_io`, `asks_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `StateT`: `get_async_io`, `gets_async_io`, `state_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `WriterT`: `tell_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`, `listen_async_io`

---

## Algebraic Effects

lambars provides an algebraic effects system as an alternative to monad transformers. This approach, inspired by libraries like Polysemy (Haskell), Eff (Scala/OCaml), and freer-simple, solves the n^2 problem of monad transformers.

### Comparison with Haskell Effect Libraries

| Haskell (freer-simple/polysemy) | lambars                                    | Description                  |
| ------------------------------- | ------------------------------------------ | ---------------------------- |
| `Eff '[e1, e2] a`               | `Eff<EffCons<E1, EffCons<E2, EffNil>>, A>` | Effect computation type      |
| `Member e r`                    | `Member<E, Index>`                         | Effect membership constraint |
| `run`                           | `Handler::run`                             | Run handler                  |
| `runReader`                     | `ReaderHandler::run`                       | Run Reader effect            |
| `runState`                      | `StateHandler::run`                        | Run State effect             |
| `runWriter`                     | `WriterHandler::run`                       | Run Writer effect            |
| `runError`                      | `ErrorHandler::run`                        | Run Error effect             |
| `send` / `embed`                | `perform_raw`                              | Perform effect operation     |
| `interpret`                     | `Handler` trait impl                       | Define handler               |
| `reinterpret`                   | Handler composition                        | Transform effects            |

### Effect Row and Member

```haskell
-- Haskell (freer-simple)
import Control.Monad.Freer
import Control.Monad.Freer.Reader
import Control.Monad.Freer.State

computation :: (Member (Reader String) r, Member (State Int) r) => Eff r Int
computation = do
  env <- ask
  s <- get
  put (s + length env)
  return (s + 1)

result :: (Int, Int)
result = run $ runState 10 $ runReader "hello" computation
-- result = (11, 15)
```

```rust
// lambars
use lambars::effect::algebraic::{
    Eff, ReaderEffect, ReaderHandler, StateEffect, StateHandler,
    Member, Here, There, EffectRow, ask, get, put,
};

type MyEffects = EffectRow!(ReaderEffect<String>, StateEffect<i32>);

fn computation() -> Eff<MyEffects, i32> {
    ask::<String, MyEffects, Here>()
        .flat_map(|env| {
            get::<i32, MyEffects, There<Here>>()
                .flat_map(move |s| {
                    put::<i32, MyEffects, There<Here>>(s + env.len() as i32)
                        .then(Eff::pure(s + 1))
                })
        })
}

let eff = computation();
let with_reader = ReaderHandler::new("hello".to_string()).run(eff);
let (result, final_state) = StateHandler::new(10).run(with_reader);
// result = 11, final_state = 15
```

### Standard Effects

| Effect | Haskell                    | lambars                           | Description           |
| ------ | -------------------------- | --------------------------------- | --------------------- |
| Reader | `ask`, `asks`, `local`     | `ask()`, `asks()`, `run_local()`  | Read-only environment |
| State  | `get`, `put`, `modify`     | `get()`, `put()`, `modify()`      | Mutable state         |
| Writer | `tell`, `listen`, `censor` | `tell()`, `listen()`              | Accumulating output   |
| Error  | `throwError`, `catchError` | `throw()`, `catch()`, `attempt()` | Error handling        |

### Defining Custom Effects

```haskell
-- Haskell (freer-simple with TH)
data Log r where
  LogMsg :: String -> Log ()

makeEffect ''Log

-- Or manually
logMsg :: Member Log r => String -> Eff r ()
logMsg msg = send (LogMsg msg)

-- Handler
runLog :: Eff (Log ': r) a -> Eff r (a, [String])
runLog = handleRelayS [] handler pure
  where
    handler logs (LogMsg msg) k = k (msg : logs) ()
```

```rust
// lambars
use lambars::define_effect;
use lambars::effect::algebraic::{Effect, Eff};

define_effect! {
    /// Logging effect
    effect Log {
        /// Log a message
        fn log_message(message: String) -> ();
    }
}

// The macro generates:
// - LogEffect struct implementing Effect
// - LogEffect::log_message(message) -> Eff<LogEffect, ()>
// - LogHandler trait with fn log_message(&mut self, message: String) -> ()

// Create a computation using the effect
fn log_computation() -> Eff<LogEffect, i32> {
    LogEffect::log_message("Starting".to_string())
        .then(LogEffect::log_message("Processing".to_string()))
        .then(Eff::pure(42))
}
```

### Key Differences from Monad Transformers

| Aspect          | Monad Transformers                            | Algebraic Effects                |
| --------------- | --------------------------------------------- | -------------------------------- |
| n^2 problem     | Yes (n effects need n^2 lift implementations) | No (effects compose freely)      |
| Effect order    | Fixed by transformer stack                    | Flexible (handle in any order)   |
| Performance     | Good (specialized code)                       | Good (continuation-based)        |
| Type complexity | Can become verbose                            | Uses type-level indices          |
| Lift operations | Required (`lift`, `liftIO`)                   | Not needed (`Member` constraint) |

### When to Use Which

| Scenario                 | Recommendation                     |
| ------------------------ | ---------------------------------- |
| Simple 2-3 effect stacks | Monad Transformers (simpler types) |
| Many effects (4+)        | Algebraic Effects (no n^2 problem) |
| Effect reordering needed | Algebraic Effects                  |
| Maximum performance      | Monad Transformers                 |
| Extensible effects       | Algebraic Effects                  |
| Existing mtl codebase    | Monad Transformers (compatibility) |

---

## Data Structures

### Lists and Sequences

| Haskell              | lambars                          | Description                                |
| -------------------- | -------------------------------- | ------------------------------------------ |
| `[a]` (List)         | `PersistentList<A>`              | Immutable list                             |
| `x : xs`             | `PersistentList::cons`           | Prepend                                    |
| `head xs`            | `PersistentList::head`           | First element                              |
| `tail xs`            | `PersistentList::tail`           | Rest of list                               |
| `xs ++ ys`           | `Semigroup::combine`             | Concatenate                                |
| `length xs`          | `Foldable::length`               | Length                                     |
| `null xs`            | `Foldable::is_empty`             | Empty check                                |
| `reverse xs`         | `PersistentList::reverse`        | Reverse                                    |
| `take n xs`          | `PersistentList::take`           | Take first n elements                      |
| `drop n xs`          | `PersistentList::drop_first`     | Drop first n elements                      |
| `splitAt n xs`       | `PersistentList::split_at`       | Split at index                             |
| `zip xs ys`          | `PersistentList::zip`            | Zip two lists                              |
| `unzip xs`           | `PersistentList::<(A,B)>::unzip` | Unzip list of pairs                        |
| `findIndex p xs`     | `PersistentList::find_index`     | Find index of first match                  |
| `foldl1 f xs`        | `PersistentList::fold_left1`     | Left fold without initial value            |
| `foldr1 f xs`        | `PersistentList::fold_right1`    | Right fold without initial value           |
| `scanl f z xs`       | `PersistentList::scan_left`      | Left scan with initial value               |
| `partition p xs`     | `PersistentList::partition`      | Split by predicate                         |
| `intersperse x xs`   | `PersistentList::intersperse`    | Insert between elements                    |
| `intercalate xs xss` | `PersistentList::intercalate`    | Insert list between lists and flatten      |
| `compare xs ys`      | `Ord::cmp`                       | Lexicographic ordering (requires `T: Ord`) |

### Vectors

| Haskell           | lambars                            | Description                                |
| ----------------- | ---------------------------------- | ------------------------------------------ |
| `Data.Vector`     | `PersistentVector<A>`              | Immutable vector                           |
| `V.!`             | `PersistentVector::get`            | Index access                               |
| `V.//`            | `PersistentVector::update`         | Update element                             |
| `V.snoc`          | `PersistentVector::push_back`      | Append                                     |
| `V.length`        | `PersistentVector::len`            | Length                                     |
| `V.take n v`      | `PersistentVector::take`           | Take first n elements                      |
| `V.drop n v`      | `PersistentVector::drop_first`     | Drop first n elements                      |
| `V.splitAt n v`   | `PersistentVector::split_at`       | Split at index                             |
| `V.zip v1 v2`     | `PersistentVector::zip`            | Zip two vectors                            |
| `V.unzip v`       | `PersistentVector::<(A,B)>::unzip` | Unzip vector of pairs                      |
| `V.findIndex p v` | `PersistentVector::find_index`     | Find index of first match                  |
| `V.foldl1 f v`    | `PersistentVector::fold_left1`     | Left fold without initial value            |
| `V.foldr1 f v`    | `PersistentVector::fold_right1`    | Right fold without initial value           |
| `V.scanl f z v`   | `PersistentVector::scan_left`      | Left scan with initial value               |
| `V.partition p v` | `PersistentVector::partition`      | Split by predicate                         |
| (N/A)             | `PersistentVector::intersperse`    | Insert between elements                    |
| (N/A)             | `PersistentVector::intercalate`    | Insert vector between vectors and flatten  |
| `compare v1 v2`   | `Ord::cmp`                         | Lexicographic ordering (requires `T: Ord`) |

### Maps

| Haskell               | lambars                   | Description           |
| --------------------- | ------------------------- | --------------------- |
| `Data.Map`            | `PersistentTreeMap<K, V>` | Ordered map           |
| `Data.HashMap`        | `PersistentHashMap<K, V>` | Hash map              |
| `M.insert k v m`      | `insert` method           | Insert                |
| `M.lookup k m`        | `get` method              | Lookup                |
| `M.delete k m`        | `remove` method           | Delete                |
| `M.member k m`        | `contains_key` method     | Membership            |
| `M.map f m`           | `map_values` method       | Transform values      |
| `M.mapKeys f m`       | `map_keys` method         | Transform keys        |
| `M.mapMaybe f m`      | `filter_map` method       | Filter and transform  |
| `M.toList m`          | `entries` method          | Get all entries       |
| `M.keys m`            | `keys` method             | Get all keys          |
| `M.elems m`           | `values` method           | Get all values        |
| `M.union m1 m2`       | `merge` method            | Merge (right wins)    |
| `M.unionWith f m1 m2` | `merge_with` method       | Merge with resolver   |
| `M.filter p m`        | `keep_if` method          | Keep matching entries |
| `M.filterWithKey p m` | `keep_if` method          | Keep matching entries |
| `M.partition p m`     | `partition` method        | Split by predicate    |

### Sets

| Haskell          | lambars                | Description  |
| ---------------- | ---------------------- | ------------ |
| `Data.Set`       | `PersistentHashSet<A>` | Set          |
| `S.insert x s`   | `insert` method        | Insert       |
| `S.member x s`   | `contains` method      | Membership   |
| `S.union s1 s2`  | `union` method         | Union        |
| `S.intersection` | `intersection` method  | Intersection |
| `S.difference`   | `difference` method    | Difference   |

### Code Examples

```haskell
-- Haskell
import qualified Data.Map as M
import qualified Data.Set as S

-- List operations
list :: [Int]
list = 1 : 2 : 3 : []

headElem :: Int
headElem = head list
-- headElem = 1

tailList :: [Int]
tailList = tail list
-- tailList = [2, 3]

-- Map operations
map1 :: M.Map String Int
map1 = M.fromList [("one", 1), ("two", 2)]

updated :: M.Map String Int
updated = M.insert "three" 3 map1

value :: Maybe Int
value = M.lookup "one" map1
-- value = Just 1

-- Set operations
set1 :: S.Set Int
set1 = S.fromList [1, 2, 3]

set2 :: S.Set Int
set2 = S.fromList [2, 3, 4]

unionSet :: S.Set Int
unionSet = S.union set1 set2
-- unionSet = {1, 2, 3, 4}

intersectionSet :: S.Set Int
intersectionSet = S.intersection set1 set2
-- intersectionSet = {2, 3}
```

```rust
// lambars
use lambars::persistent::{PersistentList, PersistentHashMap, PersistentHashSet, PersistentTreeMap};

// List operations
let list = PersistentList::new().cons(3).cons(2).cons(1);

let head_elem: Option<&i32> = list.head();
// head_elem = Some(&1)

let tail_list: Option<PersistentList<i32>> = list.tail();
// tail_list = Some(PersistentList [2, 3])

// HashMap operations
let map1 = PersistentHashMap::new()
    .insert("one".to_string(), 1)
    .insert("two".to_string(), 2);

let updated = map1.insert("three".to_string(), 3);

let value: Option<&i32> = map1.get("one");
// value = Some(&1)

// HashSet operations
let set1: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
let set2: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();

let union_set = set1.union(&set2);
// union_set contains {1, 2, 3, 4}

let intersection_set = set1.intersection(&set2);
// intersection_set contains {2, 3}

// HashSetView - lazy evaluation (similar to Haskell's lazy semantics)
let result: PersistentHashSet<i32> = set1
    .view()
    .filter(|x| *x % 2 == 1)
    .map(|x| x * 10)
    .collect();
// result contains {10, 30}
```

---

## Pattern Matching

| Haskell              | Rust               | Description      |
| -------------------- | ------------------ | ---------------- | ------ |
| `case x of ...`      | `match x { ... }`  | Match expression |
| `_ -> ...`           | `_ => ...`         | Wildcard         |
| `x@pattern`          | `x @ pattern`      | As-pattern       |
| `(a, b)`             | `(a, b)`           | Tuple pattern    |
| `Just x`             | `Some(x)`          | Maybe/Option     |
| `Left e` / `Right a` | `Err(e)` / `Ok(a)` | Either/Result    |
| `[]`                 | `[]` or `vec![]`   | Empty list       |
| `x:xs`               | Custom             | Cons pattern     |
| Guards `             | cond`              | `if cond =>`     | Guards |

### Code Examples

```haskell
-- Haskell
describeNumber :: Int -> String
describeNumber n = case n of
  0 -> "zero"
  1 -> "one"
  x | x < 0 -> "negative"
    | x > 100 -> "large"
    | otherwise -> "other"

-- Maybe pattern matching
describeMaybe :: Maybe Int -> String
describeMaybe m = case m of
  Nothing -> "nothing"
  Just 0 -> "zero"
  Just n -> "some: " ++ show n

-- List pattern matching
describeList :: [a] -> String
describeList xs = case xs of
  [] -> "empty"
  [_] -> "singleton"
  [_, _] -> "pair"
  _ -> "many"

-- As-patterns
duplicateFirst :: [a] -> [a]
duplicateFirst xs@(x:_) = x : xs
duplicateFirst [] = []
```

```rust
// Rust
fn describe_number(n: i32) -> String {
    match n {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        x if x < 0 => "negative".to_string(),
        x if x > 100 => "large".to_string(),
        _ => "other".to_string(),
    }
}

// Option pattern matching
fn describe_option(m: Option<i32>) -> String {
    match m {
        None => "nothing".to_string(),
        Some(0) => "zero".to_string(),
        Some(n) => format!("some: {}", n),
    }
}

// Slice pattern matching
fn describe_slice<T>(xs: &[T]) -> String {
    match xs {
        [] => "empty".to_string(),
        [_] => "singleton".to_string(),
        [_, _] => "pair".to_string(),
        _ => "many".to_string(),
    }
}

// As-patterns
fn duplicate_first<T: Clone>(xs: &[T]) -> Vec<T> {
    match xs {
        [] => vec![],
        [first, rest @ ..] => {
            let mut result = vec![first.clone()];
            result.push(first.clone());
            result.extend(rest.iter().cloned());
            result
        }
    }
}
```

---

## Higher-Kinded Types

Haskell natively supports higher-kinded types (HKT), while Rust does not. lambars uses Generic Associated Types (GAT) to emulate HKT.

### Comparison

```haskell
-- Haskell - Native HKT
class Functor f where
  fmap :: (a -> b) -> f a -> f b

-- 'f' is a type constructor of kind * -> *
-- This allows abstracting over Option, List, Either e, etc.
```

```rust
// lambars - HKT Emulation via GAT
pub trait TypeConstructor {
    type Inner;
    type WithType<B>: TypeConstructor<Inner = B>;
}

pub trait Functor: TypeConstructor {
    fn fmap<B, F>(self, f: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> B;
}

// TypeConstructor allows changing the type parameter
// While not as elegant as Haskell's HKT, it enables similar abstractions
```

### Limitations

1. **No direct kind polymorphism**: Rust can't express `* -> *` as a type parameter
2. **More verbose**: Trait bounds become complex with nested type constructors
3. **Limited inference**: Type annotations often required
4. **Specific implementations**: Some operations (like `traverse`) need type-specific variants

---

## Algebraic Data Types

Both Haskell and Rust support algebraic data types, with similar but syntactically different definitions.

### Sum Types (Enums)

```haskell
-- Haskell
data Maybe a = Nothing | Just a

data Either e a = Left e | Right a

data Tree a = Leaf a | Branch (Tree a) (Tree a)

data List a = Nil | Cons a (List a)
```

```rust
// Rust
enum Option<A> {  // std
    None,
    Some(A),
}

enum Result<A, E> {  // std
    Ok(A),
    Err(E),
}

enum Tree<A> {
    Leaf(A),
    Branch(Box<Tree<A>>, Box<Tree<A>>),
}

enum List<A> {
    Nil,
    Cons(A, Box<List<A>>),
}
```

### Product Types (Structs/Records)

```haskell
-- Haskell
data Person = Person
  { name :: String
  , age :: Int
  , email :: String
  }

-- Record syntax provides automatic getters
getName :: Person -> String
getName = name
```

```rust
// Rust
struct Person {
    name: String,
    age: i32,
    email: String,
}

// Field access is direct
fn get_name(p: &Person) -> &str {
    &p.name
}
```

### Newtype

```haskell
-- Haskell
newtype UserId = UserId Int
  deriving (Eq, Ord, Show)

newtype Email = Email String
  deriving (Eq, Show)
```

```rust
// Rust
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct UserId(i32);

#[derive(Clone, PartialEq, Eq, Debug)]
struct Email(String);
```

---

## Summary: Key Differences

### Syntax Mapping

| Haskell              | Rust (lambars)                  |
| -------------------- | ------------------------------- |
| `f x`                | `f(x)`                          |
| `f $ x`              | `f(x)`                          |
| `x & f`              | `pipe!(x, f)`                   |
| `f . g`              | `compose!(f, g)`                |
| `do { x <- m; ... }` | `eff! { x <= m; ... }`          |
| `\x -> x + 1`        | `\|x\| x + 1`                   |
| `x :: Int`           | `x: i32`                        |
| `[a]`                | `Vec<A>` or `PersistentList<A>` |
| `Maybe a`            | `Option<A>`                     |
| `Either e a`         | `Result<A, E>`                  |
| `IO a`               | `IO<A>`                         |
| `pure x`             | `Applicative::pure(x)`          |
| `m >>= f`            | `m.flat_map(f)`                 |
| `fmap f m`           | `m.fmap(f)`                     |

### Conceptual Differences

1. **Laziness**: Haskell is lazy by default; Rust is strict (use `Lazy` for explicit laziness)

2. **Purity**: Haskell enforces purity via IO; Rust allows side effects anywhere (use IO monad for discipline)

3. **Higher-Kinded Types**: Haskell has native HKT; Rust emulates via GAT

4. **Type Classes vs Traits**: Similar concepts, but Haskell's are more flexible with orphan instances

5. **Currying**: Haskell functions are curried by default; Rust requires explicit currying

6. **Memory Management**: Haskell uses GC; Rust uses ownership/borrowing

7. **Pattern Matching**: Both support it, but Rust requires explicit handling of all cases

8. **Inference**: Haskell has global type inference; Rust has local inference with occasional annotations needed
