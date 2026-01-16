# Scala to lambars API 比較ガイド

[English](README.en.md)

> **Note**: このドキュメントは AI によって翻訳されました。誤りや不自然な表現がある場合は、Issue または Pull Request でお知らせください。

このドキュメントは、Scala の関数型プログラミング構造（Cats および Scalaz ライブラリを含む）と、lambars（Rust）での同等の機能を包括的に比較します。

## 目次

- [概要](#概要)
- [型クラス](#型クラス)
  - [Functor](#functor)
  - [Applicative](#applicative)
  - [Monad](#monad)
  - [Semigroup and Monoid](#semigroup-and-monoid)
  - [Foldable](#foldable)
  - [Traversable](#traversable)
- [Option と Either](#option-と-either)
- [for 内包表記 vs eff! マクロ](#for-内包表記-vs-eff-マクロ)
- [関数合成](#関数合成)
- [Optics (Monocle)](#optics-monocle)
- [エフェクトシステム](#エフェクトシステム)
- [モナド変換子](#モナド変換子)
- [永続コレクション](#永続コレクション)
- [遅延評価](#遅延評価)
- [Implicits vs Traits](#implicits-vs-traits)

---

## 概要

| 概念                      | Scala (Cats)                  | lambars (Rust)       |
| ------------------------- | ----------------------------- | -------------------- |
| Functor                   | `Functor[F]`                  | `Functor` trait      |
| Applicative               | `Applicative[F]`              | `Applicative` trait  |
| Monad                     | `Monad[F]`                    | `Monad` trait        |
| Semigroup                 | `Semigroup[A]`                | `Semigroup` trait    |
| Monoid                    | `Monoid[A]`                   | `Monoid` trait       |
| Foldable                  | `Foldable[F]`                 | `Foldable` trait     |
| Traverse                  | `Traverse[F]`                 | `Traversable` trait  |
| Option                    | `Option[A]`                   | `Option<A>` (std)    |
| Either                    | `Either[E, A]`                | `Result<A, E>` (std) |
| For-comprehension (Monad) | `for { ... } yield`           | `eff!` macro         |
| For-comprehension (List)  | `for { ... } yield` (List)    | `for_!` macro        |
| Async for-comprehension   | `for { ... } yield` + `IO`    | `for_async!` macro   |
| Lens                      | `monocle.Lens`                | `Lens` trait         |
| Prism                     | `monocle.Prism`               | `Prism` trait        |
| IO                        | `cats.effect.IO`              | `IO` type            |
| State                     | `cats.data.State`             | `State` type         |
| Reader                    | `cats.data.Reader`            | `Reader` type        |
| Writer                    | `cats.data.Writer`            | `Writer` type        |
| RWS                       | `cats.data.ReaderWriterState` | `RWS` type           |
| StateT                    | `cats.data.StateT`            | `StateT` type        |
| ReaderT                   | `cats.data.ReaderT`           | `ReaderT` type       |
| WriterT                   | `cats.data.WriterT`           | `WriterT` type       |
| EitherT                   | `cats.data.EitherT`           | `ExceptT` type       |
| Free モナド               | `cats.free.Free[F, A]`        | `Freer<I, A>` type   |

---

## 型クラス

### Functor

| Scala (Cats)         | lambars                         | 説明                    |
| -------------------- | ------------------------------- | ----------------------- |
| `F[A].map(f)`        | `Functor::fmap`                 | 内部の値を変換          |
| `F[A].as(b)`         | `fmap(\|_\| b)`                 | 定数で置き換える        |
| `F[A].void`          | `fmap(\|_\| ())`                | 値を捨てる              |
| `F[A].fproduct(f)`   | `fmap(\|a\| (a.clone(), f(a)))` | 関数結果とペアにする    |
| `Functor[F].lift(f)` | 手動実装                        | 関数を functor にリフト |

#### コード例

```scala
// Scala (Cats)
import cats.Functor
import cats.syntax.functor._

val doubled: Option[Int] = Some(21).map(_ * 2)
// doubled = Some(42)

val list: List[Int] = List(1, 2, 3).map(_ * 2)
// list = List(2, 4, 6)

// Functor 型クラスを使用
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

// Functor trait を使用
fn double_f<F: Functor<Inner = i32>>(fa: F) -> F::WithType<i32>
where
    F::WithType<i32>: Functor,
{
    fa.fmap(|x| x * 2)
}
```

### Applicative

| Scala (Cats)       | lambars                   | 説明                     |
| ------------------ | ------------------------- | ------------------------ |
| `A.pure[F]`        | `Applicative::pure`       | 値をコンテキストにリフト |
| `(fa, fb).mapN(f)` | `Applicative::map2`       | 関数で結合               |
| `(fa, fb).tupled`  | `Applicative::product`    | タプルに結合             |
| `fa.ap(ff)`        | `Applicative::apply`      | ラップされた関数を適用   |
| `fa *> fb`         | `fa.map2(fb, \|_, b\| b)` | シーケンス、右を保持     |
| `fa <* fb`         | `fa.map2(fb, \|a, _\| a)` | シーケンス、左を保持     |

#### コード例

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

| Scala (Cats)              | lambars                                      | 説明                 |
| ------------------------- | -------------------------------------------- | -------------------- |
| `fa.flatMap(f)`           | `Monad::flat_map`                            | 計算を連鎖           |
| `fa.flatten`              | `Flatten::flatten` / `Option::flatten` (std) | ネストを平坦化       |
| `fa >> fb`                | `Monad::then`                                | シーケンス、右を保持 |
| `fa.mproduct(f)`          | `flat_map(\|a\| f(a).map(\|b\| (a, b)))`     | 結果とペアにする     |
| `fa.ifM(ifTrue, ifFalse)` | 手動実装                                     | 条件分岐             |
| `Monad[F].whileM_`        | 手動実装                                     | while ループ         |
| `Monad[F].iterateWhile`   | 手動実装                                     | 条件付き反復         |

#### コード例

```scala
// Scala (Cats)
import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._

def safeDivide(x: Int, y: Int): Option[Int] =
  if (y == 0) None else Some(x / y)

val result: Option[Int] = Some(10).flatMap(x => safeDivide(x, 2))
// result = Some(5)

// チェイニング
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

// eff! マクロでチェイニング
use lambars::eff;

let chained: Option<i32> = eff! {
    a <= Some(10);
    b <= safe_divide(a, 2);
    c <= safe_divide(b, 1);
    Some(c)
};
// chained = Some(5)

// Flatten (Flatten trait または std の Option::flatten を使用)
use lambars::typeclass::Flatten;
let nested: Option<Option<i32>> = Some(Some(42));
let flat: Option<i32> = nested.flatten();
// flat = Some(42)

// Flatten は Result、Box、Identity でも機能
let nested_result: Result<Result<i32, &str>, &str> = Ok(Ok(42));
let flat_result: Result<i32, &str> = nested_result.flatten();
// flat_result = Ok(42)
```

### Semigroup and Monoid

| Scala (Cats)                 | lambars               | 説明               |
| ---------------------------- | --------------------- | ------------------ |
| `a \|+\| b`                  | `Semigroup::combine`  | 値を結合           |
| `Monoid[A].empty`            | `Monoid::empty`       | 単位元             |
| `Monoid[A].combineAll(list)` | `Monoid::combine_all` | combine で畳み込む |
| `a.combineN(n)`              | 手動ループ            | n 回結合           |
| `Semigroup.maybeCombine`     | `Option::combine`     | option を結合      |

#### コード例

```scala
// Scala (Cats)
import cats.Monoid
import cats.syntax.semigroup._

val combined: String = "Hello, " |+| "World!"
// combined = "Hello, World!"

val sum: Int = Monoid[Int].combineAll(List(1, 2, 3, 4, 5))
// sum = 15

// 積のためのカスタムモノイド
import cats.kernel.instances.int._
val product: Int = List(1, 2, 3, 4, 5).foldLeft(1)(_ * _)
// product = 120

// Sum/Product ラッパーを使用
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

// Product モノイド
let items = vec![Product::new(1), Product::new(2), Product::new(3), Product::new(4), Product::new(5)];
let product: Product<i32> = Product::combine_all(items);
// product = Product(120)

// Vec の結合
let vec1 = vec![1, 2, 3];
let vec2 = vec![4, 5, 6];
let combined: Vec<i32> = vec1.combine(vec2);
// combined = vec![1, 2, 3, 4, 5, 6]
```

### Foldable

| Scala (Cats)          | lambars                | 説明                       |
| --------------------- | ---------------------- | -------------------------- |
| `fa.foldLeft(b)(f)`   | `Foldable::fold_left`  | 左畳み込み                 |
| `fa.foldRight(lb)(f)` | `Foldable::fold_right` | 右畳み込み                 |
| `fa.foldMap(f)`       | `Foldable::fold_map`   | マップしてから畳み込む     |
| `fa.fold`             | `Foldable::fold`       | Monoid で畳み込む          |
| `fa.find(p)`          | `Foldable::find`       | 最初にマッチするものを検索 |
| `fa.exists(p)`        | `Foldable::exists`     | いずれかがマッチ           |
| `fa.forall(p)`        | `Foldable::for_all`    | すべてがマッチ             |
| `fa.isEmpty`          | `Foldable::is_empty`   | 空チェック                 |
| `fa.nonEmpty`         | `!Foldable::is_empty`  | 非空チェック               |
| `fa.size`             | `Foldable::length`     | 要素数をカウント           |
| `fa.toList`           | `Foldable::to_vec`     | リストに変換               |

#### コード例

```scala
// Scala (Cats)
import cats.Foldable
import cats.syntax.foldable._

val sum: Int = List(1, 2, 3, 4, 5).foldLeft(0)(_ + _)
// sum = 15

val product: Int = List(1, 2, 3, 4, 5).foldRight(1)(_ * _)
// product = 120

// String モノイドで foldMap
val concat: String = List(1, 2, 3).foldMap(_.toString)
// concat = "123"

// find
val found: Option[Int] = List(1, 2, 3, 4, 5).find(_ > 3)
// found = Some(4)

// exists と forall
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

// String モノイドで fold_map
let concat: String = vec![1, 2, 3].fold_map(|x| x.to_string());
// concat = "123"

// find
let found: Option<&i32> = vec![1, 2, 3, 4, 5].find(|x| **x > 3);
// found = Some(&4)

// exists と for_all
let has_even: bool = vec![1, 2, 3].exists(|x| x % 2 == 0);
// has_even = true

let all_positive: bool = vec![1, 2, 3].for_all(|x| *x > 0);
// all_positive = true
```

### Traversable

| Scala (Cats)                    | lambars                               | 説明                            |
| ------------------------------- | ------------------------------------- | ------------------------------- |
| `fa.traverse(f)`                | `Traversable::traverse_option/result` | エフェクトでトラバース          |
| `fa.sequence`                   | `Traversable::sequence_option/result` | エフェクトをシーケンス          |
| `fa.flatTraverse(f)`            | flatten と合成                        | トラバースして平坦化            |
| `fa.traverseFilter(f)`          | 手動実装                              | トラバース中にフィルタ          |
| `fa.traverse[Reader[R, *]](f)`  | `Traversable::traverse_reader`        | Reader エフェクトでトラバース   |
| `fa.traverse[State[S, *]](f)`   | `Traversable::traverse_state`         | State エフェクトでトラバース    |
| `fa.traverse[IO](f)`            | `Traversable::traverse_io`            | IO エフェクトでトラバース       |
| `fa.parTraverse[IO](f)`         | `Traversable::traverse_async_io_parallel` | IO エフェクトで並行トラバース   |
| `fa.sequence[Reader[R, *]]`     | `Traversable::sequence_reader`        | Reader エフェクトをシーケンス   |
| `fa.sequence[State[S, *]]`      | `Traversable::sequence_state`         | State エフェクトをシーケンス    |
| `fa.sequence[IO]`               | `Traversable::sequence_io`            | IO エフェクトをシーケンス       |
| `fa.traverse_[Reader[R, *]](f)` | `Traversable::traverse_reader_`       | Reader をトラバース、結果を破棄 |
| `fa.traverse_[State[S, *]](f)`  | `Traversable::traverse_state_`        | State をトラバース、結果を破棄  |
| `fa.traverse_[IO](f)`           | `Traversable::traverse_io_`           | IO をトラバース、結果を破棄     |

#### コード例

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

// traverse_reader - Reader エフェクトでトラバース
use lambars::effect::Reader;

#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
// result = vec![10, 20, 30]

// traverse_state - State エフェクトでトラバース（状態は左から右にスレッド化される）
use lambars::effect::State;

let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
// result = vec![(0, "a"), (1, "b"), (2, "c")]
// final_index = 3

// traverse_io - IO エフェクトでトラバース（IO アクションは順次実行される）
use lambars::effect::IO;

let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
// contents = vec!["content of a.txt", "content of b.txt"]
```

---

## Option と Either

### Option

| Scala                    | lambars / Rust std        | 説明                   |
| ------------------------ | ------------------------- | ---------------------- |
| `Some(x)`                | `Some(x)`                 | Some を構築            |
| `None`                   | `None`                    | None を構築            |
| `opt.map(f)`             | `Functor::fmap`           | 値を変換               |
| `opt.flatMap(f)`         | `Monad::flat_map`         | 計算を連鎖             |
| `opt.getOrElse(default)` | `Option::unwrap_or`       | デフォルト値           |
| `opt.orElse(alt)`        | `Option::or`              | 代替値                 |
| `opt.fold(ifEmpty)(f)`   | `Option::map_or`          | デフォルト値で畳み込む |
| `opt.filter(p)`          | `Option::filter`          | 述語でフィルタ         |
| `opt.filterNot(p)`       | `filter(\|x\| !p(x))`     | 逆フィルタ             |
| `opt.contains(x)`        | `Option::contains`        | 値を含む               |
| `opt.exists(p)`          | `Option::is_some_and`     | 述語でテスト           |
| `opt.forall(p)`          | `opt.map_or(true, p)`     | すべてが満たす         |
| `opt.toRight(left)`      | `Option::ok_or`           | Either/Result に変換   |
| `opt.toLeft(right)`      | `opt.ok_or(right).swap()` | Either（左）に変換     |
| `opt.zip(other)`         | `Option::zip`             | 2 つの option を zip   |

### Either / Result

| Scala                 | lambars / Rust std              | 説明           |
| --------------------- | ------------------------------- | -------------- |
| `Right(x)`            | `Ok(x)`                         | Right/Ok 値    |
| `Left(e)`             | `Err(e)`                        | Left/Error 値  |
| `either.map(f)`       | `Functor::fmap` / `Result::map` | 右をマップ     |
| `either.leftMap(f)`   | `Result::map_err`               | 左をマップ     |
| `either.flatMap(f)`   | `Monad::flat_map`               | 連鎖           |
| `either.bimap(f, g)`  | 手動実装                        | 両側をマップ   |
| `either.fold(f, g)`   | `Result::map_or_else`           | 両側を畳み込む |
| `either.swap`         | `Result::swap` (nightly)        | 両側を入れ替え |
| `either.toOption`     | `Result::ok`                    | Option に変換  |
| `either.getOrElse(d)` | `Result::unwrap_or`             | デフォルト値   |
| `either.orElse(alt)`  | `Result::or`                    | 代替値         |

#### コード例

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

// bimap 相当
let either: Result<i32, String> = Ok(42);
let bi_mapped: Result<String, usize> = either
    .map(|x| x.to_string())
    .map_err(|e| e.len());
// bi_mapped = Ok("42")
```

---

## for 内包表記 vs eff! / for\_! マクロ

lambars は Scala の for 内包表記に対応する 2 つのマクロを提供します：

| ユースケース                                          | Scala                        | lambars       | 説明                    |
| ----------------------------------------------------- | ---------------------------- | ------------- | ----------------------- |
| Monad バインディング (Option, Result, IO, State など) | `for { x <- mx } yield x`    | `eff!` macro  | 単一実行、FnOnce ベース |
| リスト内包表記 (Vec, iterators)                       | `for { x <- xs } yield f(x)` | `for_!` macro | 複数実行、FnMut ベース  |

### 主な違い：`eff!` vs `for_!`

| 側面             | `eff!`                                    | `for_!`                  |
| ---------------- | ----------------------------------------- | ------------------------ |
| **対象型**       | Option, Result, IO, State, Reader, Writer | Vec, Iterator            |
| **実行**         | 単一実行 (FnOnce)                         | 複数実行 (FnMut)         |
| **最終式**       | ラップされた値を返す必要がある            | `yield` キーワードを使用 |
| **クロージャ型** | `move` クロージャ                         | 通常のクロージャ         |
| **典型的な用途** | モナディック計算の連鎖                    | リスト内包表記           |

### 構文比較

#### eff! マクロ（Monad 用）

| Scala                                   | lambars                               | 説明                 |
| --------------------------------------- | ------------------------------------- | -------------------- |
| `for { x <- mx } yield x`               | `eff! { x <= mx; mx2 }`               | 基本バインド         |
| `for { x <- mx; y <- my } yield (x, y)` | `eff! { x <= mx; y <= my; expr }`     | 複数バインド         |
| `for { x <- mx; if p(x) } yield x`      | `eff! { x <= mx.filter(p); Some(x) }` | ガード（Option）     |
| `x = expr` (in for)                     | `let x = expr;`                       | 純粋なバインディング |

#### for\_! マクロ（List 用）

| Scala                                   | lambars                                            | 説明                         |
| --------------------------------------- | -------------------------------------------------- | ---------------------------- |
| `for { x <- xs } yield f(x)`            | `for_! { x <= xs; yield f(x) }`                    | 基本リスト内包表記           |
| `for { x <- xs; y <- ys } yield (x, y)` | `for_! { x <= xs; y <= ys.clone(); yield (x, y) }` | ネストされた反復             |
| `for { x <- xs; if p(x) } yield x`      | `xs.into_iter().filter(p).collect()`               | フィルタリング（std を使用） |
| `x = expr` (in for)                     | `let x = expr;`                                    | 純粋なバインディング         |

**重要**：`for_!` では、Rust の所有権ルールにより、内部コレクションには通常 `.clone()` が必要です。

### コード例

```scala
// Scala - for 内包表記
val result: Option[Int] = for {
  x <- Some(10)
  y <- Some(20)
  z = x + y
} yield z * 2
// result = Some(60)

// Either を使用
val computation: Either[String, Int] = for {
  a <- Right(10)
  b <- Right(20)
  _ <- if (b > 0) Right(()) else Left("b must be positive")
} yield a + b
// computation = Right(30)

// ネストされた for 内包表記
val nested: Option[Int] = for {
  list <- Some(List(1, 2, 3))
  first <- list.headOption
  doubled = first * 2
} yield doubled
// nested = Some(2)
```

```rust
// lambars - eff! マクロ
use lambars::eff;

let result: Option<i32> = eff! {
    x <= Some(10);
    y <= Some(20);
    let z = x + y;
    Some(z * 2)
};
// result = Some(60)

// Result を使用
let computation: Result<i32, String> = eff! {
    a <= Ok::<i32, String>(10);
    b <= Ok::<i32, String>(20);
    _ <= if b > 0 { Ok(()) } else { Err("b must be positive".to_string()) };
    Ok(a + b)
};
// computation = Ok(30)

// ネストされた計算
let nested: Option<i32> = eff! {
    list <= Some(vec![1, 2, 3]);
    first <= list.first().copied();
    let doubled = first * 2;
    Some(doubled)
};
// nested = Some(2)
```

### 複雑な例

```scala
// Scala - データベース風の操作
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

### for\_! を使ったリスト内包表記

リストベースの for 内包表記には `for_!` マクロを使用します：

```scala
// Scala - リスト内包表記
val numbers = List(1, 2, 3, 4, 5)
val doubled: List[Int] = for {
  n <- numbers
} yield n * 2
// doubled = List(2, 4, 6, 8, 10)

// ネストされたリスト内包表記
val xs = List(1, 2)
val ys = List(10, 20)
val cartesian: List[Int] = for {
  x <- xs
  y <- ys
} yield x + y
// cartesian = List(11, 21, 12, 22)

// 書籍推薦の例
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
// lambars - for_! マクロ
use lambars::for_;

let numbers = vec![1, 2, 3, 4, 5];
let doubled: Vec<i32> = for_! {
    n <= numbers;
    yield n * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// ネストされたリスト内包表記
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // 注意：内部反復のために clone() が必要
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// 書籍推薦の例
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
    author <= book.authors.clone();  // 注意：clone() が必要
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

### 各マクロの使用タイミング

| シナリオ               | 推奨マクロ   | 理由                                 |
| ---------------------- | ------------ | ------------------------------------ |
| Option/Result の連鎖   | `eff!`       | None/Err で短絡                      |
| IO/State/Reader/Writer | `eff!`       | FnOnce モナド用に設計                |
| List/Vec の変換        | `for_!`      | 複数反復をサポート                   |
| 直積                   | `for_!`      | yield を使ったネスト反復             |
| データベース風クエリ   | `eff!`       | モナディックエラーハンドリング       |
| データ生成             | `for_!`      | 複数の結果が必要                     |
| 非同期リスト生成       | `for_async!` | yield を使った非同期反復             |
| ループ内の非同期操作   | `for_async!` | AsyncIO バインディングに `<~` を使用 |

---

## 関数合成

| Scala               | lambars             | 説明                         |
| ------------------- | ------------------- | ---------------------------- |
| `f andThen g`       | `compose!(g, f)`    | 左から右                     |
| `f compose g`       | `compose!(f, g)`    | 右から左                     |
| `m.map(f)`          | `pipe!(m, => f)`    | モナド内で純粋関数をリフト   |
| `m.flatMap(f)`      | `pipe!(m, =>> f)`   | モナド関数をバインド         |
| `asyncIO.map(f)`    | `pipe_async!(m, => f)` | AsyncIO用リフト（インヒアレント）|
| `asyncIO.flatMap(f)`| `pipe_async!(m, =>> f)`| AsyncIO用バインド（インヒアレント）|
| `f.curried`         | `curry!(fn, arity)` | 関数をカリー化               |
| `f.tupled`          | 手動実装            | タプルを受け取る             |
| `Function.const(x)` | `constant(x)`       | 定数関数                     |
| `identity`          | `identity`          | 恒等関数                     |

### コード例

```scala
// Scala
val addOne: Int => Int = _ + 1
val double: Int => Int = _ * 2

val composed1: Int => Int = addOne andThen double  // double(addOne(x))
val composed2: Int => Int = addOne compose double  // addOne(double(x))

val result1 = composed1(5)  // 12 = (5 + 1) * 2
val result2 = composed2(5)  // 11 = (5 * 2) + 1

// カリー化
val add: (Int, Int) => Int = _ + _
val curriedAdd: Int => Int => Int = add.curried
val addFive: Int => Int = curriedAdd(5)
val sum = addFive(3)  // 8

// 定数関数と恒等関数
val alwaysFive: Any => Int = Function.const(5)
val id: Int => Int = identity
```

```rust
// lambars
use lambars::{compose, curry};
use lambars::compose::{identity, constant};

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// compose! は右から左（数学的）合成を使用
let composed1 = compose!(double, add_one);  // double(add_one(x))
let composed2 = compose!(add_one, double);  // add_one(double(x))

let result1 = composed1(5);  // 12 = (5 + 1) * 2
let result2 = composed2(5);  // 11 = (5 * 2) + 1

// 関数名 + アリティ形式でカリー化
fn add(a: i32, b: i32) -> i32 { a + b }
let curried_add = curry!(add, 2);
let add_five = curried_add(5);
let sum = add_five(3);  // 8

// 定数関数と恒等関数
let always_five = constant(5);
let result = always_five("anything");  // 5

let x = identity(42);  // 42
```

---

## Optics (Monocle)

### Lens

| Scala (Monocle)        | lambars           | 説明                   |
| ---------------------- | ----------------- | ---------------------- |
| `GenLens[S](_.field)`  | `lens!(S, field)` | lens を作成            |
| `lens.get(s)`          | `Lens::get`       | フォーカスした値を取得 |
| `lens.replace(a)(s)`   | `Lens::set`       | 値を設定               |
| `lens.modify(f)(s)`    | `Lens::modify`    | 値を変更               |
| `lens1.andThen(lens2)` | `Lens::compose`   | lens を合成            |

#### コード例

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

| Scala (Monocle)                      | lambars              | 説明             |
| ------------------------------------ | -------------------- | ---------------- |
| `Prism[S, A](getOption)(reverseGet)` | `FunctionPrism::new` | prism を作成     |
| `GenPrism[S, A]`                     | `prism!(S, Variant)` | prism を導出     |
| `prism.getOption(s)`                 | `Prism::preview`     | マッチしたら取得 |
| `prism.reverseGet(a)`                | `Prism::review`      | 値から構築       |
| `prism.modify(f)(s)`                 | `Prism::modify`      | マッチしたら変更 |

#### コード例

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

### Optional と Traversal

| Scala (Monocle)          | lambars                               | 説明                   |
| ------------------------ | ------------------------------------- | ---------------------- |
| `Optional[S, A]`         | `Optional` trait                      | 存在しない可能性がある |
| `lens.andThen(prism)`    | `LensComposeExtension::compose_prism` | Lens + Prism           |
| `Traversal[S, A]`        | `Traversal` trait                     | 複数のターゲット       |
| `traversal.getAll(s)`    | `Traversal::get_all`                  | すべての値を取得       |
| `traversal.modify(f)(s)` | `Traversal::modify`                   | すべてを変更           |

#### コード例

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
// すべての給与が 10% 増加
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

// すべての給与を取得
let employees = employees_lens.get(&company);
let all_salaries: Vec<f64> = employees.iter()
    .map(|e| salary_lens.get(e).clone())
    .collect();
// all_salaries = vec![50000.0, 60000.0]

// すべての給与を 10% 増加
let raised_employees: Vec<Employee> = employees_lens.get(&company)
    .iter()
    .map(|e| salary_lens.modify(e.clone(), |s| s * 1.1))
    .collect();
let raised = employees_lens.set(company, raised_employees);
```

### Iso

| Scala (Monocle)              | lambars            | 説明           |
| ---------------------------- | ------------------ | -------------- |
| `Iso[S, A](get)(reverseGet)` | `FunctionIso::new` | iso を作成     |
| `iso.get(s)`                 | `Iso::get`         | 順方向変換     |
| `iso.reverseGet(a)`          | `Iso::reverse_get` | 逆方向変換     |
| `iso.reverse`                | `Iso::reverse`     | 方向を入れ替え |
| `iso1.andThen(iso2)`         | `Iso::compose`     | iso を合成     |

#### コード例

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

// iso を反転
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

// iso を反転
let vec_string_iso = string_vec_iso.reverse();
```

---

## エフェクトシステム

### IO Monad

| Scala (Cats Effect)  | lambars                | 説明                 |
| -------------------- | ---------------------- | -------------------- |
| `IO.pure(a)`         | `IO::pure`             | 純粋な値             |
| `IO.delay(expr)`     | `IO::new`              | 停止されたエフェクト |
| `IO.raiseError(e)`   | `IO::catch` with panic | エラーを発生         |
| `io.flatMap(f)`      | `IO::flat_map`         | エフェクトを連鎖     |
| `io.map(f)`          | `IO::fmap`             | 結果を変換           |
| `io.attempt`         | `IO::catch`            | エラーをキャッチ     |
| `io.unsafeRunSync()` | `IO::run_unsafe`       | エフェクトを実行     |
| `io *> io2`          | `IO::then`             | シーケンス           |
| `io.void`            | `io.fmap(\|_\| ())`    | 結果を破棄           |

#### コード例

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

// エラーハンドリング
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

// catch を使ったエラーハンドリング
let failing: IO<i32> = IO::new(|| panic!("oops"));
let recovered: IO<i32> = IO::catch(failing, |_| 0);
let result = recovered.run_unsafe();
// result = 0
```

### MonadError

| Scala (Cats)         | lambars                            | 説明                                  |
| -------------------- | ---------------------------------- | ------------------------------------- |
| `MonadError[F, E]`   | `MonadError<E>` trait              | エラーハンドリング抽象化              |
| `raiseError(e)`      | `MonadError::throw_error`          | エラーを投げる                        |
| `handleErrorWith(f)` | `MonadError::catch_error`          | キャッチして計算で処理                |
| `handleError(f)`     | `MonadError::handle_error`         | エラーを成功値に変換                  |
| `adaptError(pf)`     | `MonadError::adapt_error`          | 同じ型内でエラーを変換                |
| `recover(pf)`        | `MonadError::recover`              | 部分関数によるリカバリ                |
| `recoverWith(pf)`    | `MonadError::recover_with_partial` | モナディック部分リカバリ              |
| `ensure(p)(e)`       | `MonadError::ensure`               | 述語で検証                            |
| `ensureOr(p)(e)`     | `MonadError::ensure_or`            | 値依存エラーで検証                    |
| `redeem(fe, fa)`     | `MonadError::redeem`               | 成功とエラーの両方を変換              |
| `redeemWith(fe, fa)` | `MonadError::redeem_with`          | モナディック redeem                   |
| `attempt`            | catch_error で手動実装             | エラーを Either/Result としてキャッチ |
| (N/A)                | `MonadErrorExt::map_error`         | エラー型を変換                        |

#### コード例

```scala
// Scala (Cats)
import cats.MonadError
import cats.syntax.monadError._

def validatePositive[F[_]](n: Int)(implicit me: MonadError[F, String]): F[Int] =
  me.ensure(me.pure(n))(_ > 0, s"$n is not positive")

// ensure を使用
val result: Either[String, Int] = validatePositive[Either[String, *]](5)
// result = Right(5)

val failed: Either[String, Int] = validatePositive[Either[String, *]](-1)
// failed = Left("-1 is not positive")

// adaptError を使用
val adapted: Either[String, Int] = Right(42).adaptError {
  case e => s"Context: $e"
}

// redeem を使用
val redeemed: Either[String, String] = Right(42).redeem(
  e => s"Error: $e",
  v => s"Success: $v"
)
// redeemed = Right("Success: 42")
```

```rust
// lambars
use lambars::effect::MonadError;

fn validate_positive(n: i32) -> Result<i32, String> {
    <Result<i32, String>>::ensure(
        Ok(n),
        || format!("{} is not positive", n),
        |&v| v > 0
    )
}

// ensure を使用
let result = validate_positive(5);
// result = Ok(5)

let failed = validate_positive(-1);
// failed = Err("-1 is not positive")

// adapt_error を使用
let computation: Result<i32, String> = Err("error".to_string());
let adapted = <Result<i32, String>>::adapt_error(
    computation,
    |e| format!("Context: {}", e)
);

// redeem を使用
let redeemed = <Result<i32, String>>::redeem(
    Ok(42),
    |e| format!("Error: {}", e),
    |v| format!("Success: {}", v)
);
// redeemed = Ok("Success: 42")
```

### State Monad

| Scala (Cats)          | lambars         | 説明           |
| --------------------- | --------------- | -------------- |
| `State.pure(a)`       | `State::pure`   | 純粋な値       |
| `State.get`           | `State::get`    | 状態を取得     |
| `State.set(s)`        | `State::put`    | 状態を設定     |
| `State.modify(f)`     | `State::modify` | 状態を変更     |
| `State.inspect(f)`    | `State::gets`   | 派生値を取得   |
| `state.run(s).value`  | `State::run`    | 初期状態で実行 |
| `state.runS(s).value` | `State::exec`   | 最終状態を取得 |
| `state.runA(s).value` | `State::eval`   | 結果のみを取得 |

#### コード例

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

| Scala (Cats)         | lambars         | 説明       |
| -------------------- | --------------- | ---------- |
| `Reader.pure(a)`     | `Reader::pure`  | 純粋な値   |
| `Reader.ask`         | `Reader::ask`   | 環境を取得 |
| `Reader.local(f)(r)` | `Reader::local` | 環境を変更 |
| `reader.run(env)`    | `Reader::run`   | 環境で実行 |

#### コード例

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

| Scala (Cats)      | lambars          | 説明                   |
| ----------------- | ---------------- | ---------------------- |
| `Writer.pure(a)`  | `Writer::pure`   | 純粋な値               |
| `Writer.tell(w)`  | `Writer::tell`   | 出力をログ             |
| `Writer.value(a)` | `Writer::pure`   | pure のエイリアス      |
| `writer.run`      | `Writer::run`    | (結果, ログ) を取得    |
| `writer.listen`   | `Writer::listen` | 計算内でログにアクセス |

#### コード例

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

### RWS Monad (ReaderWriterState)

| Scala (Cats)                   | lambars       | 説明                   |
| ------------------------------ | ------------- | ---------------------- |
| `ReaderWriterState.pure(a)`    | `RWS::pure`   | 純粋な値               |
| `ReaderWriterState.ask`        | `RWS::ask`    | 環境を取得             |
| `ReaderWriterState.get`        | `RWS::get`    | 状態を取得             |
| `ReaderWriterState.set(s)`     | `RWS::put`    | 状態を設定             |
| `ReaderWriterState.modify(f)`  | `RWS::modify` | 状態を変更             |
| `ReaderWriterState.inspect(f)` | `RWS::gets`   | 派生値を取得           |
| `ReaderWriterState.tell(w)`    | `RWS::tell`   | ログに追加             |
| `rws.local(f)`                 | `RWS::local`  | 環境をローカルに変更   |
| `rws.listen`                   | `RWS::listen` | 計算内でログにアクセス |
| `rws.run(env, state)`          | `RWS::run`    | 環境と状態で実行       |

#### コード例

```scala
// Scala (Cats)
import cats.data.ReaderWriterState
import cats.data.RWS  // type alias

case class Config(multiplier: Int)
type Log = Vector[String]

val computation: RWS[Config, Log, Int, Int] = for {
  config <- ReaderWriterState.ask[Config, Log, Int]
  state  <- ReaderWriterState.get[Config, Log, Int]
  result = state * config.multiplier
  _      <- ReaderWriterState.set[Config, Log, Int](result)
  _      <- ReaderWriterState.tell[Config, Log, Int](Vector(s"Multiplied $state by ${config.multiplier}"))
} yield result

val (log, finalState, result) = computation.run(Config(2), 10).value
// log = Vector("Multiplied 10 by 2"), finalState = 20, result = 20
```

```rust
// lambars
use lambars::effect::RWS;
use lambars::eff;

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

`RWS` モナドは 3 つの機能すべてが必要な場合に便利です：

- **Reader**：共有設定や環境へのアクセス
- **Writer**：ログやその他のモノイダルな出力の蓄積
- **State**：計算を通じた可変状態の管理

---

## モナド変換子

### 比較

| Scala (Cats)       | lambars            | 説明          |
| ------------------ | ------------------ | ------------- |
| `OptionT[F, A]`    | カスタム実装       | F 内の Option |
| `EitherT[F, E, A]` | `ExceptT<E, F, A>` | F 内の Either |
| `StateT[F, S, A]`  | `StateT<S, F, A>`  | F 内の State  |
| `ReaderT[F, R, A]` | `ReaderT<R, F, A>` | F 内の Reader |
| `WriterT[F, W, A]` | `WriterT<W, F, A>` | F 内の Writer |
| `Kleisli[F, A, B]` | `ReaderT<A, F, B>` | 関数ラッパー  |

### コード例

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

### トランスフォーマーでの AsyncIO サポート

lambars は、Cats Effect の非同期機能と同様に、モナド変換子の AsyncIO 統合を提供します。

```rust
// lambars - AsyncIO を使った ReaderT（async feature が必要）
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

利用可能な AsyncIO メソッド：

- `ReaderT`：`ask_async_io`、`asks_async_io`、`lift_async_io`、`pure_async_io`、`flat_map_async_io`
- `StateT`：`get_async_io`、`gets_async_io`、`state_async_io`、`lift_async_io`、`pure_async_io`
- `WriterT`：`tell_async_io`、`lift_async_io`、`pure_async_io`、`flat_map_async_io`、`listen_async_io`

---

## 永続コレクション

### 比較

| Scala             | lambars                   | 説明                           |
| ----------------- | ------------------------- | ------------------------------ |
| `List[A]`         | `PersistentList<A>`       | イミュータブルリスト           |
| `Vector[A]`       | `PersistentVector<A>`     | イミュータブルベクター         |
| `Queue[A]`        | `PersistentDeque<A>`      | イミュータブル両端キュー       |
| `Map[K, V]`       | `PersistentHashMap<K, V>` | イミュータブルハッシュマップ   |
| `SortedMap[K, V]` | `PersistentTreeMap<K, V>` | イミュータブルソート済みマップ |
| `Set[A]`          | `PersistentHashSet<A>`    | イミュータブルセット           |
| `Set[A].view`     | `HashSetView<A>`          | セット上の遅延ビュー           |

### List/Vector 操作

| Scala                 | lambars                              | 説明                               |
| --------------------- | ------------------------------------ | ---------------------------------- |
| `list.take(n)`        | `PersistentList::take`               | 最初の n 要素を取る                |
| `list.drop(n)`        | `PersistentList::drop_first`         | 最初の n 要素を除く                |
| `list.splitAt(n)`     | `PersistentList::split_at`           | インデックスで分割                 |
| `list.zip(other)`     | `PersistentList::zip`                | 2 つのリストを zip                 |
| `list.unzip`          | `PersistentList::<(A,B)>::unzip`     | ペアのリストを unzip               |
| `list.indexWhere(p)`  | `PersistentList::find_index`         | 最初にマッチするインデックスを検索 |
| `list.reduceLeft(f)`  | `PersistentList::fold_left1`         | 初期値なしの左畳み込み             |
| `list.reduceRight(f)` | `PersistentList::fold_right1`        | 初期値なしの右畳み込み             |
| `list.scanLeft(z)(f)` | `PersistentList::scan_left`          | 初期値ありの左スキャン             |
| `list.partition(p)`   | `PersistentList::partition`          | 述語で分割                         |
| `list.mkString(sep)`  | `PersistentList::intersperse`        | 要素間に挿入                       |
| `lists.mkString(sep)` | `PersistentList::intercalate`        | リスト間にリストを挿入して平坦化   |
| `vec.take(n)`         | `PersistentVector::take`             | 最初の n 要素を取る                |
| `vec.drop(n)`         | `PersistentVector::drop_first`       | 最初の n 要素を除く                |
| `vec.splitAt(n)`      | `PersistentVector::split_at`         | インデックスで分割                 |
| `vec.zip(other)`      | `PersistentVector::zip`              | 2 つのベクターを zip               |
| `vec.unzip`           | `PersistentVector::<(A,B)>::unzip`   | ペアのベクターを unzip             |
| `vec.indexWhere(p)`   | `PersistentVector::find_index`       | 最初にマッチするインデックスを検索 |
| `vec.reduceLeft(f)`   | `PersistentVector::fold_left1`       | 初期値なしの左畳み込み             |
| `vec.reduceRight(f)`  | `PersistentVector::fold_right1`      | 初期値なしの右畳み込み             |
| `vec.scanLeft(z)(f)`  | `PersistentVector::scan_left`        | 初期値ありの左スキャン             |
| `vec.partition(p)`    | `PersistentVector::partition`        | 述語で分割                         |
| `Ordering[List[A]]`   | `Ord` for `PersistentList<A: Ord>`   | 辞書順序                           |
| `Ordering[Vector[A]]` | `Ord` for `PersistentVector<A: Ord>` | 辞書順序                           |

### Map 操作

| Scala                                                 | lambars                         | 説明                                     |
| ----------------------------------------------------- | ------------------------------- | ---------------------------------------- |
| `map.mapValues(f)`                                    | `PersistentHashMap::map_values` | 値を変換                                 |
| `map.transform((k, v) => f(k, v))`                    | `PersistentHashMap::map_values` | 値を変換（クロージャ内でキーが利用可能） |
| `map.map { case (k, v) => (f(k), v) }`                | `PersistentHashMap::map_keys`   | キーを変換                               |
| `map.collect { case (k, v) if p(k, v) => (k, f(v)) }` | `PersistentHashMap::filter_map` | フィルタして変換                         |
| `map.toList`                                          | `PersistentHashMap::entries`    | すべてのエントリを取得                   |
| `map.keys`                                            | `PersistentHashMap::keys`       | すべてのキーを取得                       |
| `map.values`                                          | `PersistentHashMap::values`     | すべての値を取得                         |
| `map1 ++ map2`                                        | `PersistentHashMap::merge`      | マージ（右が優先）                       |
| `map1.merged(map2)((k, v1, v2) => f(k, v1, v2))`      | `PersistentHashMap::merge_with` | リゾルバー付きマージ                     |
| `map.filter { case (k, v) => p(k, v) }`               | `PersistentHashMap::keep_if`    | マッチするエントリを保持                 |
| `map.filterNot { case (k, v) => p(k, v) }`            | `PersistentHashMap::delete_if`  | マッチするエントリを削除                 |
| `map.partition { case (k, v) => p(k, v) }`            | `PersistentHashMap::partition`  | 述語で分割                               |

### コード例

```scala
// Scala - List
val list = List(1, 2, 3)
val prepended = 0 :: list
// list = List(1, 2, 3) (変更なし)
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
// list.len() = 3 (変更なし)
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
// vec = Vector(1, 2, 3) (変更なし)
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
// vec.get(1) = Some(&2) (変更なし)
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
// map = Map("a" -> 1, "b" -> 2) (変更なし)
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
// map.get("c") = None (変更なし)
// updated.get("c") = Some(&3)

let value: Option<&i32> = map.get("a");
// value = Some(&1)

let removed = map.remove("a");
// removed.get("a") = None
```

---

## 並列コレクション

Scala は `.par` を通じて並列コレクションを提供し、lambars は並列反復のために rayon と統合します。

### 比較

| Scala                      | lambars                                    | 説明                   |
| -------------------------- | ------------------------------------------ | ---------------------- |
| `collection.par`           | `into_par_iter()` (`rayon` feature が必要) | 並列コレクションに変換 |
| `collection.par.map(f)`    | `par_iter().map(f)`                        | 並列マップ             |
| `collection.par.filter(p)` | `par_iter().filter(p)`                     | 並列フィルタ           |
| `collection.par.reduce(f)` | `par_iter().reduce(identity, f)`           | 並列畳み込み           |
| `collection.par.sum`       | `par_iter().sum()`                         | 並列合計               |
| `collection.par.find(p)`   | `par_iter().find_any(p)`                   | 検索（非決定的）       |
| `collection.par.forall(p)` | `par_iter().all(p)`                        | すべてが述語にマッチ   |
| `collection.par.exists(p)` | `par_iter().any(p)`                        | いずれかが述語にマッチ |

### コード例

```scala
// Scala
import scala.collection.parallel.CollectionConverters._

val numbers = Vector.range(0, 10000)
val doubled = numbers.par.map(_ * 2).toVector
val sum = numbers.par.sum
val filtered = numbers.par.filter(_ % 2 == 0).toVector
```

```rust
// lambars（`rayon` feature が必要）
use lambars::persistent::PersistentVector;
use rayon::prelude::*;

let numbers: PersistentVector<i32> = (0..10000).collect();
let doubled: Vec<i32> = numbers.par_iter().map(|x| x * 2).collect();
let sum: i32 = numbers.par_iter().sum();
let filtered: Vec<i32> = numbers.par_iter().filter(|x| *x % 2 == 0).cloned().collect();

// 元のベクターは変更されていない
assert_eq!(numbers.len(), 10000);
```

### 順序に関する注意

Scala と lambars の両方で、並列操作は特定の操作で順序を保持しない場合があります：

- `filter`：順序が保持されない場合がある（順序のためには逐次版を使用）
- `find_any`：マッチする任意の要素を返す（非決定的）
- `reduce`：結果は操作の結合性に依存

---

## 遅延評価

| Scala               | lambars                | 説明           |
| ------------------- | ---------------------- | -------------- |
| `lazy val x = expr` | `Lazy::new(\|\| expr)` | 遅延値         |
| `x` (force)         | `Lazy::force`          | 評価を強制     |
| `LazyList`          | `Iterator`             | 遅延シーケンス |

### コード例

```scala
// Scala
lazy val expensive = {
  println("Computing...")
  42
}
// まだ何も出力されない

val result = expensive
// "Computing..." が出力され、result = 42

val result2 = expensive
// 何も出力されない（キャッシュされている）、result2 = 42
```

```rust
// lambars
use lambars::control::Lazy;

let expensive = Lazy::new(|| {
    println!("Computing...");
    42
});
// まだ何も出力されない

let result = expensive.force();
// "Computing..." が出力され、result = 42

let result2 = expensive.force();
// 何も出力されない（キャッシュされている）、result2 = 42
```

```scala
// Scala - LazyList（旧 Stream）
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

### 型クラスインスタンス

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

### 拡張メソッド

```scala
// Scala - implicit による拡張メソッド
implicit class IntOps(val n: Int) extends AnyVal {
  def isEven: Boolean = n % 2 == 0
  def squared: Int = n * n
}

val result = 4.squared  // 16
val even = 4.isEven     // true
```

```rust
// Rust - 拡張 trait
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

## まとめ：主な違い

### 構文の違い

| 側面                  | Scala                     | lambars (Rust)               |
| --------------------- | ------------------------- | ---------------------------- |
| for 内包表記（Monad） | `for { x <- mx } yield x` | `eff! { x <= mx; expr }`     |
| for 内包表記（List）  | `for { x <- xs } yield x` | `for_! { x <= xs; yield x }` |
| 型パラメータ          | `F[A]`                    | `F<A>`                       |
| Option                | `Some(x)` / `None`        | `Some(x)` / `None`           |
| Either                | `Right(x)` / `Left(e)`    | `Ok(x)` / `Err(e)`           |
| ラムダ                | `x => x + 1`              | `\|x\| x + 1`                |
| メソッド呼び出し      | `obj.method(arg)`         | `obj.method(arg)`            |
| 型注釈                | `x: Int`                  | `x: i32`                     |
| trait 定義            | `trait T { }`             | `trait T { }`                |
| Implicit              | `implicit val`            | N/A（明示的な trait）        |

### 概念の違い

1. **Implicits vs 明示的**：Scala は型クラスインスタンスに implicit を使用。Rust は明示的な trait 実装とインポートが必要。

2. **高カインド型**：Scala はネイティブ HKT サポート。lambars は GAT を使用して HKT をエミュレート。

3. **変性**：Scala は宣言サイト変性（`+A`、`-A`）。Rust は変性のために `PhantomData` を使用。

4. **Null 安全性**：Scala には `null`（Java から）がある。Rust には null がなく、`Option` のみ。

5. **エラーハンドリング**：Scala は `Either[E, A]`（左 = エラー）を使用。Rust は `Result<T, E>`（デフォルトで右バイアス）を使用。

6. **所有権**：Rust には所有権/借用がある。Scala にはガベージコレクションがある。

7. **パターンマッチング**：両方ともパターンマッチングをサポートしているが、Rust は網羅的マッチングが必要。

---

## 移行のヒント

1. **`for` 内包表記を `eff!` または `for_!` に置き換える**：バインディングには `<-` の代わりに `<=` を使用。

   - モナド（Option、Result、IO、State、Reader、Writer）には `eff!` を使用
   - リスト（Vec）には `yield` キーワード付きで `for_!` を使用

2. **List 内包表記には `for_!` を使用**：Scala の List for 内包表記を翻訳する際は、`yield` 付きの `for_!` を使用。Rust の所有権により、内部コレクションには `.clone()` が必要なことを忘れずに。

3. **`Either` を `Result` に置き換える**：型パラメータの順序の違いに注意（`Either[E, A]` vs `Result<A, E>`）。

4. **trait を明示的にインポート**：Scala の implicit と異なり、Rust の trait はそのメソッドを使用するために明示的にインポートする必要がある。

5. **`.map()` の代わりに `.fmap()` を使用**：型クラスベースのマッピングの場合（std の `Option/Result` も `.map()` を持っている）。

6. **`.flatMap()` の代わりに `.flat_map()` を使用**：スネークケース命名規則。

7. **明示的な型注釈を追加**：Rust の型推論は Scala ほど積極的ではない。

8. **所有権を処理**：必要に応じて値をクローンするか、参照を適切に使用。

9. **`PersistentVector` を Scala の `Vector` に使用**：構造共有による類似したパフォーマンス特性。
