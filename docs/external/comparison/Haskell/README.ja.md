# Haskell から lambars への API 対応ガイド

[English](README.en.md)

> **Note**: このドキュメントは AI によって翻訳されました。誤りや不自然な表現がある場合は、Issue または Pull Request でお知らせください。

このドキュメントは、Haskell の関数型プログラミング構造と lambars (Rust) における同等の機能との包括的な対応関係を提供します。Haskell は正統な純粋関数型プログラミング言語であり、lambars はその多くの抽象化を Rust にもたらすことを目指しています。

## 目次

- [概要](#概要)
- [型クラス](#型クラス)
  - [Functor](#functor)
  - [Applicative](#applicative)
  - [Monad](#monad)
  - [Semigroup と Monoid](#semigroup-と-monoid)
  - [Foldable](#foldable)
  - [Traversable](#traversable)
- [Maybe と Either](#maybe-と-either)
- [Do 記法とリスト内包表記](#do記法とリスト内包表記)
- [関数合成](#関数合成)
- [カリー化と部分適用](#カリー化と部分適用)
- [遅延評価](#遅延評価)
- [Optics (lens)](#optics-lens)
- [エフェクトモナド](#エフェクトモナド)
  - [IO Monad](#io-monad)
  - [State Monad](#state-monad)
  - [Reader Monad](#reader-monad)
  - [Writer Monad](#writer-monad)
  - [RWS Monad](#rws-monad)
- [モナド変換子 (mtl)](#モナド変換子-mtl)
- [代数的エフェクト](#代数的エフェクト)
- [データ構造](#データ構造)
- [パターンマッチング](#パターンマッチング)
- [高カインド型](#高カインド型)
- [代数的データ型](#代数的データ型)

---

## 概要

| 概念                     | Haskell                          | lambars (Rust)                             |
| ------------------------ | -------------------------------- | ------------------------------------------ | -------------- |
| Functor                  | `Functor f`                      | `Functor` trait                            |
| Applicative              | `Applicative f`                  | `Applicative` trait                        |
| Monad                    | `Monad m`                        | `Monad` trait                              |
| Semigroup                | `Semigroup a`                    | `Semigroup` trait                          |
| Monoid                   | `Monoid a`                       | `Monoid` trait                             |
| Foldable                 | `Foldable t`                     | `Foldable` trait                           |
| Traversable              | `Traversable t`                  | `Traversable` trait                        |
| Maybe                    | `Maybe a`                        | `Option<A>` (std)                          |
| Either                   | `Either e a`                     | `Result<A, E>` (std)                       |
| Do 記法 (Monad)          | `do { ... }`                     | `eff!` マクロ                              |
| リスト内包表記           | `[x                              | x <- xs]`                                  | `for_!` マクロ |
| 非同期リスト内包表記     | `do` + `async` / `ListT IO`      | `for_async!` マクロ                        |
| 関数合成                 | `.` と `>>>`                     | `compose!` マクロ                          |
| パイプ                   | `&`                              | `pipe!` マクロ                             |
| Lens                     | `Control.Lens`                   | `Lens` trait                               |
| Prism                    | `Control.Lens.Prism`             | `Prism` trait                              |
| IO                       | `IO a`                           | `IO<A>` 型                                 |
| State                    | `State s a`                      | `State<S, A>` 型                           |
| Reader                   | `Reader r a`                     | `Reader<R, A>` 型                          |
| Writer                   | `Writer w a`                     | `Writer<W, A>` 型                          |
| RWS                      | `RWS r w s a`                    | `RWS<R, W, S, A>` 型                       |
| StateT                   | `StateT s m a`                   | `StateT<S, M, A>` 型                       |
| ReaderT                  | `ReaderT r m a`                  | `ReaderT<R, M, A>` 型                      |
| WriterT                  | `WriterT w m a`                  | `WriterT<W, M, A>` 型                      |
| ExceptT                  | `ExceptT e m a`                  | `ExceptT<E, M, A>` 型                      |
| Identity                 | `Identity a`                     | `Identity<A>` 型                           |
| 代数的エフェクト         | `Eff '[e1, e2] a` (freer-simple) | `Eff<EffCons<E1, EffCons<E2, EffNil>>, A>` |
| エフェクトメンバーシップ | `Member e r`                     | `Member<E, Index>` trait                   |
| 遅延評価                 | デフォルト (サンク)              | `Lazy<A>` 型                               |
| トランポリン             | トランポリン処理                 | `Trampoline<A>` 型                         |

---

## 型クラス

### Functor

| Haskell     | lambars             | 説明                     |
| ----------- | ------------------- | ------------------------ |
| `fmap f fa` | `Functor::fmap`     | Functor 上で関数をマップ |
| `f <$> fa`  | `fa.fmap(f)`        | 中置 fmap                |
| `fa $> b`   | `fa.fmap(\|_\| b)`  | 定数で置換               |
| `void fa`   | `fa.fmap(\|_\| ())` | 値を破棄                 |
| `fa <$ b`   | `fa.fmap(\|_\| b)`  | 構造を保持して置換       |

#### Functor の法則

```
1. 恒等性:     fmap id == id
2. 合成性:     fmap (f . g) == fmap f . fmap g
```

#### コード例

```haskell
-- Haskell
import Data.Functor

doubled :: Maybe Int
doubled = fmap (*2) (Just 21)
-- doubled = Just 42

-- 中置演算子の使用
doubled' :: Maybe Int
doubled' = (*2) <$> Just 21
-- doubled' = Just 42

-- リスト Functor
doubledList :: [Int]
doubledList = fmap (*2) [1, 2, 3]
-- doubledList = [2, 4, 6]

-- 値の置換
replaced :: Maybe String
replaced = Just 42 $> "hello"
-- replaced = Just "hello"
```

```rust
// lambars
use lambars::typeclass::Functor;

let doubled: Option<i32> = Some(21).fmap(|x| x * 2);
// doubled = Some(42)

// リスト (Vec) Functor
let doubled_list: Vec<i32> = vec![1, 2, 3].fmap(|x| x * 2);
// doubled_list = vec![2, 4, 6]

// 値の置換
let replaced: Option<String> = Some(42).fmap(|_| "hello".to_string());
// replaced = Some("hello".to_string())
```

### Applicative

| Haskell             | lambars                   | 説明                     |
| ------------------- | ------------------------- | ------------------------ |
| `pure a`            | `Applicative::pure`       | コンテキストに値をリフト |
| `ff <*> fa`         | `Applicative::apply`      | ラップされた関数を適用   |
| `liftA2 f fa fb`    | `Applicative::map2`       | 二項関数をリフト         |
| `liftA3 f fa fb fc` | `Applicative::map3`       | 三項関数をリフト         |
| `fa *> fb`          | `fa.map2(fb, \|_, b\| b)` | 順序付け、右側を保持     |
| `fa <* fb`          | `fa.map2(fb, \|a, _\| a)` | 順序付け、左側を保持     |
| `(,) <$> fa <*> fb` | `Applicative::product`    | 値をペア化               |

#### Applicative の法則

```
1. 恒等性:        pure id <*> v == v
2. 合成性:        pure (.) <*> u <*> v <*> w == u <*> (v <*> w)
3. 準同型性:      pure f <*> pure x == pure (f x)
4. 交換性:        u <*> pure y == pure ($ y) <*> u
```

#### コード例

```haskell
-- Haskell
import Control.Applicative

x :: Maybe Int
x = pure 42
-- x = Just 42

-- コンテキスト内で関数を適用
result :: Maybe Int
result = Just (+) <*> Just 10 <*> Just 20
-- result = Just 30

-- liftA2 の使用
sum :: Maybe Int
sum = liftA2 (+) (Just 10) (Just 20)
-- sum = Just 30

-- liftA3
sum3 :: Maybe Int
sum3 = liftA3 (\a b c -> a + b + c) (Just 1) (Just 2) (Just 3)
-- sum3 = Just 6

-- ペア化
paired :: Maybe (Int, String)
paired = (,) <$> Just 42 <*> Just "hello"
-- paired = Just (42, "hello")

-- 順序演算子
sequenceRight :: Maybe Int
sequenceRight = Just 10 *> Just 20
-- sequenceRight = Just 20
```

```rust
// lambars
use lambars::typeclass::Applicative;

let x: Option<i32> = <Option<()>>::pure(42);
// x = Some(42)

// map2 の使用
let sum: Option<i32> = Some(10).map2(Some(20), |a, b| a + b);
// sum = Some(30)

// map3 の使用
let sum3: Option<i32> = Some(1).map3(Some(2), Some(3), |a, b, c| a + b + c);
// sum3 = Some(6)

// product でペア化
let paired: Option<(i32, String)> = Some(42).product(Some("hello".to_string()));
// paired = Some((42, "hello".to_string()))

// 順序付け (右側を保持)
let sequence_right: Option<i32> = Some(10).map2(Some(20), |_, b| b);
// sequence_right = Some(20)
```

### Monad

| Haskell         | lambars                                      | 説明                        |
| --------------- | -------------------------------------------- | --------------------------- |
| `return a`      | `Monad::pure` (Applicative 経由)             | Monad にリフト              |
| `ma >>= f`      | `Monad::flat_map`                            | バインド操作                |
| `ma >> mb`      | `Monad::then`                                | 順序付け、右側を保持        |
| `join mma`      | `Flatten::flatten` / `Option::flatten` (std) | ネストされた Monad を平坦化 |
| `ma =<< f`      | `f(ma.run())`                                | 逆バインド                  |
| `>=>` (Kleisli) | 手動合成                                     | Monad 関数の合成            |
| `<=<` (Kleisli) | 手動合成                                     | 逆 Kleisli 合成             |

#### Monad の法則

```
1. 左単位元:     return a >>= f  ==  f a
2. 右単位元:     m >>= return    ==  m
3. 結合性:       (m >>= f) >>= g ==  m >>= (\x -> f x >>= g)
```

#### コード例

```haskell
-- Haskell
import Control.Monad

-- 安全な除算
safeDivide :: Int -> Int -> Maybe Int
safeDivide _ 0 = Nothing
safeDivide x y = Just (x `div` y)

-- バインド操作
result :: Maybe Int
result = Just 10 >>= \x -> safeDivide x 2
-- result = Just 5

-- チェーン
chained :: Maybe Int
chained = Just 100 >>= safeDivide 10 >>= safeDivide 2
-- chained = Just 5

-- join の使用
nested :: Maybe (Maybe Int)
nested = Just (Just 42)

flattened :: Maybe Int
flattened = join nested
-- flattened = Just 42

-- 順序付け
sequenced :: Maybe Int
sequenced = Just 10 >> Just 20
-- sequenced = Just 20

-- Kleisli 合成
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

// 安全な除算
fn safe_divide(x: i32, y: i32) -> Option<i32> {
    if y == 0 { None } else { Some(x / y) }
}

// バインド操作
let result: Option<i32> = Some(10).flat_map(|x| safe_divide(x, 2));
// result = Some(5)

// チェーン
let chained: Option<i32> = Some(100)
    .flat_map(|x| safe_divide(10, x))
    .flat_map(|x| safe_divide(x, 2));
// 注: safe_divide(10, 100) = 0 で、safe_divide(0, 2) = 0 なので None が返る

// Flatten trait の使用 (std の Option::flatten でも動作)
use lambars::typeclass::Flatten;
let nested: Option<Option<i32>> = Some(Some(42));
let flattened: Option<i32> = nested.flatten();
// flattened = Some(42)

// Flatten は Result、Box、Identity でも動作
let nested_result: Result<Result<i32, &str>, &str> = Ok(Ok(42));
let flattened_result: Result<i32, &str> = nested_result.flatten();
// flattened_result = Ok(42)

// then で順序付け
let sequenced: Option<i32> = Some(10).then(Some(20));
// sequenced = Some(20)

// Kleisli 風の合成 (手動)
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

### Semigroup と Monoid

| Haskell          | lambars                            | 説明                    |
| ---------------- | ---------------------------------- | ----------------------- |
| `a <> b`         | `Semigroup::combine`               | 二つの値を結合          |
| `sconcat`        | `Semigroup::combine` (fold された) | 非空リストを結合        |
| `mempty`         | `Monoid::empty`                    | 単位元                  |
| `mconcat`        | `Monoid::combine_all`              | mappend でリストを fold |
| `mappend`        | `Semigroup::combine`               | `<>` と同じ             |
| `Sum`, `Product` | `Sum`, `Product`                   | 数値ラッパー            |
| `Min`, `Max`     | `Min`, `Max`                       | 境界付きラッパー        |
| `First`, `Last`  | カスタム実装                       | 最初/最後の非 Nothing   |
| `Endo`           | カスタム実装                       | 自己準同型 Monoid       |
| `Dual`           | カスタム実装                       | 逆 Monoid               |

#### Semigroup/Monoid の法則

```
Semigroup:
  結合性: (a <> b) <> c == a <> (b <> c)

Monoid:
  左単位元:  mempty <> a == a
  右単位元:  a <> mempty == a
```

#### コード例

```haskell
-- Haskell
import Data.Semigroup
import Data.Monoid

-- 文字列の連結
combined :: String
combined = "Hello, " <> "World!"
-- combined = "Hello, World!"

-- リストの連結
listCombined :: [Int]
listCombined = [1, 2] <> [3, 4]
-- listCombined = [1, 2, 3, 4]

-- Sum Monoid の使用
sumResult :: Sum Int
sumResult = mconcat [Sum 1, Sum 2, Sum 3, Sum 4, Sum 5]
-- sumResult = Sum 15

-- Product Monoid の使用
productResult :: Product Int
productResult = mconcat [Product 1, Product 2, Product 3, Product 4, Product 5]
-- productResult = Product 120

-- Max と Min の使用
maxResult :: Max Int
maxResult = mconcat [Max 3, Max 1, Max 4, Max 1, Max 5]
-- maxResult = Max 5

-- Monoid としての Option/Maybe (内部 Semigroup と共に)
maybeResult :: Maybe String
maybeResult = Just "Hello" <> Just " World"
-- maybeResult = Just "Hello World"
```

```rust
// lambars
use lambars::typeclass::{Semigroup, Monoid, Sum, Product, Max, Min};

// 文字列の連結
let combined: String = "Hello, ".to_string().combine("World!".to_string());
// combined = "Hello, World!"

// Vec の連結
let list_combined: Vec<i32> = vec![1, 2].combine(vec![3, 4]);
// list_combined = vec![1, 2, 3, 4]

// Sum Monoid の使用
let items = vec![Sum::new(1), Sum::new(2), Sum::new(3), Sum::new(4), Sum::new(5)];
let sum_result: Sum<i32> = Sum::combine_all(items);
// sum_result = Sum(15)

// Product Monoid の使用
let items = vec![Product::new(1), Product::new(2), Product::new(3), Product::new(4), Product::new(5)];
let product_result: Product<i32> = Product::combine_all(items);
// product_result = Product(120)

// Max の使用
let items = vec![Max::new(3), Max::new(1), Max::new(4), Max::new(1), Max::new(5)];
let max_result: Max<i32> = Max::combine_all(items);
// max_result = Max(5)

// Semigroup としての Option
let maybe_result: Option<String> = Some("Hello".to_string())
    .combine(Some(" World".to_string()));
// maybe_result = Some("Hello World")
```

### Foldable

| Haskell       | lambars                     | 説明                       |
| ------------- | --------------------------- | -------------------------- |
| `foldl f z t` | `Foldable::fold_left`       | 左畳み込み                 |
| `foldr f z t` | `Foldable::fold_right`      | 右畳み込み                 |
| `foldMap f t` | `Foldable::fold_map`        | マップしてから畳み込み     |
| `fold t`      | `Foldable::fold`            | Monoid で畳み込み          |
| `length t`    | `Foldable::length`          | 要素数をカウント           |
| `null t`      | `Foldable::is_empty`        | 空チェック                 |
| `elem x t`    | 手動実装                    | 要素のメンバーシップ       |
| `find p t`    | `Foldable::find`            | 最初にマッチした要素を検索 |
| `any p t`     | `Foldable::exists`          | いずれかの要素がマッチ     |
| `all p t`     | `Foldable::for_all`         | 全ての要素がマッチ         |
| `toList t`    | `Foldable::to_vec`          | リストに変換               |
| `sum t`       | `fold_left(0, \|a,b\| a+b)` | 要素の合計                 |
| `product t`   | `fold_left(1, \|a,b\| a*b)` | 要素の積                   |
| `maximum t`   | `fold_left` で max を使用   | 最大要素                   |
| `minimum t`   | `fold_left` で min を使用   | 最小要素                   |

#### コード例

```haskell
-- Haskell
import Data.Foldable

-- 左畳み込み
sumList :: Int
sumList = foldl (+) 0 [1, 2, 3, 4, 5]
-- sumList = 15

-- 右畳み込み
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

-- any と all
hasEven :: Bool
hasEven = any even [1, 2, 3]
-- hasEven = True

allPositive :: Bool
allPositive = all (> 0) [1, 2, 3]
-- allPositive = True

-- length と null
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

// 左畳み込み
let sum_list: i32 = vec![1, 2, 3, 4, 5].fold_left(0, |acc, x| acc + x);
// sum_list = 15

// 右畳み込み
let product_list: i32 = vec![1, 2, 3, 4, 5].fold_right(1, |x, acc| x * acc);
// product_list = 120

// fold_map
let concat_strings: String = vec![1, 2, 3].fold_map(|x| x.to_string());
// concat_strings = "123"

// find
let found: Option<&i32> = vec![1, 2, 3, 4, 5].find(|x| **x > 3);
// found = Some(&4)

// exists と for_all
let has_even: bool = vec![1, 2, 3].exists(|x| x % 2 == 0);
// has_even = true

let all_positive: bool = vec![1, 2, 3].for_all(|x| *x > 0);
// all_positive = true

// length と is_empty
let list_length: usize = vec![1, 2, 3, 4, 5].length();
// list_length = 5

let is_empty: bool = Vec::<i32>::new().is_empty();
// is_empty = true
```

### Traversable

| Haskell                 | lambars                               | 説明                          |
| ----------------------- | ------------------------------------- | ----------------------------- |
| `traverse f t`          | `Traversable::traverse_option/result` | エフェクトと共に走査          |
| `sequenceA t`           | `Traversable::sequence_option/result` | エフェクトの順序付け          |
| `for t f`               | `traverse` で引数を反転               | 走査 (引数反転)               |
| `mapM f t`              | `traverse_option/result`              | traverse と同じ (Monad 用)    |
| `sequence t`            | `sequence_option/result`              | sequenceA と同じ (Monad 用)   |
| `forM t f`              | `traverse` 反転                       | for と同じ (Monad 用)         |
| `traverse @(Reader r)`  | `Traversable::traverse_reader`        | Reader エフェクトで走査       |
| `traverse @(State s)`   | `Traversable::traverse_state`         | State エフェクトで走査        |
| `traverse @IO`          | `Traversable::traverse_io`            | IO エフェクトで走査           |
| `traverse @(Async IO)`  | `Traversable::traverse_async_io`      | AsyncIO エフェクトで走査      |
| `mapConcurrently`       | `Traversable::traverse_async_io_parallel` | AsyncIO エフェクトで並行走査  |
| `sequence @(Reader r)`  | `Traversable::sequence_reader`        | Reader エフェクトの順序付け   |
| `sequence @(State s)`   | `Traversable::sequence_state`         | State エフェクトの順序付け    |
| `sequence @IO`          | `Traversable::sequence_io`            | IO エフェクトの順序付け       |
| `sequence @(Async IO)`  | `Traversable::sequence_async_io`      | AsyncIO エフェクトの順序付け  |
| `traverse_ @(Reader r)` | `Traversable::traverse_reader_`       | Reader を走査、結果を破棄     |
| `traverse_ @(State s)`  | `Traversable::traverse_state_`        | State を走査、結果を破棄      |
| `traverse_ @IO`         | `Traversable::traverse_io_`           | IO を走査、結果を破棄         |
| `for_ @(Reader r)`      | `Traversable::for_each_reader`        | traverse*reader* のエイリアス |
| `for_ @(State s)`       | `Traversable::for_each_state`         | traverse*state* のエイリアス  |
| `for_ @IO`              | `Traversable::for_each_io`            | traverse*io* のエイリアス     |

#### Traversable の法則

```
1. 自然性:     t . traverse f == traverse (t . f)  (任意の Applicative 変換 t に対して)
2. 恒等性:     traverse Identity == Identity
3. 合成性:     traverse (Compose . fmap g . f) == Compose . fmap (traverse g) . traverse f
```

#### コード例

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

-- Either との組み合わせ
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

// Result との組み合わせ
fn parse_int_result(s: &str) -> Result<i32, String> {
    s.parse().map_err(|_| format!("Cannot parse: {}", s))
}

let result_ok: Result<Vec<i32>, String> = vec!["1", "2", "3"]
    .traverse_result(parse_int_result);
// result_ok = Ok(vec![1, 2, 3])

let result_err: Result<Vec<i32>, String> = vec!["1", "two", "3"]
    .traverse_result(parse_int_result);
// result_err = Err("Cannot parse: two")

// traverse_reader - Reader エフェクトで走査
use lambars::effect::Reader;

#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
// result = vec![10, 20, 30]

// traverse_state - State エフェクトで走査 (状態は左から右へスレッド)
use lambars::effect::State;

let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
// result = vec![(0, "a"), (1, "b"), (2, "c")]
// final_index = 3

// traverse_io - IO エフェクトで走査 (IO アクションは順次実行)
use lambars::effect::IO;

let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
// contents = vec!["content of a.txt", "content of b.txt"]
```

---

## Maybe と Either

### Maybe / Option

| Haskell          | lambars / Rust std                   | 説明                |
| ---------------- | ------------------------------------ | ------------------- |
| `Just x`         | `Some(x)`                            | Just/Some の構築    |
| `Nothing`        | `None`                               | Nothing/None の構築 |
| `fmap f ma`      | `Functor::fmap`                      | Maybe 上でマップ    |
| `ma >>= f`       | `Monad::flat_map`                    | バインド            |
| `fromMaybe d ma` | `Option::unwrap_or`                  | デフォルト値        |
| `maybe d f ma`   | `Option::map_or`                     | Maybe を畳み込み    |
| `isJust ma`      | `Option::is_some`                    | Just のテスト       |
| `isNothing ma`   | `Option::is_none`                    | Nothing のテスト    |
| `fromJust ma`    | `Option::unwrap`                     | 抽出 (安全でない)   |
| `listToMaybe xs` | `xs.first()`                         | 最初の要素          |
| `maybeToList ma` | `Option::into_iter`                  | リストへ            |
| `catMaybes xs`   | `Iterator::flatten`                  | Nothing をフィルタ  |
| `mapMaybe f xs`  | `Iterator::filter_map`               | マップとフィルタ    |
| `ma <\|> mb`     | `Option::or`                         | 代替                |
| `guard cond`     | `if cond { Some(()) } else { None }` | Monad でのガード    |

### Either / Result

| Haskell            | lambars / Rust std                    | 説明               |
| ------------------ | ------------------------------------- | ------------------ |
| `Right x`          | `Ok(x)`                               | Right/Ok の構築    |
| `Left e`           | `Err(e)`                              | Left/Err の構築    |
| `fmap f ea`        | `Functor::fmap` / `Result::map`       | Right 上でマップ   |
| `first f ea`       | `Result::map_err`                     | Left 上でマップ    |
| `bimap f g ea`     | 手動                                  | 両側をマップ       |
| `ea >>= f`         | `Monad::flat_map`                     | バインド           |
| `either f g ea`    | `Result::map_or_else`                 | Either を畳み込み  |
| `isRight ea`       | `Result::is_ok`                       | Right のテスト     |
| `isLeft ea`        | `Result::is_err`                      | Left のテスト      |
| `fromRight d ea`   | `Result::unwrap_or`                   | Right のデフォルト |
| `fromLeft d ea`    | `Result::err().unwrap_or`             | Left のデフォルト  |
| `rights xs`        | `Iterator::filter_map(\|r\| r.ok())`  | Right をフィルタ   |
| `lefts xs`         | `Iterator::filter_map(\|r\| r.err())` | Left をフィルタ    |
| `partitionEithers` | 手動                                  | 二つのリストに分割 |

#### コード例

```haskell
-- Haskell
import Data.Maybe
import Data.Either

-- Maybe 操作
doubled :: Maybe Int
doubled = fmap (*2) (Just 21)
-- doubled = Just 42

defaulted :: Int
defaulted = fromMaybe 0 Nothing
-- defaulted = 0

-- Either 操作
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

// Option 操作
let doubled: Option<i32> = Some(21).fmap(|x| x * 2);
// doubled = Some(42)

let defaulted: i32 = None.unwrap_or(0);
// defaulted = 0

// Result 操作
let mapped: Result<i32, String> = Ok(21).fmap(|x| x * 2);
// mapped = Ok(42)

let left_mapped: Result<i32, usize> = Err("error".to_string()).map_err(|e| e.len());
// left_mapped = Err(5)

// bimap 相当
let result: Result<i32, String> = Ok(42);
let bi_mapped: Result<String, usize> = result
    .map(|x| x.to_string())
    .map_err(|e| e.len());
// bi_mapped = Ok("42")
```

---

## Do 記法とリスト内包表記

lambars は、Haskell の do 記法とリスト内包表記に対応する二つのマクロを提供します:

| ユースケース   | Haskell               | lambars       | 説明                    |
| -------------- | --------------------- | ------------- | ----------------------- | ---------------------- |
| Monad バインド | `do { x <- mx; ... }` | `eff!` マクロ | 単一実行、FnOnce ベース |
| リスト内包表記 | `[f x                 | x <- xs]`     | `for_!` マクロ          | 複数実行、FnMut ベース |

### eff! マクロ (Monad の Do 記法)

#### 構文比較

| Haskell               | lambars                    | 説明            |
| --------------------- | -------------------------- | --------------- |
| `do { x <- mx; ... }` | `eff! { x <= mx; ... }`    | バインド        |
| `let x = expr`        | `let x = expr;`            | 純粋なバインド  |
| `pure x`              | `Some(x)` / `Ok(x)` / など | 値を返す        |
| `mx >> my`            | `_ <= mx; my`              | 順序付け (破棄) |
| ガード (MonadPlus)    | `_ <= guard(cond);`        | ガード          |

### コード例

```haskell
-- Haskell - do 記法
computation :: Maybe Int
computation = do
  x <- Just 10
  y <- Just 20
  let z = x + y
  pure (z * 2)
-- computation = Just 60

-- ガード付き (MonadPlus/Alternative が必要)
withGuard :: Maybe Int
withGuard = do
  x <- Just 10
  guard (x > 5)
  pure (x * 2)
-- withGuard = Just 20

-- ネストされた計算
nested :: Maybe Int
nested = do
  list <- Just [1, 2, 3]
  first <- listToMaybe list
  let doubled = first * 2
  pure doubled
-- nested = Just 2

-- Either の do 記法
eitherComputation :: Either String Int
eitherComputation = do
  x <- Right 10
  y <- Right 20
  pure (x + y)
-- eitherComputation = Right 30

-- パターンマッチング付き
withPattern :: Maybe (Int, Int)
withPattern = do
  (a, b) <- Just (10, 20)
  pure (a + b, a * b)
-- withPattern = Just (30, 200)
```

```rust
// lambars - eff! マクロ
use lambars::eff;

let computation: Option<i32> = eff! {
    x <= Some(10);
    y <= Some(20);
    let z = x + y;
    Some(z * 2)
};
// computation = Some(60)

// ガード風のパターン
fn guard(condition: bool) -> Option<()> {
    if condition { Some(()) } else { None }
}

let with_guard: Option<i32> = eff! {
    x <= Some(10);
    _ <= guard(x > 5);
    Some(x * 2)
};
// with_guard = Some(20)

// ネストされた計算
let nested: Option<i32> = eff! {
    list <= Some(vec![1, 2, 3]);
    first <= list.first().copied();
    let doubled = first * 2;
    Some(doubled)
};
// nested = Some(2)

// Result の eff! マクロ
let result_computation: Result<i32, String> = eff! {
    x <= Ok::<i32, String>(10);
    y <= Ok::<i32, String>(20);
    Ok(x + y)
};
// result_computation = Ok(30)

// タプル分解付き
let with_pattern: Option<(i32, i32)> = eff! {
    pair <= Some((10, 20));
    let (a, b) = pair;
    Some((a + b, a * b))
};
// with_pattern = Some((30, 200))
```

### 複雑な例

```haskell
-- Haskell - データベース風の操作
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

fn find_user(id: i32) -> Option<User> { None } // プレースホルダー
fn find_orders(user_id: i32) -> Option<Vec<Order>> { None } // プレースホルダー

fn get_user_orders(uid: i32) -> Option<(User, Vec<Order>)> {
    eff! {
        user <= find_user(uid);
        orders <= find_orders(user.user_id);
        Some((user, orders))
    }
}
```

### for\_! マクロ (リスト内包表記)

Haskell のリスト内包表記には、`yield` を使った `for_!` マクロを使用します:

#### 構文比較

| Haskell                       | lambars                                           | 説明                  |
| ----------------------------- | ------------------------------------------------- | --------------------- |
| `[f x \| x <- xs]`            | `for_! { x <= xs; yield f(x) }`                   | 基本的な内包表記      |
| `[x + y \| x <- xs, y <- ys]` | `for_! { x <= xs; y <= ys.clone(); yield x + y }` | ネスト                |
| `[x \| x <- xs, p x]`         | `xs.into_iter().filter(p).collect()`              | フィルタ (std を使用) |
| `let y = expr`                | `let y = expr;`                                   | 純粋なバインド        |

**重要**: `for_!` では、Rust の所有権ルールにより内部コレクションは `.clone()` が必要です。

#### コード例

```haskell
-- Haskell - リスト内包表記
doubled :: [Int]
doubled = [x * 2 | x <- [1, 2, 3, 4, 5]]
-- doubled = [2, 4, 6, 8, 10]

-- ネストされた内包表記 (直積)
cartesian :: [Int]
cartesian = [x + y | x <- [1, 2], y <- [10, 20]]
-- cartesian = [11, 21, 12, 22]

-- フィルタ付き
filtered :: [Int]
filtered = [x | x <- [1, 2, 3, 4, 5], even x]
-- filtered = [2, 4]

-- 複数のジェネレータを持つ複雑な例
triples :: [(Int, Int, Int)]
triples = [(a, b, c) | c <- [1..10], b <- [1..c], a <- [1..b], a^2 + b^2 == c^2]
-- triples = [(3, 4, 5), (6, 8, 10)]
```

```rust
// lambars - for_! マクロ
use lambars::for_;

let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// ネストされた内包表記 (直積)
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // 注: 内部イテレーションには clone() が必要
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// フィルタ付き (std のイテレータメソッドを使用)
let filtered: Vec<i32> = (1..=5).filter(|x| x % 2 == 0).collect();
// filtered = vec![2, 4]

// ピタゴラス三組の例
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

### 各マクロを使用すべき時

| シナリオ                | 推奨マクロ   | 理由                                   |
| ----------------------- | ------------ | -------------------------------------- |
| Maybe/Either のチェーン | `eff!`       | Nothing/Left で短絡                    |
| IO/State/Reader/Writer  | `eff!`       | FnOnce Monad 向けに設計                |
| リスト生成              | `for_!`      | yield での複数イテレーションをサポート |
| 直積                    | `for_!`      | ネストされたイテレーション             |
| データベース風のクエリ  | `eff!`       | Monad でのエラー処理                   |
| 非同期リスト生成        | `for_async!` | yield での非同期イテレーション         |
| ループ内の非同期操作    | `for_async!` | AsyncIO バインドに `<~` を使用         |

---

## 関数合成

| Haskell                   | lambars          | 説明                         |
| ------------------------- | ---------------- | ---------------------------- |
| `f . g`                   | `compose!(f, g)` | 右から左への合成             |
| `f >>> g` (Control.Arrow) | `compose!(g, f)` | 左から右への合成             |
| `f <<< g` (Control.Arrow) | `compose!(f, g)` | `.` と同じ                   |
| `x & f`                   | `pipe!(x, f)`    | パイプ演算子                 |
| `fmap f m`                | `pipe!(m, => f)` | モナド内で純粋関数をリフト   |
| `m >>= f`                 | `pipe!(m, =>> f)`| モナド関数をバインド         |
| `f $ x`                   | `f(x)`           | 関数適用                     |
| `flip f`                  | `flip(f)`        | 引数の反転                   |
| `const x`                 | `constant(x)`    | 定数関数                     |
| `id`                      | `identity`       | 恒等関数                     |
| `on`                      | 手動実装         | 射影上の二項関数             |

### コード例

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

-- 右から左への合成 (.)
composed1 :: Int -> Int
composed1 = addOne . double . square
-- composed1 5 = addOne (double (square 5)) = addOne (double 25) = addOne 50 = 51

-- 左から右への合成 (>>>)
composed2 :: Int -> Int
composed2 = square >>> double >>> addOne
-- composed2 5 = 51

-- パイプ (&) の使用
result :: Int
result = 5 & square & double & addOne
-- result = 51

-- flip
subtract' :: Int -> Int -> Int
subtract' = (-)

flippedSubtract :: Int -> Int -> Int
flippedSubtract = flip subtract'
-- flippedSubtract 3 10 = 7 (つまり 10 - 3)

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

// 右から左への合成 (Haskell の . のように)
let composed1 = compose!(add_one, double, square);
let result1 = composed1(5);
// result1 = 51

// 左から右のデータフローに pipe! を使用
let result2 = pipe!(5, square, double, add_one);
// result2 = 51

// flip
fn subtract(a: i32, b: i32) -> i32 { a - b }
let flipped_subtract = flip(subtract);
let result = flipped_subtract(3, 10);
// result = 7 (つまり 10 - 3)

// constant
let always_five = constant(5);
let result = always_five("anything");
// result = 5

// identity
let x = identity(42);
// x = 42
```

---

## カリー化と部分適用

| Haskell                   | lambars                                               | 説明             |
| ------------------------- | ----------------------------------------------------- | ---------------- |
| 自動カリー化              | `curry!(fn, arity)` または `curry!(\|args...\| body)` | 明示的なカリー化 |
| 部分適用                  | `partial!`                                            | 部分適用         |
| セクション `(+1)`, `(1+)` | クロージャ                                            | 演算子セクション |
| `uncurry f`               | `\|(a, b)\| f(a, b)`                                  | アンカリー       |

### コード例

```haskell
-- Haskell - 全ての関数はデフォルトでカリー化されている
add :: Int -> Int -> Int
add x y = x + y

addFive :: Int -> Int
addFive = add 5

result :: Int
result = addFive 3
-- result = 8

-- 複数引数の部分適用
add3 :: Int -> Int -> Int -> Int
add3 x y z = x + y + z

addFiveAndTen :: Int -> Int
addFiveAndTen = add3 5 10

result3 :: Int
result3 = addFiveAndTen 3
-- result3 = 18

-- セクション
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

// 関数名 + アリティ形式でのカリー化
let curried_add = curry!(add, 2);
let add_five = curried_add(5);
let result = add_five(3);
// result = 8

// クロージャ形式でのカリー化
let curried_add = curry!(|a, b| add(a, b));
let add_five = curried_add(5);
let result = add_five(3);
// result = 8

// 複数引数のカリー化
fn add3(a: i32, b: i32, c: i32) -> i32 { a + b + c }

let curried_add3 = curry!(add3, 3);
let add_five = curried_add3(5);
let add_five_and_ten = add_five(10);
let result3 = add_five_and_ten(3);
// result3 = 18

// プレースホルダを使った部分適用
let add_five_partial = partial!(add, 5, __);
let result = add_five_partial(3);
// result = 8

// セクション相当 (クロージャ)
let add_one = |x: i32| x + 1;
let halve = |x: f64| x / 2.0;

// uncurry 相当
let add_tuple = |(a, b): (i32, i32)| add(a, b);
let result = add_tuple((3, 5));
// result = 8
```

---

## 遅延評価

| Haskell           | lambars                 | 説明             |
| ----------------- | ----------------------- | ---------------- |
| デフォルトで遅延  | `Lazy::new`             | 明示的な遅延     |
| `seq a b`         | `Lazy::force`           | 評価の強制       |
| バンパターン `!x` | Rust は正格             | 正格パターン     |
| `$!` (正格適用)   | 通常の適用              | 正格適用         |
| 無限リスト        | `Iterator`              | 遅延シーケンス   |
| `take n xs`       | `Iterator::take`        | n 要素を取得     |
| `drop n xs`       | `Iterator::skip`        | n 要素をスキップ |
| `iterate f x`     | `std::iter::successors` | 関数の反復       |
| `repeat x`        | `std::iter::repeat`     | 無限の繰り返し   |
| `cycle xs`        | `Iterator::cycle`       | リストを循環     |

### コード例

```haskell
-- Haskell - デフォルトで遅延
expensive :: Int
expensive = trace "Computing..." (42 * 1000000)

-- 必要になるまで評価されない
main :: IO ()
main = do
  let x = expensive  -- まだ計算されない
  putStrLn "Before"
  print x            -- ここで計算される
  print x            -- キャッシュされ、再計算されない

-- 無限リスト
naturals :: [Int]
naturals = [0..]

firstTen :: [Int]
firstTen = take 10 naturals
-- firstTen = [0,1,2,3,4,5,6,7,8,9]

-- iterate を使った無限リスト
powersOfTwo :: [Int]
powersOfTwo = iterate (*2) 1

firstPowers :: [Int]
firstPowers = take 10 powersOfTwo
-- firstPowers = [1,2,4,8,16,32,64,128,256,512]

-- seq で評価を強制
strictSum :: Int -> Int -> Int
strictSum x y = x `seq` y `seq` (x + y)
```

```rust
// lambars - 明示的な遅延評価
use lambars::control::Lazy;

let expensive = Lazy::new(|| {
    println!("Computing...");
    42 * 1000000
});

// 必要になるまで評価されない
println!("Before");
let x = expensive.force();  // ここで計算され、"Computing..." が表示される
let y = expensive.force();  // キャッシュされ、再計算されない

// 無限イテレータ (Haskell の遅延リストに類似)
let naturals = 0..;
let first_ten: Vec<i32> = naturals.take(10).collect();
// first_ten = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

// iterate 相当
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

| Haskell (lens)              | lambars              | 説明                   |
| --------------------------- | -------------------- | ---------------------- |
| `makeLenses ''Type`         | `lens!(Type, field)` | レンズの生成           |
| `view l s` / `s ^. l`       | `Lens::get`          | フォーカスした値を取得 |
| `set l a s` / `s & l .~ a`  | `Lens::set`          | 値を設定               |
| `over l f s` / `s & l %~ f` | `Lens::modify`       | 値を変更               |
| `l1 . l2`                   | `Lens::compose`      | レンズの合成           |
| `_1`, `_2`                  | `lens!((A,B), 0)`    | タプルレンズ           |

### Prism

| Haskell (lens)              | lambars                 | 説明             |
| --------------------------- | ----------------------- | ---------------- |
| `makePrisms ''Type`         | `prism!(Type, Variant)` | Prism の生成     |
| `preview p s` / `s ^? p`    | `Prism::preview`        | マッチすれば取得 |
| `review p a` / `p # a`      | `Prism::review`         | 値から構築       |
| `over p f s` / `s & p %~ f` | `Prism::modify`         | マッチすれば変更 |
| `_Just`, `_Nothing`         | `prism!(Option, Some)`  | Maybe prism      |
| `_Left`, `_Right`           | `prism!(Result, Ok)`    | Either prism     |

### Iso

| Haskell (lens) | lambars            | 説明       |
| -------------- | ------------------ | ---------- |
| `iso f g`      | `FunctionIso::new` | 同型の作成 |
| `view i s`     | `Iso::get`         | 前方変換   |
| `review i a`   | `Iso::reverse_get` | 後方変換   |
| `from i`       | `Iso::reverse`     | 方向の反転 |

### Traversal

| Haskell (lens) | lambars              | 説明                   |
| -------------- | -------------------- | ---------------------- |
| `traversed`    | `VecTraversal::new`  | リスト走査             |
| `toListOf t s` | `Traversal::get_all` | 全てのターゲットを取得 |
| `over t f s`   | `Traversal::modify`  | 全てのターゲットを変更 |
| `each`         | `VecTraversal`       | 各要素                 |

### コード例

```haskell
-- Haskell (lens ライブラリ)
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

-- 取得
streetName :: Person -> String
streetName p = p ^. address . street

-- 設定
setStreet :: Person -> String -> Person
setStreet p s = p & address . street .~ s

-- 変更
upperStreet :: Person -> Person
upperStreet p = p & address . street %~ map toUpper

-- 例
person :: Person
person = Person "Alice" (Address "Main St" "Tokyo")

result :: String
result = person ^. address . street
-- result = "Main St"

updated :: Person
updated = person & address . street .~ "Oak Ave"
-- updated.address.street = "Oak Ave"

-- Prism の例
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

// レンズの合成
let person_street = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        street: "Main St".to_string(),
        city: "Tokyo".to_string(),
    },
};

// 取得
let street_name: &String = person_street.get(&person);
// street_name = "Main St"

// 設定
let updated = person_street.set(person.clone(), "Oak Ave".to_string());
// updated.address.street = "Oak Ave"

// 変更
let uppercased = person_street.modify(person, |s| s.to_uppercase());
// uppercased.address.street = "MAIN ST"

// Prism の例
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

// Review (構築)
let constructed: Shape = circle_prism.review(10.0);
// constructed = Shape::Circle(10.0)
```

---

## エフェクトモナド

### IO Monad

| Haskell               | lambars          | 説明                    |
| --------------------- | ---------------- | ----------------------- |
| `pure a` / `return a` | `IO::pure`       | IO 内の純粋な値         |
| `IO action`           | `IO::new`        | IO アクションの作成     |
| `io >>= f`            | `IO::flat_map`   | IO アクションのバインド |
| `io >> io2`           | `IO::then`       | 順序付け                |
| `putStrLn s`          | `IO::print_line` | 行を出力                |
| `getLine`             | `IO::read_line`  | 行を読み込み            |
| `threadDelay n`       | `IO::delay`      | 実行を遅延              |
| `catch io handler`    | `IO::catch`      | 例外を処理              |

#### コード例

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

-- エラー処理
safeComputation :: IO Int
safeComputation = catch
  (error "oops" :: IO Int)
  (\(e :: SomeException) -> pure 0)

-- main の例
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

// エラー処理
let safe_computation: IO<i32> = IO::catch(
    IO::new(|| panic!("oops")),
    |_| 0
);

// run_unsafe で実行
let result = computation.run_unsafe();
// "Computing..." が表示され、result = 42

// IO アクション
let print_io = IO::print_line("Hello, World!");
print_io.run_unsafe();  // "Hello, World!" が表示される
```

### State Monad

| Haskell               | lambars                  | 説明             |
| --------------------- | ------------------------ | ---------------- |
| `pure a` / `return a` | `State::pure`            | 純粋な値         |
| `get`                 | `State::get`             | 状態を取得       |
| `put s`               | `State::put`             | 状態を設定       |
| `modify f`            | `State::modify`          | 状態を変更       |
| `gets f`              | `State::gets`            | 派生値を取得     |
| `runState m s`        | `State::run`             | 初期状態で実行   |
| `evalState m s`       | `State::eval`            | 結果のみ取得     |
| `execState m s`       | `State::exec`            | 最終状態のみ取得 |
| `state f`             | `State::from_transition` | 関数から作成     |

#### コード例

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
-- result = (2, 2)  -- (戻り値、最終状態)
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

| Haskell               | lambars         | 説明           |
| --------------------- | --------------- | -------------- |
| `pure a` / `return a` | `Reader::pure`  | 純粋な値       |
| `ask`                 | `Reader::ask`   | 環境を取得     |
| `asks f`              | `Reader::asks`  | 派生値を取得   |
| `local f m`           | `Reader::local` | 環境を変更     |
| `runReader m r`       | `Reader::run`   | 環境と共に実行 |

#### コード例

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

| Haskell               | lambars          | 説明                   |
| --------------------- | ---------------- | ---------------------- |
| `pure a` / `return a` | `Writer::pure`   | 純粋な値               |
| `tell w`              | `Writer::tell`   | 出力をログ             |
| `listen m`            | `Writer::listen` | 計算内でログにアクセス |
| `pass m`              | `Writer::pass`   | ログを変換             |
| `censor f m`          | `Writer::censor` | ログを検閲             |
| `runWriter m`         | `Writer::run`    | (結果、ログ) を取得    |
| `execWriter m`        | `Writer::exec`   | ログのみ取得           |

#### コード例

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

| Haskell         | lambars           | 説明                                 |
| --------------- | ----------------- | ------------------------------------ |
| `RWS r w s a`   | `RWS<R, W, S, A>` | Reader + Writer + State の組み合わせ |
| `rws f`         | `RWS::new`        | 関数から作成                         |
| `runRWS m r s`  | `RWS::run`        | 環境と状態で実行                     |
| `evalRWS m r s` | `RWS::eval`       | (結果、出力) のみ取得                |
| `execRWS m r s` | `RWS::exec`       | (状態、出力) のみ取得                |
| `mapRWS f m`    | `RWS::map_rws`    | (結果、状態、出力) を変換            |
| `withRWS f m`   | `RWS::with_rws`   | (環境、状態) 入力を変換              |
| `ask`           | `RWS::ask`        | 環境を取得                           |
| `asks f`        | `RWS::asks`       | 環境から射影                         |
| `local f m`     | `RWS::local`      | 環境をローカルに変更                 |
| `tell w`        | `RWS::tell`       | 出力を追加                           |
| `listen m`      | `RWS::listen`     | 出力をキャプチャ                     |
| `censor f m`    | `RWS::censor`     | 出力を変換                           |
| `get`           | `RWS::get`        | 状態を取得                           |
| `put s`         | `RWS::put`        | 状態を設定                           |
| `modify f`      | `RWS::modify`     | 状態を変更                           |
| `gets f`        | `RWS::gets`       | 状態から射影                         |

#### コード例

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

## モナド変換子 (mtl)

### 比較

| Haskell (mtl)       | lambars                            | 説明                        |
| ------------------- | ---------------------------------- | --------------------------- |
| `StateT s m a`      | `StateT<S, M, A>`                  | State 変換子                |
| `ReaderT r m a`     | `ReaderT<R, M, A>`                 | Reader 変換子               |
| `WriterT w m a`     | `WriterT<W, M, A>`                 | Writer 変換子               |
| `ExceptT e m a`     | `ExceptT<E, M, A>`                 | 例外変換子                  |
| `MaybeT m a`        | カスタム                           | Maybe 変換子                |
| `lift`              | `lift_*` メソッド                  | 変換子にリフト              |
| `liftIO`            | `lift_io`, `lift_async_io`         | IO/AsyncIO をリフト         |
| `MonadState`        | `MonadState` trait                 | State の抽象化              |
| `MonadReader`       | `MonadReader` trait                | Reader の抽象化             |
| `MonadWriter`       | `MonadWriter` trait                | Writer の抽象化             |
| `MonadError`        | `MonadError` trait                 | Error の抽象化              |
| `throwError`        | `MonadError::throw_error`          | エラーをスロー              |
| `catchError`        | `MonadError::catch_error`          | エラーをキャッチして処理    |
| `liftEither`        | `MonadError::from_result`          | Either/Result をリフト      |
| `handleError`       | `MonadError::handle_error`         | エラーを成功値に変換        |
| (カスタム)          | `MonadError::adapt_error`          | 同じ型内でエラーを変換      |
| (カスタム)          | `MonadError::recover`              | 部分関数による復旧          |
| (カスタム)          | `MonadError::recover_with_partial` | Monad 部分復旧              |
| (カスタム)          | `MonadError::ensure`               | 述語で検証                  |
| (カスタム)          | `MonadError::ensure_or`            | 値依存エラーで検証          |
| (カスタム)          | `MonadError::redeem`               | 成功とエラーの両方を変換    |
| (カスタム)          | `MonadError::redeem_with`          | Monad redeem                |
| (カスタム)          | `MonadErrorExt::map_error`         | エラー型を変換              |
| 変換子内の非同期 IO | `*_async_io` メソッド              | 変換子での AsyncIO サポート |

### コード例

```haskell
-- Haskell (mtl)
import Control.Monad.State
import Control.Monad.Reader
import Control.Monad.Except

data AppConfig = AppConfig { configMaxRetries :: Int }
type AppState = Int  -- リトライ回数
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

// Rust の型システムにより、具体的な変換子スタックで動作します
// これは Result 上の ReaderT の簡略化された例です

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

### 変換子での AsyncIO サポート

lambars は、モナド変換子の AsyncIO 統合を提供し、変換子スタック内での非同期操作を可能にします。

```rust
// lambars - AsyncIO を使った ReaderT
use lambars::effect::{ReaderT, AsyncIO};

#[derive(Clone)]
struct Config { api_url: String }

// AsyncIO 上の ReaderT
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

変換子で利用可能な AsyncIO メソッド:

- `ReaderT`: `ask_async_io`, `asks_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `StateT`: `get_async_io`, `gets_async_io`, `state_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `WriterT`: `tell_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`, `listen_async_io`

---

## 代数的エフェクト

lambars は、モナド変換子の代替として代数的エフェクトシステムを提供します。このアプローチは、Polysemy (Haskell)、Eff (Scala/OCaml)、freer-simple などのライブラリに触発されており、モナド変換子の n^2 問題を解決します。

### Haskell エフェクトライブラリとの比較

| Haskell (freer-simple/polysemy) | lambars                                    | 説明                         |
| ------------------------------- | ------------------------------------------ | ---------------------------- |
| `Eff '[e1, e2] a`               | `Eff<EffCons<E1, EffCons<E2, EffNil>>, A>` | エフェクト計算型             |
| `Member e r`                    | `Member<E, Index>`                         | エフェクトメンバーシップ制約 |
| `run`                           | `Handler::run`                             | ハンドラを実行               |
| `runReader`                     | `ReaderHandler::run`                       | Reader エフェクトを実行      |
| `runState`                      | `StateHandler::run`                        | State エフェクトを実行       |
| `runWriter`                     | `WriterHandler::run`                       | Writer エフェクトを実行      |
| `runError`                      | `ErrorHandler::run`                        | Error エフェクトを実行       |
| `send` / `embed`                | `perform_raw`                              | エフェクト操作を実行         |
| `interpret`                     | `Handler` trait impl                       | ハンドラを定義               |
| `reinterpret`                   | ハンドラの合成                             | エフェクトを変換             |

### エフェクト Row と Member

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

### 標準エフェクト

| エフェクト | Haskell                    | lambars                           | 説明             |
| ---------- | -------------------------- | --------------------------------- | ---------------- |
| Reader     | `ask`, `asks`, `local`     | `ask()`, `asks()`, `run_local()`  | 読み取り専用環境 |
| State      | `get`, `put`, `modify`     | `get()`, `put()`, `modify()`      | 可変状態         |
| Writer     | `tell`, `listen`, `censor` | `tell()`, `listen()`              | 出力の累積       |
| Error      | `throwError`, `catchError` | `throw()`, `catch()`, `attempt()` | エラー処理       |

### カスタムエフェクトの定義

```haskell
-- Haskell (freer-simple with TH)
data Log r where
  LogMsg :: String -> Log ()

makeEffect ''Log

-- または手動で
logMsg :: Member Log r => String -> Eff r ()
logMsg msg = send (LogMsg msg)

-- ハンドラ
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
    /// ロギングエフェクト
    effect Log {
        /// メッセージをログに記録
        fn log_message(message: String) -> ();
    }
}

// マクロが生成するもの:
// - Effect を実装した LogEffect 構造体
// - LogEffect::log_message(message) -> Eff<LogEffect, ()>
// - fn log_message(&mut self, message: String) -> () を持つ LogHandler trait

// エフェクトを使った計算を作成
fn log_computation() -> Eff<LogEffect, i32> {
    LogEffect::log_message("Starting".to_string())
        .then(LogEffect::log_message("Processing".to_string()))
        .then(Eff::pure(42))
}
```

### モナド変換子との主な違い

| 側面             | モナド変換子                                 | 代数的エフェクト              |
| ---------------- | -------------------------------------------- | ----------------------------- |
| n^2 問題         | あり (n エフェクトに n^2 の lift 実装が必要) | なし (エフェクトは自由に合成) |
| エフェクトの順序 | 変換子スタックで固定                         | 柔軟 (任意の順序で処理)       |
| パフォーマンス   | 良好 (特殊化されたコード)                    | 良好 (継続ベース)             |
| 型の複雑さ       | 冗長になり得る                               | 型レベルインデックスを使用    |
| Lift 操作        | 必要 (`lift`, `liftIO`)                      | 不要 (`Member` 制約)          |

### どちらを使うべきか

| シナリオ                          | 推奨事項                        |
| --------------------------------- | ------------------------------- |
| シンプルな 2-3 エフェクトスタック | モナド変換子 (より単純な型)     |
| 多数のエフェクト (4+)             | 代数的エフェクト (n^2 問題なし) |
| エフェクトの並び替えが必要        | 代数的エフェクト                |
| 最高のパフォーマンス              | モナド変換子                    |
| 拡張可能なエフェクト              | 代数的エフェクト                |
| 既存の mtl コードベース           | モナド変換子 (互換性)           |

---

## データ構造

### リストとシーケンス

| Haskell              | lambars                          | 説明                               |
| -------------------- | -------------------------------- | ---------------------------------- |
| `[a]` (List)         | `PersistentList<A>`              | 不変リスト                         |
| `x : xs`             | `PersistentList::cons`           | 先頭に追加                         |
| `head xs`            | `PersistentList::head`           | 最初の要素                         |
| `tail xs`            | `PersistentList::tail`           | リストの残り                       |
| `xs ++ ys`           | `Semigroup::combine`             | 連結                               |
| `length xs`          | `Foldable::length`               | 長さ                               |
| `null xs`            | `Foldable::is_empty`             | 空チェック                         |
| `reverse xs`         | `PersistentList::reverse`        | 逆順                               |
| `take n xs`          | `PersistentList::take`           | 最初の n 要素を取得                |
| `drop n xs`          | `PersistentList::drop_first`     | 最初の n 要素を削除                |
| `splitAt n xs`       | `PersistentList::split_at`       | インデックスで分割                 |
| `zip xs ys`          | `PersistentList::zip`            | 二つのリストを zip                 |
| `unzip xs`           | `PersistentList::<(A,B)>::unzip` | ペアのリストを unzip               |
| `findIndex p xs`     | `PersistentList::find_index`     | 最初にマッチしたインデックスを検索 |
| `foldl1 f xs`        | `PersistentList::fold_left1`     | 初期値なしの左畳み込み             |
| `foldr1 f xs`        | `PersistentList::fold_right1`    | 初期値なしの右畳み込み             |
| `scanl f z xs`       | `PersistentList::scan_left`      | 初期値ありの左スキャン             |
| `partition p xs`     | `PersistentList::partition`      | 述語で分割                         |
| `intersperse x xs`   | `PersistentList::intersperse`    | 要素間に挿入                       |
| `intercalate xs xss` | `PersistentList::intercalate`    | リスト間にリストを挿入して平坦化   |
| `compare xs ys`      | `Ord::cmp`                       | 辞書順 (T: Ord が必要)             |

### ベクトル

| Haskell           | lambars                            | 説明                                 |
| ----------------- | ---------------------------------- | ------------------------------------ |
| `Data.Vector`     | `PersistentVector<A>`              | 不変ベクトル                         |
| `V.!`             | `PersistentVector::get`            | インデックスアクセス                 |
| `V.//`            | `PersistentVector::update`         | 要素の更新                           |
| `V.snoc`          | `PersistentVector::push_back`      | 追加                                 |
| `V.length`        | `PersistentVector::len`            | 長さ                                 |
| `V.take n v`      | `PersistentVector::take`           | 最初の n 要素を取得                  |
| `V.drop n v`      | `PersistentVector::drop_first`     | 最初の n 要素を削除                  |
| `V.splitAt n v`   | `PersistentVector::split_at`       | インデックスで分割                   |
| `V.zip v1 v2`     | `PersistentVector::zip`            | 二つのベクトルを zip                 |
| `V.unzip v`       | `PersistentVector::<(A,B)>::unzip` | ペアのベクトルを unzip               |
| `V.findIndex p v` | `PersistentVector::find_index`     | 最初にマッチしたインデックスを検索   |
| `V.foldl1 f v`    | `PersistentVector::fold_left1`     | 初期値なしの左畳み込み               |
| `V.foldr1 f v`    | `PersistentVector::fold_right1`    | 初期値なしの右畳み込み               |
| `V.scanl f z v`   | `PersistentVector::scan_left`      | 初期値ありの左スキャン               |
| `V.partition p v` | `PersistentVector::partition`      | 述語で分割                           |
| (N/A)             | `PersistentVector::intersperse`    | 要素間に挿入                         |
| (N/A)             | `PersistentVector::intercalate`    | ベクトル間にベクトルを挿入して平坦化 |
| `compare v1 v2`   | `Ord::cmp`                         | 辞書順 (T: Ord が必要)               |

### マップ

| Haskell               | lambars                   | 説明                     |
| --------------------- | ------------------------- | ------------------------ |
| `Data.Map`            | `PersistentTreeMap<K, V>` | 順序付きマップ           |
| `Data.HashMap`        | `PersistentHashMap<K, V>` | ハッシュマップ           |
| `M.insert k v m`      | `insert` メソッド         | 挿入                     |
| `M.lookup k m`        | `get` メソッド            | 検索                     |
| `M.delete k m`        | `remove` メソッド         | 削除                     |
| `M.member k m`        | `contains_key` メソッド   | メンバーシップ           |
| `M.map f m`           | `map_values` メソッド     | 値を変換                 |
| `M.mapKeys f m`       | `map_keys` メソッド       | キーを変換               |
| `M.mapMaybe f m`      | `filter_map` メソッド     | フィルタと変換           |
| `M.toList m`          | `entries` メソッド        | 全エントリを取得         |
| `M.keys m`            | `keys` メソッド           | 全キーを取得             |
| `M.elems m`           | `values` メソッド         | 全値を取得               |
| `M.union m1 m2`       | `merge` メソッド          | マージ (右が優先)        |
| `M.unionWith f m1 m2` | `merge_with` メソッド     | リゾルバでマージ         |
| `M.filter p m`        | `keep_if` メソッド        | マッチするエントリを保持 |
| `M.filterWithKey p m` | `keep_if` メソッド        | マッチするエントリを保持 |
| `M.partition p m`     | `partition` メソッド      | 述語で分割               |

### セット

| Haskell          | lambars                 | 説明           |
| ---------------- | ----------------------- | -------------- |
| `Data.Set`       | `PersistentHashSet<A>`  | セット         |
| `S.insert x s`   | `insert` メソッド       | 挿入           |
| `S.member x s`   | `contains` メソッド     | メンバーシップ |
| `S.union s1 s2`  | `union` メソッド        | 和集合         |
| `S.intersection` | `intersection` メソッド | 積集合         |
| `S.difference`   | `difference` メソッド   | 差集合         |

### コード例

```haskell
-- Haskell
import qualified Data.Map as M
import qualified Data.Set as S

-- リスト操作
list :: [Int]
list = 1 : 2 : 3 : []

headElem :: Int
headElem = head list
-- headElem = 1

tailList :: [Int]
tailList = tail list
-- tailList = [2, 3]

-- マップ操作
map1 :: M.Map String Int
map1 = M.fromList [("one", 1), ("two", 2)]

updated :: M.Map String Int
updated = M.insert "three" 3 map1

value :: Maybe Int
value = M.lookup "one" map1
-- value = Just 1

-- セット操作
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

// リスト操作
let list = PersistentList::new().cons(3).cons(2).cons(1);

let head_elem: Option<&i32> = list.head();
// head_elem = Some(&1)

let tail_list: Option<PersistentList<i32>> = list.tail();
// tail_list = Some(PersistentList [2, 3])

// HashMap 操作
let map1 = PersistentHashMap::new()
    .insert("one".to_string(), 1)
    .insert("two".to_string(), 2);

let updated = map1.insert("three".to_string(), 3);

let value: Option<&i32> = map1.get("one");
// value = Some(&1)

// HashSet 操作
let set1: PersistentHashSet<i32> = [1, 2, 3].into_iter().collect();
let set2: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();

let union_set = set1.union(&set2);
// union_set は {1, 2, 3, 4} を含む

let intersection_set = set1.intersection(&set2);
// intersection_set は {2, 3} を含む

// HashSetView - 遅延評価 (Haskell の遅延セマンティクスに類似)
let result: PersistentHashSet<i32> = set1
    .view()
    .filter(|x| *x % 2 == 1)
    .map(|x| x * 10)
    .collect();
// result は {10, 30} を含む
```

---

## パターンマッチング

| Haskell              | Rust                 | 説明           |
| -------------------- | -------------------- | -------------- | ------ |
| `case x of ...`      | `match x { ... }`    | マッチ式       |
| `_ -> ...`           | `_ => ...`           | ワイルドカード |
| `x@pattern`          | `x @ pattern`        | As パターン    |
| `(a, b)`             | `(a, b)`             | タプルパターン |
| `Just x`             | `Some(x)`            | Maybe/Option   |
| `Left e` / `Right a` | `Err(e)` / `Ok(a)`   | Either/Result  |
| `[]`                 | `[]` または `vec![]` | 空リスト       |
| `x:xs`               | カスタム             | Cons パターン  |
| ガード `             | cond`                | `if cond =>`   | ガード |

### コード例

```haskell
-- Haskell
describeNumber :: Int -> String
describeNumber n = case n of
  0 -> "zero"
  1 -> "one"
  x | x < 0 -> "negative"
    | x > 100 -> "large"
    | otherwise -> "other"

-- Maybe のパターンマッチング
describeMaybe :: Maybe Int -> String
describeMaybe m = case m of
  Nothing -> "nothing"
  Just 0 -> "zero"
  Just n -> "some: " ++ show n

-- リストのパターンマッチング
describeList :: [a] -> String
describeList xs = case xs of
  [] -> "empty"
  [_] -> "singleton"
  [_, _] -> "pair"
  _ -> "many"

-- As パターン
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

// Option のパターンマッチング
fn describe_option(m: Option<i32>) -> String {
    match m {
        None => "nothing".to_string(),
        Some(0) => "zero".to_string(),
        Some(n) => format!("some: {}", n),
    }
}

// スライスのパターンマッチング
fn describe_slice<T>(xs: &[T]) -> String {
    match xs {
        [] => "empty".to_string(),
        [_] => "singleton".to_string(),
        [_, _] => "pair".to_string(),
        _ => "many".to_string(),
    }
}

// As パターン
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

## 高カインド型

Haskell はネイティブに高カインド型 (HKT) をサポートしていますが、Rust にはありません。lambars は Generic Associated Types (GAT) を使用して HKT をエミュレートします。

### 比較

```haskell
-- Haskell - ネイティブ HKT
class Functor f where
  fmap :: (a -> b) -> f a -> f b

-- 'f' はカインド * -> * の型コンストラクタ
-- これにより Option、List、Either e などを抽象化できる
```

```rust
// lambars - GAT による HKT エミュレーション
pub trait TypeConstructor {
    type Inner;
    type WithType<B>: TypeConstructor<Inner = B>;
}

pub trait Functor: TypeConstructor {
    fn fmap<B, F>(self, f: F) -> Self::WithType<B>
    where
        F: FnOnce(Self::Inner) -> B;
}

// TypeConstructor は型パラメータの変更を可能にします
// Haskell の HKT ほどエレガントではありませんが、同様の抽象化を可能にします
```

### 制限事項

1. **直接的なカインド多相性なし**: Rust は型パラメータとして `* -> *` を表現できない
2. **より冗長**: ネストされた型コンストラクタでは trait の境界が複雑になる
3. **限定的な推論**: 型注釈が必要になることが多い
4. **特殊な実装**: `traverse` のような操作には型固有のバリアントが必要

---

## 代数的データ型

Haskell と Rust は共に代数的データ型をサポートしていますが、構文は異なります。

### 直和型 (Enum)

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

### 直積型 (構造体/レコード)

```haskell
-- Haskell
data Person = Person
  { name :: String
  , age :: Int
  , email :: String
  }

-- レコード構文は自動的にゲッターを提供
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

// フィールドアクセスは直接
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

## まとめ: 主な違い

### 構文マッピング

| Haskell              | Rust (lambars)                      |
| -------------------- | ----------------------------------- |
| `f x`                | `f(x)`                              |
| `f $ x`              | `f(x)`                              |
| `x & f`              | `pipe!(x, f)`                       |
| `f . g`              | `compose!(f, g)`                    |
| `do { x <- m; ... }` | `eff! { x <= m; ... }`              |
| `\x -> x + 1`        | `\|x\| x + 1`                       |
| `x :: Int`           | `x: i32`                            |
| `[a]`                | `Vec<A>` または `PersistentList<A>` |
| `Maybe a`            | `Option<A>`                         |
| `Either e a`         | `Result<A, E>`                      |
| `IO a`               | `IO<A>`                             |
| `pure x`             | `Applicative::pure(x)`              |
| `m >>= f`            | `m.flat_map(f)`                     |
| `fmap f m`           | `m.fmap(f)`                         |

### 概念的な違い

1. **遅延性**: Haskell はデフォルトで遅延、Rust は正格 (明示的な遅延には `Lazy` を使用)

2. **純粋性**: Haskell は IO を通じて純粋性を強制、Rust はどこでも副作用を許可 (規律のために IO モナドを使用)

3. **高カインド型**: Haskell はネイティブ HKT、Rust は GAT によるエミュレーション

4. **型クラス vs Trait**: 似た概念だが、Haskell の方が孤児インスタンスで柔軟性が高い

5. **カリー化**: Haskell の関数はデフォルトでカリー化、Rust は明示的なカリー化が必要

6. **メモリ管理**: Haskell は GC、Rust は所有権/借用

7. **パターンマッチング**: 両方サポートするが、Rust は全ケースの明示的な処理が必要
