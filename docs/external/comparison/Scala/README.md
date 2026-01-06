# Scala to lambars API Comparison Guide

This document provides a comprehensive comparison between Scala functional programming constructs (including Cats and Scalaz libraries) and their equivalents in lambars (Rust).

## Table of Contents

- [Overview](#overview)
- [Type Classes](#type-classes)
  - [Functor](#functor)
  - [Applicative](#applicative)
  - [Monad](#monad)
  - [Semigroup and Monoid](#semigroup-and-monoid)
  - [Foldable](#foldable)
  - [Traversable](#traversable)
- [Option and Either](#option-and-either)
- [For-Comprehensions vs eff! Macro](#for-comprehensions-vs-eff-macro)
- [Function Composition](#function-composition)
- [Optics (Monocle)](#optics-monocle)
- [Effect Systems](#effect-systems)
- [Monad Transformers](#monad-transformers)
- [Persistent Collections](#persistent-collections)
- [Lazy Evaluation](#lazy-evaluation)
- [Implicits vs Traits](#implicits-vs-traits)

---

## Overview

| Concept | Scala (Cats) | lambars (Rust) |
|---------|--------------|----------------|
| Functor | `Functor[F]` | `Functor` trait |
| Applicative | `Applicative[F]` | `Applicative` trait |
| Monad | `Monad[F]` | `Monad` trait |
| Semigroup | `Semigroup[A]` | `Semigroup` trait |
| Monoid | `Monoid[A]` | `Monoid` trait |
| Foldable | `Foldable[F]` | `Foldable` trait |
| Traverse | `Traverse[F]` | `Traversable` trait |
| Option | `Option[A]` | `Option<A>` (std) |
| Either | `Either[E, A]` | `Result<A, E>` (std) |
| For-comprehension (Monad) | `for { ... } yield` | `eff!` macro |
| For-comprehension (List) | `for { ... } yield` (List) | `for_!` macro |
| Async for-comprehension | `for { ... } yield` + `IO` | `for_async!` macro |
| Lens | `monocle.Lens` | `Lens` trait |
| Prism | `monocle.Prism` | `Prism` trait |
| IO | `cats.effect.IO` | `IO` type |
| State | `cats.data.State` | `State` type |
| Reader | `cats.data.Reader` | `Reader` type |
| Writer | `cats.data.Writer` | `Writer` type |
| StateT | `cats.data.StateT` | `StateT` type |
| ReaderT | `cats.data.ReaderT` | `ReaderT` type |
| WriterT | `cats.data.WriterT` | `WriterT` type |
| EitherT | `cats.data.EitherT` | `ExceptT` type |

---

## Type Classes

### Functor

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `F[A].map(f)` | `Functor::fmap` | Transform inner value |
| `F[A].as(b)` | `fmap(\|_\| b)` | Replace with constant |
| `F[A].void` | `fmap(\|_\| ())` | Discard value |
| `F[A].fproduct(f)` | `fmap(\|a\| (a.clone(), f(a)))` | Pair with function result |
| `Functor[F].lift(f)` | Manual implementation | Lift function to functor |

#### Code Examples

```scala
// Scala (Cats)
import cats.Functor
import cats.syntax.functor._

val doubled: Option[Int] = Some(21).map(_ * 2)
// doubled = Some(42)

val list: List[Int] = List(1, 2, 3).map(_ * 2)
// list = List(2, 4, 6)

// Using Functor type class
def doubleF[F[_]: Functor](fa: F[Int]): F[Int] =
  Functor[F].map(fa)(_ * 2)
```

```rust
// lambars
use lambars::typeclass::Functor;

let doubled: Option<i32> = Some(21).fmap(|x| x * 2);
// doubled = Some(42)

let list: Vec<i32> = vec![1, 2, 3].fmap(|x| x * 2);
// list = vec![2, 4, 6]

// Using Functor trait
fn double_f<F: Functor<Inner = i32>>(fa: F) -> F::WithType<i32>
where
    F::WithType<i32>: Functor,
{
    fa.fmap(|x| x * 2)
}
```

### Applicative

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `A.pure[F]` | `Applicative::pure` | Lift value into context |
| `(fa, fb).mapN(f)` | `Applicative::map2` | Combine with function |
| `(fa, fb).tupled` | `Applicative::product` | Combine into tuple |
| `fa.ap(ff)` | `Applicative::apply` | Apply wrapped function |
| `fa *> fb` | `fa.map2(fb, \|_, b\| b)` | Sequence, keep right |
| `fa <* fb` | `fa.map2(fb, \|a, _\| a)` | Sequence, keep left |

#### Code Examples

```scala
// Scala (Cats)
import cats.Applicative
import cats.syntax.applicative._
import cats.syntax.apply._

val x: Option[Int] = 42.pure[Option]
// x = Some(42)

val sum: Option[Int] = (Some(1), Some(2)).mapN(_ + _)
// sum = Some(3)

val product: Option[(Int, String)] = (Some(1), Some("hello")).tupled
// product = Some((1, "hello"))

// map3
val sum3: Option[Int] = (Some(1), Some(2), Some(3)).mapN(_ + _ + _)
// sum3 = Some(6)
```

```rust
// lambars
use lambars::typeclass::Applicative;

let x: Option<i32> = <Option<()>>::pure(42);
// x = Some(42)

let sum: Option<i32> = Some(1).map2(Some(2), |a, b| a + b);
// sum = Some(3)

let product: Option<(i32, String)> = Some(1).product(Some("hello".to_string()));
// product = Some((1, "hello".to_string()))

// map3
let sum3: Option<i32> = Some(1).map3(Some(2), Some(3), |a, b, c| a + b + c);
// sum3 = Some(6)
```

### Monad

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `fa.flatMap(f)` | `Monad::flat_map` | Chain computations |
| `fa.flatten` | `Option::flatten` (std) | Flatten nested |
| `fa >> fb` | `Monad::then` | Sequence, keep right |
| `fa.mproduct(f)` | `flat_map(\|a\| f(a).map(\|b\| (a, b)))` | Pair with result |
| `fa.ifM(ifTrue, ifFalse)` | Manual implementation | Conditional |
| `Monad[F].whileM_` | Manual implementation | While loop |
| `Monad[F].iterateWhile` | Manual implementation | Iterate with condition |

#### Code Examples

```scala
// Scala (Cats)
import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._

def safeDivide(x: Int, y: Int): Option[Int] =
  if (y == 0) None else Some(x / y)

val result: Option[Int] = Some(10).flatMap(x => safeDivide(x, 2))
// result = Some(5)

// Chaining
val chained: Option[Int] = for {
  a <- Some(10)
  b <- safeDivide(a, 2)
  c <- safeDivide(b, 1)
} yield c
// chained = Some(5)

// Flatten
val nested: Option[Option[Int]] = Some(Some(42))
val flat: Option[Int] = nested.flatten
// flat = Some(42)
```

```rust
// lambars
use lambars::typeclass::Monad;

fn safe_divide(x: i32, y: i32) -> Option<i32> {
    if y == 0 { None } else { Some(x / y) }
}

let result: Option<i32> = Some(10).flat_map(|x| safe_divide(x, 2));
// result = Some(5)

// Chaining with eff! macro
use lambars::eff;

let chained: Option<i32> = eff! {
    a <= Some(10);
    b <= safe_divide(a, 2);
    c <= safe_divide(b, 1);
    Some(c)
};
// chained = Some(5)

// Flatten (std)
let nested: Option<Option<i32>> = Some(Some(42));
let flat: Option<i32> = nested.flatten();
// flat = Some(42)
```

### Semigroup and Monoid

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `a \|+\| b` | `Semigroup::combine` | Combine values |
| `Monoid[A].empty` | `Monoid::empty` | Identity element |
| `Monoid[A].combineAll(list)` | `Monoid::combine_all` | Fold with combine |
| `a.combineN(n)` | Manual loop | Combine n times |
| `Semigroup.maybeCombine` | `Option::combine` | Combine options |

#### Code Examples

```scala
// Scala (Cats)
import cats.Monoid
import cats.syntax.semigroup._

val combined: String = "Hello, " |+| "World!"
// combined = "Hello, World!"

val sum: Int = Monoid[Int].combineAll(List(1, 2, 3, 4, 5))
// sum = 15

// Custom monoid for product
import cats.kernel.instances.int._
val product: Int = List(1, 2, 3, 4, 5).foldLeft(1)(_ * _)
// product = 120

// Using Sum/Product wrappers
import cats.data.{Ior}
```

```rust
// lambars
use lambars::typeclass::{Semigroup, Monoid, Sum, Product};

let combined: String = "Hello, ".to_string().combine("World!".to_string());
// combined = "Hello, World!"

let items = vec![Sum::new(1), Sum::new(2), Sum::new(3), Sum::new(4), Sum::new(5)];
let sum: Sum<i32> = Sum::combine_all(items);
// sum = Sum(15)

// Product monoid
let items = vec![Product::new(1), Product::new(2), Product::new(3), Product::new(4), Product::new(5)];
let product: Product<i32> = Product::combine_all(items);
// product = Product(120)

// Vec combination
let vec1 = vec![1, 2, 3];
let vec2 = vec![4, 5, 6];
let combined: Vec<i32> = vec1.combine(vec2);
// combined = vec![1, 2, 3, 4, 5, 6]
```

### Foldable

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `fa.foldLeft(b)(f)` | `Foldable::fold_left` | Left fold |
| `fa.foldRight(lb)(f)` | `Foldable::fold_right` | Right fold |
| `fa.foldMap(f)` | `Foldable::fold_map` | Map then fold |
| `fa.fold` | `Foldable::fold` | Fold with Monoid |
| `fa.find(p)` | `Foldable::find` | Find first matching |
| `fa.exists(p)` | `Foldable::exists` | Any matches |
| `fa.forall(p)` | `Foldable::for_all` | All match |
| `fa.isEmpty` | `Foldable::is_empty` | Check empty |
| `fa.nonEmpty` | `!Foldable::is_empty` | Check non-empty |
| `fa.size` | `Foldable::length` | Count elements |
| `fa.toList` | `Foldable::to_vec` | Convert to list |

#### Code Examples

```scala
// Scala (Cats)
import cats.Foldable
import cats.syntax.foldable._

val sum: Int = List(1, 2, 3, 4, 5).foldLeft(0)(_ + _)
// sum = 15

val product: Int = List(1, 2, 3, 4, 5).foldRight(1)(_ * _)
// product = 120

// foldMap with String monoid
val concat: String = List(1, 2, 3).foldMap(_.toString)
// concat = "123"

// find
val found: Option[Int] = List(1, 2, 3, 4, 5).find(_ > 3)
// found = Some(4)

// exists and forall
val hasEven: Boolean = List(1, 2, 3).exists(_ % 2 == 0)
// hasEven = true

val allPositive: Boolean = List(1, 2, 3).forall(_ > 0)
// allPositive = true
```

```rust
// lambars
use lambars::typeclass::{Foldable, Monoid};

let sum: i32 = vec![1, 2, 3, 4, 5].fold_left(0, |acc, x| acc + x);
// sum = 15

let product: i32 = vec![1, 2, 3, 4, 5].fold_right(1, |x, acc| x * acc);
// product = 120

// fold_map with String monoid
let concat: String = vec![1, 2, 3].fold_map(|x| x.to_string());
// concat = "123"

// find
let found: Option<&i32> = vec![1, 2, 3, 4, 5].find(|x| **x > 3);
// found = Some(&4)

// exists and for_all
let has_even: bool = vec![1, 2, 3].exists(|x| x % 2 == 0);
// has_even = true

let all_positive: bool = vec![1, 2, 3].for_all(|x| *x > 0);
// all_positive = true
```

### Traversable

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `fa.traverse(f)` | `Traversable::traverse_option/result` | Traverse with effect |
| `fa.sequence` | `Traversable::sequence_option/result` | Sequence effects |
| `fa.flatTraverse(f)` | Compose with flatten | Traverse then flatten |
| `fa.traverseFilter(f)` | Manual implementation | Filter during traverse |
| `fa.traverse[Reader[R, *]](f)` | `Traversable::traverse_reader` | Traverse with Reader effect |
| `fa.traverse[State[S, *]](f)` | `Traversable::traverse_state` | Traverse with State effect |
| `fa.traverse[IO](f)` | `Traversable::traverse_io` | Traverse with IO effect |
| `fa.sequence[Reader[R, *]]` | `Traversable::sequence_reader` | Sequence Reader effects |
| `fa.sequence[State[S, *]]` | `Traversable::sequence_state` | Sequence State effects |
| `fa.sequence[IO]` | `Traversable::sequence_io` | Sequence IO effects |
| `fa.traverse_[Reader[R, *]](f)` | `Traversable::traverse_reader_` | Traverse Reader, discard result |
| `fa.traverse_[State[S, *]](f)` | `Traversable::traverse_state_` | Traverse State, discard result |
| `fa.traverse_[IO](f)` | `Traversable::traverse_io_` | Traverse IO, discard result |

#### Code Examples

```scala
// Scala (Cats)
import cats.Traverse
import cats.syntax.traverse._

def parseIntOption(s: String): Option[Int] =
  scala.util.Try(s.toInt).toOption

val result: Option[List[Int]] = List("1", "2", "3").traverse(parseIntOption)
// result = Some(List(1, 2, 3))

val failed: Option[List[Int]] = List("1", "two", "3").traverse(parseIntOption)
// failed = None

// sequence
val list: List[Option[Int]] = List(Some(1), Some(2), Some(3))
val sequenced: Option[List[Int]] = list.sequence
// sequenced = Some(List(1, 2, 3))

val withNone: List[Option[Int]] = List(Some(1), None, Some(3))
val sequencedNone: Option[List[Int]] = withNone.sequence
// sequencedNone = None
```

```rust
// lambars
use lambars::typeclass::Traversable;

fn parse_int_option(s: &str) -> Option<i32> {
    s.parse().ok()
}

let result: Option<Vec<i32>> = vec!["1", "2", "3"]
    .traverse_option(parse_int_option);
// result = Some(vec![1, 2, 3])

let failed: Option<Vec<i32>> = vec!["1", "two", "3"]
    .traverse_option(parse_int_option);
// failed = None

// sequence_option
let list: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
let sequenced: Option<Vec<i32>> = list.sequence_option();
// sequenced = Some(vec![1, 2, 3])

let with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
let sequenced_none: Option<Vec<i32>> = with_none.sequence_option();
// sequenced_none = None

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

## Option and Either

### Option

| Scala | lambars / Rust std | Description |
|-------|-------------------|-------------|
| `Some(x)` | `Some(x)` | Construct Some |
| `None` | `None` | Construct None |
| `opt.map(f)` | `Functor::fmap` | Transform value |
| `opt.flatMap(f)` | `Monad::flat_map` | Chain computation |
| `opt.getOrElse(default)` | `Option::unwrap_or` | Default value |
| `opt.orElse(alt)` | `Option::or` | Alternative |
| `opt.fold(ifEmpty)(f)` | `Option::map_or` | Fold with default |
| `opt.filter(p)` | `Option::filter` | Filter by predicate |
| `opt.filterNot(p)` | `filter(\|x\| !p(x))` | Filter inverse |
| `opt.contains(x)` | `Option::contains` | Contains value |
| `opt.exists(p)` | `Option::is_some_and` | Test with predicate |
| `opt.forall(p)` | `opt.map_or(true, p)` | All satisfy |
| `opt.toRight(left)` | `Option::ok_or` | To Either/Result |
| `opt.toLeft(right)` | `opt.ok_or(right).swap()` | To Either (left) |
| `opt.zip(other)` | `Option::zip` | Zip two options |

### Either / Result

| Scala | lambars / Rust std | Description |
|-------|-------------------|-------------|
| `Right(x)` | `Ok(x)` | Right/Ok value |
| `Left(e)` | `Err(e)` | Left/Error value |
| `either.map(f)` | `Functor::fmap` / `Result::map` | Map right |
| `either.leftMap(f)` | `Result::map_err` | Map left |
| `either.flatMap(f)` | `Monad::flat_map` | Chain |
| `either.bimap(f, g)` | Manual | Map both sides |
| `either.fold(f, g)` | `Result::map_or_else` | Fold both |
| `either.swap` | `Result::swap` (nightly) | Swap sides |
| `either.toOption` | `Result::ok` | To Option |
| `either.getOrElse(d)` | `Result::unwrap_or` | Default |
| `either.orElse(alt)` | `Result::or` | Alternative |

#### Code Examples

```scala
// Scala
val opt: Option[Int] = Some(42)
val result: Option[Int] = opt.map(_ * 2).filter(_ > 50)
// result = Some(84)

val either: Either[String, Int] = Right(42)
val mapped: Either[String, Int] = either.map(_ * 2)
// mapped = Right(84)

val biMapped: Either[Int, String] = either.bimap(_.length, _.toString)
// biMapped = Right("42")
```

```rust
// lambars / std
use lambars::typeclass::{Functor, Monad};

let opt: Option<i32> = Some(42);
let result: Option<i32> = opt.fmap(|x| x * 2).filter(|x| *x > 50);
// result = Some(84)

let either: Result<i32, String> = Ok(42);
let mapped: Result<i32, String> = either.fmap(|x| x * 2);
// mapped = Ok(84)

// bimap equivalent
let either: Result<i32, String> = Ok(42);
let bi_mapped: Result<String, usize> = either
    .map(|x| x.to_string())
    .map_err(|e| e.len());
// bi_mapped = Ok("42")
```

---

## For-Comprehensions vs eff! / for_! Macros

lambars provides two macros that correspond to Scala's for-comprehension:

| Use Case | Scala | lambars | Description |
|----------|-------|---------|-------------|
| Monad binding (Option, Result, IO, State, etc.) | `for { x <- mx } yield x` | `eff!` macro | Single execution, FnOnce-based |
| List comprehension (Vec, iterators) | `for { x <- xs } yield f(x)` | `for_!` macro | Multiple execution, FnMut-based |

### Key Differences: `eff!` vs `for_!`

| Aspect | `eff!` | `for_!` |
|--------|--------|---------|
| **Target types** | Option, Result, IO, State, Reader, Writer | Vec, Iterator |
| **Execution** | Single execution (FnOnce) | Multiple executions (FnMut) |
| **Final expression** | Must return wrapped value | Uses `yield` keyword |
| **Closure type** | `move` closures | Regular closures |
| **Typical use** | Monadic computation chaining | List comprehensions |

### Syntax Comparison

#### eff! Macro (for Monads)

| Scala | lambars | Description |
|-------|---------|-------------|
| `for { x <- mx } yield x` | `eff! { x <= mx; mx2 }` | Basic bind |
| `for { x <- mx; y <- my } yield (x, y)` | `eff! { x <= mx; y <= my; expr }` | Multiple binds |
| `for { x <- mx; if p(x) } yield x` | `eff! { x <= mx.filter(p); Some(x) }` | Guard (Option) |
| `x = expr` (in for) | `let x = expr;` | Pure binding |

#### for_! Macro (for Lists)

| Scala | lambars | Description |
|-------|---------|-------------|
| `for { x <- xs } yield f(x)` | `for_! { x <= xs; yield f(x) }` | Basic list comprehension |
| `for { x <- xs; y <- ys } yield (x, y)` | `for_! { x <= xs; y <= ys.clone(); yield (x, y) }` | Nested iteration |
| `for { x <- xs; if p(x) } yield x` | `xs.into_iter().filter(p).collect()` | Filtering (use std) |
| `x = expr` (in for) | `let x = expr;` | Pure binding |

**Important**: In `for_!`, inner collections typically need `.clone()` due to Rust's ownership rules.

### Code Examples

```scala
// Scala - for-comprehension
val result: Option[Int] = for {
  x <- Some(10)
  y <- Some(20)
  z = x + y
} yield z * 2
// result = Some(60)

// With Either
val computation: Either[String, Int] = for {
  a <- Right(10)
  b <- Right(20)
  _ <- if (b > 0) Right(()) else Left("b must be positive")
} yield a + b
// computation = Right(30)

// Nested for-comprehension
val nested: Option[Int] = for {
  list <- Some(List(1, 2, 3))
  first <- list.headOption
  doubled = first * 2
} yield doubled
// nested = Some(2)
```

```rust
// lambars - eff! macro
use lambars::eff;

let result: Option<i32> = eff! {
    x <= Some(10);
    y <= Some(20);
    let z = x + y;
    Some(z * 2)
};
// result = Some(60)

// With Result
let computation: Result<i32, String> = eff! {
    a <= Ok::<i32, String>(10);
    b <= Ok::<i32, String>(20);
    _ <= if b > 0 { Ok(()) } else { Err("b must be positive".to_string()) };
    Ok(a + b)
};
// computation = Ok(30)

// Nested computation
let nested: Option<i32> = eff! {
    list <= Some(vec![1, 2, 3]);
    first <= list.first().copied();
    let doubled = first * 2;
    Some(doubled)
};
// nested = Some(2)
```

### Complex Examples

```scala
// Scala - Database-like operations
case class User(id: Int, name: String)
case class Order(id: Int, userId: Int, amount: Double)

def findUser(id: Int): Option[User] = ???
def findOrders(userId: Int): Option[List[Order]] = ???

val userOrders: Option[(User, List[Order])] = for {
  user <- findUser(1)
  orders <- findOrders(user.id)
} yield (user, orders)
```

```rust
// lambars
use lambars::eff;

struct User { id: i32, name: String }
struct Order { id: i32, user_id: i32, amount: f64 }

fn find_user(id: i32) -> Option<User> { /* ... */ None }
fn find_orders(user_id: i32) -> Option<Vec<Order>> { /* ... */ None }

let user_orders: Option<(User, Vec<Order>)> = eff! {
    user <= find_user(1);
    orders <= find_orders(user.id);
    Some((user, orders))
};
```

### List Comprehension with for_!

For List-based for-comprehensions, use the `for_!` macro:

```scala
// Scala - List comprehension
val numbers = List(1, 2, 3, 4, 5)
val doubled: List[Int] = for {
  n <- numbers
} yield n * 2
// doubled = List(2, 4, 6, 8, 10)

// Nested list comprehension
val xs = List(1, 2)
val ys = List(10, 20)
val cartesian: List[Int] = for {
  x <- xs
  y <- ys
} yield x + y
// cartesian = List(11, 21, 12, 22)

// Book recommendations example
case class Book(title: String, authors: List[String])
case class Movie(title: String)

val books = List(
  Book("FP in Scala", List("Chiusano", "Bjarnason")),
  Book("The Hobbit", List("Tolkien"))
)

def bookAdaptations(author: String): List[Movie] = {
  if (author == "Tolkien") List(Movie("An Unexpected Journey"), Movie("The Desolation of Smaug"))
  else List.empty
}

val recommendations: List[String] = for {
  book <- books
  author <- book.authors
  movie <- bookAdaptations(author)
} yield s"You may like ${movie.title}, because you liked ${author}'s ${book.title}"
// recommendations = List(
//   "You may like An Unexpected Journey, because you liked Tolkien's The Hobbit",
//   "You may like The Desolation of Smaug, because you liked Tolkien's The Hobbit"
// )
```

```rust
// lambars - for_! macro
use lambars::for_;

let numbers = vec![1, 2, 3, 4, 5];
let doubled: Vec<i32> = for_! {
    n <= numbers;
    yield n * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// Nested list comprehension
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // Note: clone() needed for inner iteration
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// Book recommendations example
#[derive(Clone)]
struct Book { title: String, authors: Vec<String> }

struct Movie { title: String }

fn book_adaptations(author: &str) -> Vec<Movie> {
    if author == "Tolkien" {
        vec![
            Movie { title: "An Unexpected Journey".to_string() },
            Movie { title: "The Desolation of Smaug".to_string() },
        ]
    } else {
        vec![]
    }
}

let books = vec![
    Book {
        title: "FP in Scala".to_string(),
        authors: vec!["Chiusano".to_string(), "Bjarnason".to_string()],
    },
    Book {
        title: "The Hobbit".to_string(),
        authors: vec!["Tolkien".to_string()],
    },
];

let recommendations: Vec<String> = for_! {
    book <= books;
    author <= book.authors.clone();  // Note: clone() needed
    movie <= book_adaptations(&author);
    yield format!(
        "You may like {}, because you liked {}'s {}",
        movie.title, author, book.title
    )
};
// recommendations = vec![
//     "You may like An Unexpected Journey, because you liked Tolkien's The Hobbit",
//     "You may like The Desolation of Smaug, because you liked Tolkien's The Hobbit",
// ]
```

### When to Use Each Macro

| Scenario | Recommended Macro | Reason |
|----------|-------------------|--------|
| Option/Result chaining | `eff!` | Short-circuits on None/Err |
| IO/State/Reader/Writer | `eff!` | Designed for FnOnce monads |
| List/Vec transformation | `for_!` | Supports multiple iterations |
| Cartesian products | `for_!` | Nested iteration with yield |
| Database-style queries | `eff!` | Monadic error handling |
| Data generation | `for_!` | Multiple results needed |
| Async list generation | `for_async!` | Async iteration with yield |
| Async operations in loops | `for_async!` | Uses `<~` for AsyncIO binding |

---

## Function Composition

| Scala | lambars | Description |
|-------|---------|-------------|
| `f andThen g` | `compose!(g, f)` | Left-to-right |
| `f compose g` | `compose!(f, g)` | Right-to-left |
| `f.curried` | `curry2!`, `curry3!`, etc. | Curry function |
| `f.tupled` | Manual | Accept tuple |
| `Function.const(x)` | `constant(x)` | Constant function |
| `identity` | `identity` | Identity function |

### Code Examples

```scala
// Scala
val addOne: Int => Int = _ + 1
val double: Int => Int = _ * 2

val composed1: Int => Int = addOne andThen double  // double(addOne(x))
val composed2: Int => Int = addOne compose double  // addOne(double(x))

val result1 = composed1(5)  // 12 = (5 + 1) * 2
val result2 = composed2(5)  // 11 = (5 * 2) + 1

// Currying
val add: (Int, Int) => Int = _ + _
val curriedAdd: Int => Int => Int = add.curried
val addFive: Int => Int = curriedAdd(5)
val sum = addFive(3)  // 8

// Constant and identity
val alwaysFive: Any => Int = Function.const(5)
val id: Int => Int = identity
```

```rust
// lambars
use lambars::{compose, curry2};
use lambars::compose::{identity, constant};

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// compose! uses right-to-left (mathematical) composition
let composed1 = compose!(double, add_one);  // double(add_one(x))
let composed2 = compose!(add_one, double);  // add_one(double(x))

let result1 = composed1(5);  // 12 = (5 + 1) * 2
let result2 = composed2(5);  // 11 = (5 * 2) + 1

// Currying
fn add(a: i32, b: i32) -> i32 { a + b }
let curried_add = curry2!(add);
let add_five = curried_add(5);
let sum = add_five(3);  // 8

// Constant and identity
let always_five = constant(5);
let result = always_five("anything");  // 5

let x = identity(42);  // 42
```

---

## Optics (Monocle)

### Lens

| Scala (Monocle) | lambars | Description |
|-----------------|---------|-------------|
| `GenLens[S](_.field)` | `lens!(S, field)` | Create lens |
| `lens.get(s)` | `Lens::get` | Get focused value |
| `lens.replace(a)(s)` | `Lens::set` | Set value |
| `lens.modify(f)(s)` | `Lens::modify` | Modify value |
| `lens1.andThen(lens2)` | `Lens::compose` | Compose lenses |

#### Code Examples

```scala
// Scala (Monocle)
import monocle.macros.GenLens

case class Address(street: String, city: String)
case class Person(name: String, address: Address)

val addressLens = GenLens[Person](_.address)
val streetLens = GenLens[Address](_.street)
val personStreetLens = addressLens.andThen(streetLens)

val person = Person("Alice", Address("Main St", "Tokyo"))

val street: String = personStreetLens.get(person)
// street = "Main St"

val updated: Person = personStreetLens.replace("Oak Ave")(person)
// updated.address.street = "Oak Ave"

val modified: Person = personStreetLens.modify(_.toUpperCase)(person)
// modified.address.street = "MAIN ST"
```

```rust
// lambars
use lambars::optics::Lens;
use lambars::lens;

#[derive(Clone)]
struct Address { street: String, city: String }

#[derive(Clone)]
struct Person { name: String, address: Address }

let address_lens = lens!(Person, address);
let street_lens = lens!(Address, street);
let person_street_lens = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        street: "Main St".to_string(),
        city: "Tokyo".to_string(),
    },
};

let street: &String = person_street_lens.get(&person);
// street = "Main St"

let updated: Person = person_street_lens.set(person.clone(), "Oak Ave".to_string());
// updated.address.street = "Oak Ave"

let modified: Person = person_street_lens.modify(person, |s| s.to_uppercase());
// modified.address.street = "MAIN ST"
```

### Prism

| Scala (Monocle) | lambars | Description |
|-----------------|---------|-------------|
| `Prism[S, A](getOption)(reverseGet)` | `FunctionPrism::new` | Create prism |
| `GenPrism[S, A]` | `prism!(S, Variant)` | Derive prism |
| `prism.getOption(s)` | `Prism::preview` | Get if matches |
| `prism.reverseGet(a)` | `Prism::review` | Construct from value |
| `prism.modify(f)(s)` | `Prism::modify` | Modify if matches |

#### Code Examples

```scala
// Scala (Monocle)
import monocle.Prism

sealed trait Shape
case class Circle(radius: Double) extends Shape
case class Rectangle(width: Double, height: Double) extends Shape

val circlePrism = Prism.partial[Shape, Double] {
  case Circle(r) => r
}(Circle.apply)

val shape: Shape = Circle(5.0)
val radius: Option[Double] = circlePrism.getOption(shape)
// radius = Some(5.0)

val constructed: Shape = circlePrism.reverseGet(10.0)
// constructed = Circle(10.0)

val modified: Shape = circlePrism.modify(_ * 2)(shape)
// modified = Circle(10.0)
```

```rust
// lambars
use lambars::optics::{Prism, FunctionPrism};
use lambars::prism;

#[derive(Clone, PartialEq, Debug)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

let circle_prism = prism!(Shape, Circle);

let shape = Shape::Circle(5.0);
let radius: Option<&f64> = circle_prism.preview(&shape);
// radius = Some(&5.0)

let constructed: Shape = circle_prism.review(10.0);
// constructed = Shape::Circle(10.0)

let modified: Shape = circle_prism.modify(shape, |r| r * 2.0);
// modified = Shape::Circle(10.0)
```

### Optional and Traversal

| Scala (Monocle) | lambars | Description |
|-----------------|---------|-------------|
| `Optional[S, A]` | `Optional` trait | May or may not exist |
| `lens.andThen(prism)` | `LensComposeExtension::compose_prism` | Lens + Prism |
| `Traversal[S, A]` | `Traversal` trait | Multiple targets |
| `traversal.getAll(s)` | `Traversal::get_all` | Get all values |
| `traversal.modify(f)(s)` | `Traversal::modify` | Modify all |

#### Code Examples

```scala
// Scala (Monocle)
import monocle.{Optional, Traversal}
import monocle.macros.GenLens

case class Company(employees: List[Employee])
case class Employee(name: String, salary: Double)

val employeesLens = GenLens[Company](_.employees)
val employeeTraversal: Traversal[List[Employee], Employee] =
  Traversal.fromTraverse[List, Employee]
val salaryLens = GenLens[Employee](_.salary)

val companyEmployeeSalaries = employeesLens
  .andThen(employeeTraversal)
  .andThen(salaryLens)

val company = Company(List(
  Employee("Alice", 50000),
  Employee("Bob", 60000)
))

val allSalaries: List[Double] = companyEmployeeSalaries.getAll(company)
// allSalaries = List(50000, 60000)

val raised: Company = companyEmployeeSalaries.modify(_ * 1.1)(company)
// All salaries increased by 10%
```

```rust
// lambars
use lambars::optics::{Lens, Traversal, VecTraversal};
use lambars::lens;

#[derive(Clone)]
struct Company { employees: Vec<Employee> }

#[derive(Clone)]
struct Employee { name: String, salary: f64 }

let employees_lens = lens!(Company, employees);
let salary_lens = lens!(Employee, salary);

let company = Company {
    employees: vec![
        Employee { name: "Alice".to_string(), salary: 50000.0 },
        Employee { name: "Bob".to_string(), salary: 60000.0 },
    ],
};

// Get all salaries
let employees = employees_lens.get(&company);
let all_salaries: Vec<f64> = employees.iter()
    .map(|e| salary_lens.get(e).clone())
    .collect();
// all_salaries = vec![50000.0, 60000.0]

// Raise all salaries by 10%
let raised_employees: Vec<Employee> = employees_lens.get(&company)
    .iter()
    .map(|e| salary_lens.modify(e.clone(), |s| s * 1.1))
    .collect();
let raised = employees_lens.set(company, raised_employees);
```

### Iso

| Scala (Monocle) | lambars | Description |
|-----------------|---------|-------------|
| `Iso[S, A](get)(reverseGet)` | `FunctionIso::new` | Create iso |
| `iso.get(s)` | `Iso::get` | Forward conversion |
| `iso.reverseGet(a)` | `Iso::reverse_get` | Backward conversion |
| `iso.reverse` | `Iso::reverse` | Swap directions |
| `iso1.andThen(iso2)` | `Iso::compose` | Compose isos |

#### Code Examples

```scala
// Scala (Monocle)
import monocle.Iso

val stringListIso: Iso[String, List[Char]] = Iso[String, List[Char]](
  _.toList
)(_.mkString)

val chars: List[Char] = stringListIso.get("hello")
// chars = List('h', 'e', 'l', 'l', 'o')

val str: String = stringListIso.reverseGet(List('h', 'i'))
// str = "hi"

// Reverse the iso
val listStringIso: Iso[List[Char], String] = stringListIso.reverse
```

```rust
// lambars
use lambars::optics::{Iso, FunctionIso};

let string_vec_iso: FunctionIso<String, Vec<char>> = FunctionIso::new(
    |s: String| s.chars().collect(),
    |chars: Vec<char>| chars.into_iter().collect(),
);

let chars: Vec<char> = string_vec_iso.get("hello".to_string());
// chars = vec!['h', 'e', 'l', 'l', 'o']

let s: String = string_vec_iso.reverse_get(vec!['h', 'i']);
// s = "hi"

// Reverse the iso
let vec_string_iso = string_vec_iso.reverse();
```

---

## Effect Systems

### IO Monad

| Scala (Cats Effect) | lambars | Description |
|---------------------|---------|-------------|
| `IO.pure(a)` | `IO::pure` | Pure value |
| `IO.delay(expr)` | `IO::new` | Suspended effect |
| `IO.raiseError(e)` | `IO::catch` with panic | Raise error |
| `io.flatMap(f)` | `IO::flat_map` | Chain effects |
| `io.map(f)` | `IO::fmap` | Transform result |
| `io.attempt` | `IO::catch` | Catch errors |
| `io.unsafeRunSync()` | `IO::run_unsafe` | Execute effects |
| `io *> io2` | `IO::then` | Sequence |
| `io.void` | `io.fmap(\|_\| ())` | Discard result |

#### Code Examples

```scala
// Scala (Cats Effect)
import cats.effect.IO

val io: IO[Int] = IO.pure(42)
val delayed: IO[Int] = IO.delay {
  println("Computing...")
  42
}

val chained: IO[Int] = for {
  x <- IO.pure(10)
  y <- IO.pure(20)
} yield x + y

val result: Int = chained.unsafeRunSync()
// result = 30

// Error handling
val failing: IO[Int] = IO.raiseError(new Exception("oops"))
val recovered: IO[Int] = failing.handleErrorWith(_ => IO.pure(0))
```

```rust
// lambars
use lambars::effect::IO;

let io: IO<i32> = IO::pure(42);
let delayed: IO<i32> = IO::new(|| {
    println!("Computing...");
    42
});

let chained: IO<i32> = IO::pure(10)
    .flat_map(|x| IO::pure(20).fmap(move |y| x + y));

let result: i32 = chained.run_unsafe();
// result = 30

// Error handling with catch
let failing: IO<i32> = IO::new(|| panic!("oops"));
let recovered: IO<i32> = IO::catch(failing, |_| 0);
let result = recovered.run_unsafe();
// result = 0
```

### State Monad

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `State.pure(a)` | `State::pure` | Pure value |
| `State.get` | `State::get` | Get state |
| `State.set(s)` | `State::put` | Set state |
| `State.modify(f)` | `State::modify` | Modify state |
| `State.inspect(f)` | `State::gets` | Get derived value |
| `state.run(s).value` | `State::run` | Run with initial state |
| `state.runS(s).value` | `State::exec` | Get final state |
| `state.runA(s).value` | `State::eval` | Get result only |

#### Code Examples

```scala
// Scala (Cats)
import cats.data.State

val computation: State[Int, String] = for {
  a <- State.get[Int]
  _ <- State.set(a + 10)
  b <- State.get[Int]
} yield s"Started at $a, ended at $b"

val (finalState, result) = computation.run(5).value
// finalState = 15, result = "Started at 5, ended at 15"
```

```rust
// lambars
use lambars::effect::State;
use lambars::eff;

let computation: State<i32, String> = eff! {
    a <= State::get();
    _ <= State::put(a + 10);
    b <= State::get();
    State::pure(format!("Started at {}, ended at {}", a, b))
};

let (result, final_state) = computation.run(5);
// final_state = 15, result = "Started at 5, ended at 15"
```

### Reader Monad

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `Reader.pure(a)` | `Reader::pure` | Pure value |
| `Reader.ask` | `Reader::ask` | Get environment |
| `Reader.local(f)(r)` | `Reader::local` | Modify environment |
| `reader.run(env)` | `Reader::run` | Run with environment |

#### Code Examples

```scala
// Scala (Cats)
import cats.data.Reader

case class Config(baseUrl: String, timeout: Int)

val getUrl: Reader[Config, String] = Reader(_.baseUrl)
val getTimeout: Reader[Config, Int] = Reader(_.timeout)

val computation: Reader[Config, String] = for {
  url <- getUrl
  timeout <- getTimeout
} yield s"$url with timeout $timeout"

val result = computation.run(Config("http://example.com", 30))
// result = "http://example.com with timeout 30"
```

```rust
// lambars
use lambars::effect::Reader;
use lambars::eff;

#[derive(Clone)]
struct Config { base_url: String, timeout: i32 }

let computation: Reader<Config, String> = eff! {
    config <= Reader::ask();
    let url = config.base_url.clone();
    let timeout = config.timeout;
    Reader::pure(format!("{} with timeout {}", url, timeout))
};

let result = computation.run(Config {
    base_url: "http://example.com".to_string(),
    timeout: 30,
});
// result = "http://example.com with timeout 30"
```

### Writer Monad

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `Writer.pure(a)` | `Writer::pure` | Pure value |
| `Writer.tell(w)` | `Writer::tell` | Log output |
| `Writer.value(a)` | `Writer::pure` | Alias for pure |
| `writer.run` | `Writer::run` | Get (result, log) |
| `writer.listen` | `Writer::listen` | Access log in computation |

#### Code Examples

```scala
// Scala (Cats)
import cats.data.Writer
import cats.syntax.writer._

type Logged[A] = Writer[List[String], A]

def logComputation(x: Int): Logged[Int] = for {
  _ <- List(s"Got $x").tell
  result = x * 2
  _ <- List(s"Doubled to $result").tell
} yield result

val (log, result) = logComputation(5).run
// log = List("Got 5", "Doubled to 10"), result = 10
```

```rust
// lambars
use lambars::effect::Writer;
use lambars::eff;

fn log_computation(x: i32) -> Writer<Vec<String>, i32> {
    eff! {
        _ <= Writer::tell(vec![format!("Got {}", x)]);
        let result = x * 2;
        _ <= Writer::tell(vec![format!("Doubled to {}", result)]);
        Writer::pure(result)
    }
}

let (result, log) = log_computation(5).run();
// log = vec!["Got 5", "Doubled to 10"], result = 10
```

---

## Monad Transformers

### Comparison

| Scala (Cats) | lambars | Description |
|--------------|---------|-------------|
| `OptionT[F, A]` | Custom implementation | Option in F |
| `EitherT[F, E, A]` | `ExceptT<E, F, A>` | Either in F |
| `StateT[F, S, A]` | `StateT<S, F, A>` | State in F |
| `ReaderT[F, R, A]` | `ReaderT<R, F, A>` | Reader in F |
| `WriterT[F, W, A]` | `WriterT<W, F, A>` | Writer in F |
| `Kleisli[F, A, B]` | `ReaderT<A, F, B>` | Function wrapper |

### Code Examples

```scala
// Scala (Cats) - EitherT
import cats.data.EitherT
import cats.effect.IO

type IOEither[A] = EitherT[IO, String, A]

def validatePositive(x: Int): IOEither[Int] =
  EitherT.cond[IO](x > 0, x, s"$x is not positive")

val computation: IOEither[Int] = for {
  a <- validatePositive(10)
  b <- validatePositive(20)
} yield a + b

val result: IO[Either[String, Int]] = computation.value
```

```rust
// lambars
use lambars::effect::{ExceptT, IO};

fn validate_positive(x: i32) -> ExceptT<String, IO<Result<i32, String>>> {
    if x > 0 {
        ExceptT::pure_io(x)
    } else {
        ExceptT::throw_io(format!("{} is not positive", x))
    }
}

let computation = validate_positive(10)
    .flat_map_io(|a| {
        validate_positive(20)
            .fmap_io(move |b| a + b)
    });

let result: Result<i32, String> = computation.run_io().run_unsafe();
// result = Ok(30)
```

```scala
// Scala (Cats) - ReaderT / Kleisli
import cats.data.ReaderT
import cats.effect.IO

case class AppConfig(dbUrl: String, apiKey: String)

type AppIO[A] = ReaderT[IO, AppConfig, A]

def getDbUrl: AppIO[String] = ReaderT(config => IO.pure(config.dbUrl))
def logMessage(msg: String): AppIO[Unit] = ReaderT(_ => IO(println(msg)))

val program: AppIO[String] = for {
  url <- getDbUrl
  _ <- logMessage(s"Connecting to $url")
} yield url

val result: IO[String] = program.run(AppConfig("jdbc://...", "secret"))
```

```rust
// lambars
use lambars::effect::{ReaderT, IO};

#[derive(Clone)]
struct AppConfig { db_url: String, api_key: String }

fn get_db_url() -> ReaderT<AppConfig, IO<String>> {
    ReaderT::ask_io().fmap_io(|config: AppConfig| config.db_url)
}

fn log_message(msg: String) -> ReaderT<AppConfig, IO<()>> {
    ReaderT::lift_io(IO::new(move || println!("{}", msg)))
}

let program = get_db_url()
    .flat_map_io(|url| {
        log_message(format!("Connecting to {}", url))
            .then_io(ReaderT::pure_io(url))
    });

let config = AppConfig {
    db_url: "jdbc://...".to_string(),
    api_key: "secret".to_string(),
};
let result: String = program.run_io(config).run_unsafe();
```

### AsyncIO Support in Transformers

lambars provides AsyncIO integration for monad transformers, similar to Cats Effect's async capabilities.

```rust
// lambars - ReaderT with AsyncIO (async feature required)
use lambars::effect::{ReaderT, AsyncIO};

#[derive(Clone)]
struct AppConfig { api_url: String }

type AppAsync<A> = ReaderT<AppConfig, AsyncIO<A>>;

fn get_api_url() -> AppAsync<String> {
    ReaderT::asks_async_io(|c: &AppConfig| c.api_url.clone())
}

async fn example() {
    let computation = get_api_url()
        .flat_map_async_io(|url| ReaderT::pure_async_io(format!("Fetching: {}", url)));

    let config = AppConfig { api_url: "https://api.example.com".to_string() };
    let result = computation.run_async_io(config).run_async().await;
    // result = "Fetching: https://api.example.com"
}
```

Available AsyncIO methods:
- `ReaderT`: `ask_async_io`, `asks_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `StateT`: `get_async_io`, `gets_async_io`, `state_async_io`, `lift_async_io`, `pure_async_io`
- `WriterT`: `tell_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`, `listen_async_io`

---

## Persistent Collections

### Comparison

| Scala | lambars | Description |
|-------|---------|-------------|
| `List[A]` | `PersistentList<A>` | Immutable list |
| `Vector[A]` | `PersistentVector<A>` | Immutable vector |
| `Map[K, V]` | `PersistentHashMap<K, V>` | Immutable hash map |
| `SortedMap[K, V]` | `PersistentTreeMap<K, V>` | Immutable sorted map |
| `Set[A]` | `PersistentHashSet<A>` | Immutable set |
| `Set[A].view` | `HashSetView<A>` | Lazy view over set |

### Code Examples

```scala
// Scala - List
val list = List(1, 2, 3)
val prepended = 0 :: list
// list = List(1, 2, 3) (unchanged)
// prepended = List(0, 1, 2, 3)

val head: Option[Int] = list.headOption
// head = Some(1)

val tail: List[Int] = list.tail
// tail = List(2, 3)
```

```rust
// lambars
use lambars::persistent::PersistentList;

let list = PersistentList::new().cons(3).cons(2).cons(1);
let prepended = list.cons(0);
// list.len() = 3 (unchanged)
// prepended.len() = 4

let head: Option<&i32> = list.head();
// head = Some(&1)

let tail: Option<PersistentList<i32>> = list.tail();
// tail = Some(PersistentList [2, 3])
```

```scala
// Scala - Vector
val vec = Vector(1, 2, 3)
val updated = vec.updated(1, 99)
// vec = Vector(1, 2, 3) (unchanged)
// updated = Vector(1, 99, 3)

val appended = vec :+ 4
// appended = Vector(1, 2, 3, 4)

val element: Int = vec(1)
// element = 2
```

```rust
// lambars
use lambars::persistent::PersistentVector;

let vec: PersistentVector<i32> = vec![1, 2, 3].into_iter().collect();
let updated = vec.update(1, 99).unwrap();
// vec.get(1) = Some(&2) (unchanged)
// updated.get(1) = Some(&99)

let appended = vec.push_back(4);
// appended.len() = 4

let element: Option<&i32> = vec.get(1);
// element = Some(&2)
```

```scala
// Scala - Map
val map = Map("a" -> 1, "b" -> 2)
val updated = map + ("c" -> 3)
// map = Map("a" -> 1, "b" -> 2) (unchanged)
// updated = Map("a" -> 1, "b" -> 2, "c" -> 3)

val value: Option[Int] = map.get("a")
// value = Some(1)

val removed = map - "a"
// removed = Map("b" -> 2)
```

```rust
// lambars
use lambars::persistent::PersistentHashMap;

let map = PersistentHashMap::new()
    .insert("a".to_string(), 1)
    .insert("b".to_string(), 2);
let updated = map.insert("c".to_string(), 3);
// map.get("c") = None (unchanged)
// updated.get("c") = Some(&3)

let value: Option<&i32> = map.get("a");
// value = Some(&1)

let removed = map.remove("a");
// removed.get("a") = None
```

---

## Lazy Evaluation

| Scala | lambars | Description |
|-------|---------|-------------|
| `lazy val x = expr` | `Lazy::new(\|\| expr)` | Lazy value |
| `x` (force) | `Lazy::force` | Force evaluation |
| `LazyList` | `Iterator` | Lazy sequence |

### Code Examples

```scala
// Scala
lazy val expensive = {
  println("Computing...")
  42
}
// Nothing printed yet

val result = expensive
// "Computing..." printed, result = 42

val result2 = expensive
// Nothing printed (cached), result2 = 42
```

```rust
// lambars
use lambars::control::Lazy;

let expensive = Lazy::new(|| {
    println!("Computing...");
    42
});
// Nothing printed yet

let result = expensive.force();
// "Computing..." printed, result = 42

let result2 = expensive.force();
// Nothing printed (cached), result2 = 42
```

```scala
// Scala - LazyList (formerly Stream)
val naturals: LazyList[Int] = LazyList.from(0)
val firstTen: List[Int] = naturals.take(10).toList
// firstTen = List(0, 1, 2, 3, 4, 5, 6, 7, 8, 9)
```

```rust
// Rust (std)
let naturals = 0..;
let first_ten: Vec<i32> = naturals.take(10).collect();
// first_ten = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
```

---

## Implicits vs Traits

### Type Class Instances

```scala
// Scala - Implicits
trait Show[A] {
  def show(a: A): String
}

object Show {
  def apply[A](implicit instance: Show[A]): Show[A] = instance

  implicit val intShow: Show[Int] = (a: Int) => a.toString
  implicit val stringShow: Show[String] = (a: String) => s"\"$a\""
}

def printShow[A: Show](a: A): Unit =
  println(Show[A].show(a))

printShow(42)        // "42"
printShow("hello")   // "\"hello\""
```

```rust
// Rust - Traits
trait Show {
    fn show(&self) -> String;
}

impl Show for i32 {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl Show for String {
    fn show(&self) -> String {
        format!("\"{}\"", self)
    }
}

fn print_show<A: Show>(a: &A) {
    println!("{}", a.show());
}

print_show(&42);              // "42"
print_show(&"hello".to_string());  // "\"hello\""
```

### Extension Methods

```scala
// Scala - Extension methods via implicits
implicit class IntOps(val n: Int) extends AnyVal {
  def isEven: Boolean = n % 2 == 0
  def squared: Int = n * n
}

val result = 4.squared  // 16
val even = 4.isEven     // true
```

```rust
// Rust - Extension traits
trait IntOps {
    fn is_even(&self) -> bool;
    fn squared(&self) -> i32;
}

impl IntOps for i32 {
    fn is_even(&self) -> bool { self % 2 == 0 }
    fn squared(&self) -> i32 { self * self }
}

let result = 4.squared();  // 16
let even = 4.is_even();    // true
```

---

## Summary: Key Differences

### Syntax Differences

| Aspect | Scala | lambars (Rust) |
|--------|-------|----------------|
| For-comprehension (Monad) | `for { x <- mx } yield x` | `eff! { x <= mx; expr }` |
| For-comprehension (List) | `for { x <- xs } yield x` | `for_! { x <= xs; yield x }` |
| Type parameters | `F[A]` | `F<A>` |
| Option | `Some(x)` / `None` | `Some(x)` / `None` |
| Either | `Right(x)` / `Left(e)` | `Ok(x)` / `Err(e)` |
| Lambda | `x => x + 1` | `\|x\| x + 1` |
| Method call | `obj.method(arg)` | `obj.method(arg)` |
| Type annotation | `x: Int` | `x: i32` |
| Trait definition | `trait T { }` | `trait T { }` |
| Implicit | `implicit val` | N/A (explicit traits) |

### Conceptual Differences

1. **Implicits vs Explicit**: Scala uses implicits for type class instances; Rust requires explicit trait implementations and imports.

2. **Higher-Kinded Types**: Scala has native HKT support; lambars emulates HKT using GAT.

3. **Variance**: Scala has declaration-site variance (`+A`, `-A`); Rust uses `PhantomData` for variance.

4. **Null Safety**: Scala has `null` (from Java); Rust has no null, only `Option`.

5. **Error Handling**: Scala uses `Either[E, A]` (left = error); Rust uses `Result<T, E>` (right-biased by default).

6. **Ownership**: Rust has ownership/borrowing; Scala has garbage collection.

7. **Pattern Matching**: Both support pattern matching, but Rust requires exhaustive matching.

---

## Migration Tips

1. **Replace `for` comprehension with `eff!` or `for_!`**: Use `<=` instead of `<-` for binding.
   - Use `eff!` for monads (Option, Result, IO, State, Reader, Writer)
   - Use `for_!` for lists (Vec) with `yield` keyword

2. **Use `for_!` for List comprehensions**: When translating Scala List for-comprehensions, use `for_!` with `yield`. Remember to `.clone()` inner collections due to Rust's ownership.

3. **Replace `Either` with `Result`**: Note the type parameter order difference (`Either[E, A]` vs `Result<A, E>`).

4. **Import traits explicitly**: Unlike Scala implicits, Rust traits must be imported to use their methods.

5. **Use `.fmap()` instead of `.map()`**: For type class-based mapping (std `Option/Result` have `.map()` too).

6. **Use `.flat_map()` instead of `.flatMap()`**: Snake case naming convention.

7. **Add explicit type annotations**: Rust's type inference is less aggressive than Scala's.

8. **Handle ownership**: Clone values when needed, or use references appropriately.

9. **Use `PersistentVector` for Scala `Vector`**: Similar performance characteristics with structural sharing.
