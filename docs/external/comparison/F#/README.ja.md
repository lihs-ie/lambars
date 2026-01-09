# F# から lambars への API 対応ガイド

[English](README.en.md)

> **Note**: このドキュメントは AI によって翻訳されました。誤りや不自然な表現がある場合は、Issue または Pull Request でお知らせください。

本ドキュメントは、F# の関数型プログラミング構文と lambars (Rust) の同等の機能を包括的に比較します。

## 目次

- [概要](#概要)
- [Option モジュール](#option-モジュール)
- [Result モジュール](#result-モジュール)
- [リストとシーケンス操作](#リストとシーケンス操作)
- [関数合成](#関数合成)
- [コンピュテーション式と Effect システム](#コンピュテーション式と-effect-システム)
- [アクティブパターンと Optics](#アクティブパターンと-optics)
- [遅延評価](#遅延評価)
- [型クラス / インターフェース](#型クラス--インターフェース)
- [永続データ構造](#永続データ構造)

---

## 概要

| 概念                 | F#                        | lambars (Rust)                 |
| -------------------- | ------------------------- | ------------------------------ | --- | --- | ------- |
| Option 型            | `Option<'T>`              | `Option<T>` (std)              |
| Result 型            | `Result<'T, 'E>`          | `Result<T, E>` (std)           |
| リスト型             | `list<'T>` (immutable)    | `PersistentList<T>`            |
| シーケンス           | `seq<'T>` (lazy)          | `Iterator` / `Lazy<T>`         |
| パイプ演算子         | `\|>`                     | `pipe!` マクロ                 |
| 合成                 | `>>`                      | `compose!` マクロ              |
| コンピュテーション式 | `async { }`, `result { }` | `eff!` マクロ                  |
| リスト内包表記       | `[ for ... ]`, `seq { }`  | `for_!` マクロ                 |
| 非同期リスト内包表記 | `async { for ... }`       | `for_async!` マクロ            |
| アクティブパターン   | `(                        | Pattern                        | \_  | )`  | `Prism` |
| レンズ               | ライブラリ経由            | `Lens`, `lens!` マクロ         |
| Monoid               | `+` 演算子オーバーロード  | `Semigroup`, `Monoid` トレイト |

---

## Option モジュール

### 基本操作

| F#                    | lambars                                      | 説明                       |
| --------------------- | -------------------------------------------- | -------------------------- |
| `Option.map`          | `Functor::fmap`                              | 内部の値を変換             |
| `Option.bind`         | `Monad::flat_map`                            | 計算を連鎖                 |
| `Option.filter`       | `Option::filter` (std)                       | 述語でフィルタ             |
| `Option.defaultValue` | `Option::unwrap_or` (std)                    | デフォルト値を提供         |
| `Option.defaultWith`  | `Option::unwrap_or_else` (std)               | 遅延デフォルト             |
| `Option.orElse`       | `Option::or` (std)                           | 代替 Option                |
| `Option.orElseWith`   | `Option::or_else` (std)                      | 遅延代替                   |
| `Option.isSome`       | `Option::is_some` (std)                      | Some かチェック            |
| `Option.isNone`       | `Option::is_none` (std)                      | None かチェック            |
| `Option.iter`         | `Option::iter` (std)                         | 値をイテレート             |
| `Option.toList`       | `Option::into_iter().collect()`              | リストに変換               |
| `Option.flatten`      | `Flatten::flatten` / `Option::flatten` (std) | ネストした Option を平坦化 |
| `Option.map2`         | `Applicative::map2`                          | 2 つの Option を結合       |
| `Option.map3`         | `Applicative::map3`                          | 3 つの Option を結合       |

### コード例

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

#### F# Option コンピュテーション式 vs lambars eff! マクロ

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

## Result モジュール

### 基本操作

| F#                    | lambars                        | 説明                 |
| --------------------- | ------------------------------ | -------------------- |
| `Result.map`          | `Functor::fmap`                | Ok 値を変換          |
| `Result.mapError`     | `Result::map_err` (std)        | Error 値を変換       |
| `Result.bind`         | `Monad::flat_map`              | 計算を連鎖           |
| `Result.isOk`         | `Result::is_ok` (std)          | Ok かチェック        |
| `Result.isError`      | `Result::is_err` (std)         | Error かチェック     |
| `Result.defaultValue` | `Result::unwrap_or` (std)      | エラー時のデフォルト |
| `Result.defaultWith`  | `Result::unwrap_or_else` (std) | 遅延デフォルト       |
| `Result.toOption`     | `Result::ok` (std)             | Option に変換        |

### エラーハンドリング

| F#                   | lambars                            | 説明                     |
| -------------------- | ---------------------------------- | ------------------------ |
| `try ... with`       | `MonadError::catch_error`          | エラーをキャッチして処理 |
| `raise` / `failwith` | `MonadError::throw_error`          | エラーを投げる           |
| `Result.mapError`    | `MonadErrorExt::map_error`         | エラー型を変換           |
| (パターンマッチ)     | `MonadError::handle_error`         | エラーを成功値に変換     |
| (パターンマッチ)     | `MonadError::adapt_error`          | 同じ型内でエラーを変換   |
| (パターンマッチ)     | `MonadError::recover`              | 部分関数で復旧           |
| (パターンマッチ)     | `MonadError::recover_with_partial` | モナド的部分復旧         |
| (カスタム)           | `MonadError::ensure`               | 述語で検証               |
| (カスタム)           | `MonadError::ensure_or`            | 値依存のエラーで検証     |
| (カスタム)           | `MonadError::redeem`               | 成功とエラーの両方を変換 |
| (カスタム)           | `MonadError::redeem_with`          | モナド的 redeem          |

### コード例

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

#### F# エラーハンドリング vs lambars MonadError

```fsharp
// F# - handle_error 相当 (エラーを成功に変換)
let handleError result =
    match result with
    | Ok x -> Ok x
    | Error e -> Ok (String.length e)

let recovered = Error "error" |> handleError
// recovered = Ok 5

// F# - adapt_error 相当 (コンテキストを追加)
let adaptError result =
    match result with
    | Ok x -> Ok x
    | Error e -> Error (sprintf "Context: %s" e)

let adapted = Error "original" |> adaptError
// adapted = Error "Context: original"

// F# - ensure 相当 (述語で検証)
let ensurePositive result =
    match result with
    | Error e -> Error e
    | Ok x when x > 0 -> Ok x
    | Ok _ -> Error "Value must be positive"

let validated = Ok 5 |> ensurePositive
// validated = Ok 5

// F# - redeem 相当 (両方のケースを変換)
let redeem result =
    match result with
    | Ok x -> Ok (sprintf "Success: %d" x)
    | Error e -> Ok (sprintf "Error: %s" e)

let redeemed = Ok 42 |> redeem
// redeemed = Ok "Success: 42"
```

```rust
// lambars
use lambars::effect::MonadError;

// handle_error - エラーを成功値に変換
let failing: Result<i32, String> = Err("error".to_string());
let recovered = <Result<i32, String>>::handle_error(failing, |e| e.len() as i32);
// recovered = Ok(5)

// adapt_error - エラーにコンテキストを追加
let computation: Result<i32, String> = Err("original".to_string());
let adapted = <Result<i32, String>>::adapt_error(
    computation,
    |e| format!("Context: {}", e)
);
// adapted = Err("Context: original")

// ensure - 述語で検証
let validated = <Result<i32, String>>::ensure(
    Ok(5),
    || "Value must be positive".to_string(),
    |&x| x > 0
);
// validated = Ok(5)

// redeem - 成功とエラーの両方を変換
let redeemed = <Result<i32, String>>::redeem(
    Ok(42),
    |e| format!("Error: {}", e),
    |v| format!("Success: {}", v)
);
// redeemed = Ok("Success: 42")
```

---

## リストとシーケンス操作

### コレクション操作

| F#                  | lambars                                  | 説明                             |
| ------------------- | ---------------------------------------- | -------------------------------- |
| `List.map`          | `Functor::fmap` / `FunctorMut::fmap_mut` | 要素を変換                       |
| `List.collect`      | `Monad::flat_map` + `flatten`            | マップして平坦化                 |
| `List.filter`       | `Iterator::filter` (std)                 | 要素をフィルタ                   |
| `List.fold`         | `Foldable::fold_left`                    | 左畳み込み                       |
| `List.foldBack`     | `Foldable::fold_right`                   | 右畳み込み                       |
| `List.reduce`       | `Iterator::reduce` (std)                 | 初期値なしで削減                 |
| `List.sum`          | `Foldable::fold_left` + `Monoid`         | 要素を合計                       |
| `List.length`       | `Foldable::length`                       | 要素数をカウント                 |
| `List.isEmpty`      | `Foldable::is_empty`                     | 空かチェック                     |
| `List.head`         | `PersistentList::head`                   | 最初の要素                       |
| `List.tail`         | `PersistentList::tail`                   | リストの残り                     |
| `List.cons`         | `PersistentList::cons`                   | 要素を先頭に追加                 |
| `List.append`       | `Semigroup::combine`                     | リストを連結                     |
| `List.rev`          | `PersistentList::reverse`                | リストを逆順に                   |
| `List.exists`       | `Foldable::exists`                       | いずれかの要素がマッチ           |
| `List.forall`       | `Foldable::for_all`                      | すべての要素がマッチ             |
| `List.find`         | `Foldable::find`                         | 最初のマッチを検索               |
| `List.tryFind`      | `Foldable::find`                         | 検索 (Option を返す)             |
| `List.choose`       | `Iterator::filter_map` (std)             | フィルタとマップ                 |
| `List.zip`          | `PersistentList::zip`                    | 2 つのリストを zip               |
| `List.unzip`        | `PersistentList::<(A,B)>::unzip`         | ペアのリストを unzip             |
| `List.take`         | `PersistentList::take`                   | 最初の n 要素を取得              |
| `List.skip`         | `PersistentList::drop_first`             | 最初の n 要素をスキップ          |
| `List.splitAt`      | `PersistentList::split_at`               | インデックスで分割               |
| `List.findIndex`    | `PersistentList::find_index`             | 最初のマッチのインデックスを検索 |
| `List.reduce`       | `PersistentList::fold_left1`             | 初期値なしの左畳み込み           |
| `List.reduceBack`   | `PersistentList::fold_right1`            | 初期値なしの右畳み込み           |
| `List.scan`         | `PersistentList::scan_left`              | 初期値付き左スキャン             |
| `List.partition`    | `PersistentList::partition`              | 述語で分割                       |
| (N/A)               | `PersistentList::intersperse`            | 要素間に挿入                     |
| `String.concat sep` | `PersistentList::intercalate`            | リスト間にリストを挿入して平坦化 |
| `compare`           | `Ord::cmp`                               | 辞書順比較 (`T: Ord` が必要)     |
| `Seq.unfold`        | 手動実装                                 | シーケンスを生成                 |

### Traversable 操作

| F#                          | lambars                               | 説明                                 |
| --------------------------- | ------------------------------------- | ------------------------------------ |
| `List.traverse` (カスタム)  | `Traversable::traverse_option/result` | Option/Result でトラバース           |
| `List.sequence` (カスタム)  | `Traversable::sequence_option/result` | Option/Result エフェクトをシーケンス |
| Reader での `List.traverse` | `Traversable::traverse_reader`        | Reader エフェクトでトラバース        |
| State での `List.traverse`  | `Traversable::traverse_state`         | State エフェクトでトラバース         |
| Async での `List.traverse`  | `Traversable::traverse_io`            | IO エフェクトでトラバース            |
| Reader での `List.sequence` | `Traversable::sequence_reader`        | Reader エフェクトをシーケンス        |
| State での `List.sequence`  | `Traversable::sequence_state`         | State エフェクトをシーケンス         |
| Async での `List.sequence`  | `Traversable::sequence_io`            | IO エフェクトをシーケンス            |
| Reader での `List.iter`     | `Traversable::for_each_reader`        | Reader エフェクトで for-each         |
| State での `List.iter`      | `Traversable::for_each_state`         | State エフェクトで for-each          |
| IO での `List.iter`         | `Traversable::for_each_io`            | IO エフェクトで for-each             |

### コード例

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

#### エフェクト型による Traversable

```fsharp
// F# - Reader 風パターンでのカスタム traverse
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

// traverse_reader - Reader エフェクトでトラバース
#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
// result = vec![10, 20, 30]

// traverse_state - State エフェクトでトラバース (状態は左から右へ渡される)
let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
// result = vec![(0, "a"), (1, "b"), (2, "c")]
// final_index = 3

// traverse_io - IO エフェクトでトラバース (IO アクションは順次実行)
let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
// contents = vec!["content of a.txt", "content of b.txt"]
```

---

## 関数合成

### 演算子とマクロ

| F#                 | lambars                                           | 説明                 |
| ------------------ | ------------------------------------------------- | -------------------- |
| `\|>` (前方パイプ) | `pipe!`                                           | 値を関数に適用       |
| `<\|` (後方パイプ) | 関数呼び出し                                      | 関数を値に適用       |
| `>>` (前方合成)    | `compose!` (逆順)                                 | 左から右へ合成       |
| `<<` (後方合成)    | `compose!`                                        | 右から左へ合成       |
| 部分適用           | `partial!`                                        | いくつかの引数を固定 |
| カリー化 (自動)    | `curry!(fn, arity)` or `curry!(\|args...\| body)` | カリー化形式に変換   |

### コード例

#### F# パイプ演算子 vs lambars pipe! マクロ

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

#### F# 合成 vs lambars compose! マクロ

```fsharp
// F# 前方合成 (>>)
let transform = double >> addOne >> square
let result = transform 5  // 121

// F# 後方合成 (<<)
let transform2 = square << addOne << double
let result2 = transform2 5  // 121
```

```rust
// lambars
use lambars::compose;

fn double(x: i32) -> i32 { x * 2 }
fn add_one(x: i32) -> i32 { x + 1 }
fn square(x: i32) -> i32 { x * x }

// compose! は数学的 (右から左へ) 合成を使用
// これは F# の << 演算子と同等
let transform = compose!(square, add_one, double);
let result = transform(5);  // 121
```

#### F# 部分適用 vs lambars partial! マクロ

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

// 残りの引数のプレースホルダーとして __ を使用
let add_five = partial!(add, 5, __);
let result = add_five(3);  // 8
```

#### F# 自動カリー化 vs lambars curry! マクロ

```fsharp
// F# - 関数はデフォルトでカリー化される
let add a b c = a + b + c
let addFive = add 5
let addFiveAndTen = addFive 10
let result = addFiveAndTen 3  // 18
```

```rust
// lambars
use lambars::curry;

fn add(a: i32, b: i32, c: i32) -> i32 { a + b + c }

// 関数名 + アリティ形式を使用
let curried = curry!(add, 3);
let add_five = curried(5);
let add_five_and_ten = add_five(10);
let result = add_five_and_ten(3);  // 18

// またはクロージャ形式を使用
let curried = curry!(|a, b, c| add(a, b, c));
let add_five = curried(5);
let add_five_and_ten = add_five(10);
let result = add_five_and_ten(3);  // 18
```

#### ヘルパー関数

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

## コンピュテーション式と Effect システム

### 比較概要

| F#                        | lambars                  | 説明                                 |
| ------------------------- | ------------------------ | ------------------------------------ |
| `async { }`               | `AsyncIO` + `eff_async!` | 非同期計算                           |
| `result { }`              | Result での `eff!`       | Result ベースの計算                  |
| `option { }`              | Option での `eff!`       | Option ベースの計算                  |
| `seq { }` / `[ for ... ]` | `for_!` マクロ           | リスト/シーケンス生成                |
| `state { }`               | `State` Monad            | 状態を持つ計算                       |
| `reader { }`              | `Reader` Monad           | 環境の読み取り                       |
| `writer { }`              | `Writer` Monad           | ログ出力する計算                     |
| `rws { }` (カスタム)      | `RWS` Monad              | Reader + Writer + State の組み合わせ |

### コード例

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
// lambars ("async" feature が必要)
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

#### F# result 式 vs lambars Result での eff!

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

#### F# State Monad パターン vs lambars State

```fsharp
// F# (state monad ライブラリまたは手動のスレッディングを使用)
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

#### F# Reader パターン vs lambars Reader

```fsharp
// F# (reader パターンを使用)
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

#### F# RWS パターン vs lambars RWS Monad

F# には組み込みの RWS コンピュテーション式はありませんが、パターンは手動で実装できます。lambars は Reader、Writer、State の機能を組み合わせた専用の `RWS` Monad を提供します。

```fsharp
// F# - 手動の RWS パターン (Reader + Writer + State の組み合わせ)
type Config = { Multiplier: int }
type Log = string list

// RWS 風の関数: Config -> State -> (Result, State, Log)
let rwsComputation config state =
    let result = state * config.Multiplier
    let newState = result
    let log = [sprintf "Multiplied %d by %d" state config.Multiplier]
    (result, newState, log)

// 使用例
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

`RWS` Monad は以下の操作を提供します:

| F# パターン                           | lambars        | 説明                         |
| ------------------------------------- | -------------- | ---------------------------- |
| `fun config -> ...`                   | `RWS::ask`     | 環境にアクセス               |
| `fun config -> f config`              | `RWS::asks`    | 環境から派生した値にアクセス |
| `fun _ state -> (state, state, [])`   | `RWS::get`     | 現在の状態を取得             |
| `fun _ _ -> ((), newState, [])`       | `RWS::put`     | 新しい状態を設定             |
| `fun _ state -> ((), f state, [])`    | `RWS::modify`  | 関数で状態を変更             |
| `fun _ state -> (f state, state, [])` | `RWS::gets`    | 状態から派生した値を取得     |
| `fun _ state -> ((), state, log)`     | `RWS::tell`    | ログ出力に追加               |
| N/A                                   | `RWS::listen`  | 計算内でログにアクセス       |
| N/A                                   | `RWS::listens` | 変換されたログにアクセス     |
| N/A                                   | `RWS::local`   | 変更された環境で実行         |

#### F# シーケンス式 / リスト内包表記 vs lambars for\_!

```fsharp
// F# - リスト内包表記
let doubled = [ for x in [1; 2; 3; 4; 5] -> x * 2 ]
// doubled = [2; 4; 6; 8; 10]

// ネストした内包表記
let cartesian = [ for x in [1; 2] do
                  for y in [10; 20] -> x + y ]
// cartesian = [11; 21; 12; 22]

// シーケンス式
let doubledSeq = seq {
    for x in [1; 2; 3; 4; 5] -> x * 2
}
// 2, 4, 6, 8, 10 を生成する遅延シーケンス

// フィルタリング付き
let evens = [ for x in [1..10] do if x % 2 = 0 then yield x ]
// evens = [2; 4; 6; 8; 10]
```

```rust
// lambars - for_! マクロ
use lambars::for_;

let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
// doubled = vec![2, 4, 6, 8, 10]

// ネストした内包表記
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // 注: 内側のイテレーションには clone() が必要
    yield x + y
};
// cartesian = vec![11, 21, 12, 22]

// フィルタリング付き (std iterator メソッドを使用)
let evens: Vec<i32> = (1..=10).filter(|x| x % 2 == 0).collect();
// evens = vec![2, 4, 6, 8, 10]
```

### eff! と for\_! の使い分け

| シナリオ                    | 推奨マクロ            | 理由                           |
| --------------------------- | --------------------- | ------------------------------ |
| `option { }` / `result { }` | `eff!`                | 短絡評価を伴うモナド連鎖       |
| `async { }`                 | `eff_async!`          | 非同期モナド連鎖               |
| `[ for ... ]` / `seq { }`   | `for_!`               | yield を使ったリスト生成       |
| 直積                        | `for_!`               | 複数のイテレーション           |
| State/Reader/Writer         | `eff!`                | モナドエフェクト連鎖           |
| State/Reader/Writer + Async | `*_async_io` メソッド | トランスフォーマーの非同期統合 |

### AsyncIO を使った Monad Transformer

lambars は、F# の async ワークフローと reader/state パターンの組み合わせと同様に、Monad Transformer との AsyncIO 統合をサポートしています。

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

トランスフォーマーで利用可能な AsyncIO メソッド:

- `ReaderT`: `ask_async_io`, `asks_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`
- `StateT`: `get_async_io`, `gets_async_io`, `state_async_io`, `lift_async_io`, `pure_async_io`
- `WriterT`: `tell_async_io`, `lift_async_io`, `pure_async_io`, `flat_map_async_io`, `listen_async_io`

---

## アクティブパターンと Optics

### 比較

| F#                         | lambars           | 説明                    |
| -------------------------- | ----------------- | ----------------------- |
| アクティブパターン (完全)  | `Prism`           | enum バリアントのマッチ |
| アクティブパターン (部分)  | `Optional`        | マッチするかもしれない  |
| レコードフィールドアクセス | `Lens`            | フィールドの取得/設定   |
| ネストしたアクセス         | 合成された Optics | 深いアクセス            |

### コード例

#### F# アクティブパターン vs lambars Prism

```fsharp
// F# - パース用のアクティブパターン
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
// lambars - enum バリアント用の Prism を使用
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

#### F# レコード更新 vs lambars Lens

```fsharp
// F# - コピーと更新によるレコード
type Person = { Name: string; Age: int }

let person = { Name = "Alice"; Age = 30 }
let older = { person with Age = person.Age + 1 }
// older = { Name = "Alice"; Age = 31 }
```

```rust
// lambars - Lens を使用
use lambars::optics::Lens;
use lambars::lens;

#[derive(Clone)]
struct Person { name: String, age: i32 }

let age_lens = lens!(Person, age);

let person = Person { name: "Alice".to_string(), age: 30 };
let older = age_lens.modify(person, |a| a + 1);
// older.age = 31
```

#### F# ネストしたレコード更新 vs lambars 合成された Lens

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

## 遅延評価

### 比較

| F#              | lambars                | 説明           |
| --------------- | ---------------------- | -------------- |
| `lazy { expr }` | `Lazy::new(\|\| expr)` | 遅延計算       |
| `Lazy.Force`    | `Lazy::force`          | 評価を強制     |
| `Lazy.Value`    | `Lazy::force`          | 値にアクセス   |
| `seq { }`       | `Iterator`             | 遅延シーケンス |

### コード例

#### F# lazy vs lambars Lazy

```fsharp
// F#
let lazyValue = lazy (
    printfn "Computing..."
    42
)
// まだ何も出力されない

let result = lazyValue.Force()
// "Computing..." が出力され、result = 42

let result2 = lazyValue.Force()
// 何も出力されない (キャッシュされている)、result2 = 42
```

```rust
// lambars
use lambars::control::Lazy;

let lazy_value = Lazy::new(|| {
    println!("Computing...");
    42
});
// まだ何も出力されない

let result = lazy_value.force();
// "Computing..." が出力され、result = 42

let result2 = lazy_value.force();
// 何も出力されない (キャッシュされている)、result2 = 42
```

#### F# 遅延シーケンス vs Rust Iterator

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

## 型クラス / インターフェース

### 比較

| F# 概念                       | lambars トレイト     | 説明           |
| ----------------------------- | -------------------- | -------------- |
| `IComparable<'T>`             | `Ord` (std)          | 順序比較       |
| `IEquatable<'T>`              | `Eq` (std)           | 等価性         |
| `+` を持つインターフェース    | `Semigroup`          | 結合的な結合   |
| `Zero` を持つインターフェース | `Monoid`             | 単位元         |
| `IEnumerable<'T>`             | `IntoIterator` (std) | イテレーション |

### コード例

#### F# Monoid 風パターン vs lambars Monoid

```fsharp
// F# - 演算子オーバーロードを使用
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

#### F# ジェネリック制約 vs lambars トレイト境界

```fsharp
// F# - 静的メンバー制約を使用
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

## 永続データ構造

### 比較

| F# 型         | lambars 型                | 説明                          |
| ------------- | ------------------------- | ----------------------------- |
| `list<'T>`    | `PersistentList<T>`       | 不変の単方向リスト            |
| `Map<'K, 'V>` | `PersistentTreeMap<K, V>` | 不変の順序付きマップ          |
| `Set<'T>`     | `PersistentHashSet<T>`    | 不変の集合                    |
| -             | `PersistentVector<T>`     | 不変ベクタ (Clojure スタイル) |
| -             | `PersistentHashMap<K, V>` | 不変ハッシュマップ (HAMT)     |

### Map 操作

| F#                       | lambars               | 説明                                  |
| ------------------------ | --------------------- | ------------------------------------- |
| `Map.map f m`            | `map_values` メソッド | 値を変換                              |
| `Map.map f m` (f にキー) | `map_values` メソッド | 値を変換 (クロージャでキーが利用可能) |
| `Map.toSeq m`            | `entries` メソッド    | すべてのエントリを取得                |
| `Map.keys m`             | `keys` メソッド       | すべてのキーを取得                    |
| `Map.values m`           | `values` メソッド     | すべての値を取得                      |
| `Map.fold f m1 m2`       | `merge` メソッド      | マージ (右側が優先)                   |
| -                        | `merge_with` メソッド | カスタム解決でマージ                  |
| `Map.filter p m`         | `keep_if` メソッド    | マッチするエントリを保持              |
| -                        | `delete_if` メソッド  | マッチするエントリを削除              |
| `Map.partition p m`      | `partition` メソッド  | 述語で分割                            |
| `Map.pick f m`           | `filter_map` メソッド | フィルタと変換                        |

### コード例

#### F# List vs lambars PersistentList

```fsharp
// F#
let list = [1; 2; 3]
let extended = 0 :: list
// list = [1; 2; 3] (変更されていない)
// extended = [0; 1; 2; 3]

let head = List.head list  // 1
let tail = List.tail list  // [2; 3]
```

```rust
// lambars
use lambars::persistent::PersistentList;

let list = PersistentList::new().cons(3).cons(2).cons(1);
let extended = list.cons(0);
// list.len() = 3 (変更されていない)
// extended.len() = 4

let head = list.head();  // Some(&1)
let tail = list.tail();  // Some(PersistentList [2, 3])
```

#### F# Map vs lambars PersistentTreeMap

```fsharp
// F#
let map = Map.empty |> Map.add 1 "one" |> Map.add 2 "two"
let updated = map |> Map.add 1 "ONE"
// map.[1] = "one" (変更されていない)
// updated.[1] = "ONE"

let keys = map |> Map.toSeq |> Seq.map fst |> Seq.toList
// keys = [1; 2] (ソート済み)
```

```rust
// lambars
use lambars::persistent::PersistentTreeMap;

let map = PersistentTreeMap::new()
    .insert(1, "one")
    .insert(2, "two");
let updated = map.insert(1, "ONE");
// map.get(&1) = Some(&"one") (変更されていない)
// updated.get(&1) = Some(&"ONE")

let keys: Vec<&i32> = map.keys().collect();
// keys = vec![&1, &2] (ソート済み)
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

## まとめ: 主な違い

### 構文の違い

| 側面                       | F#                 | lambars (Rust)     |
| -------------------------- | ------------------ | ------------------ |
| 関数適用                   | `f x y`            | `f(x, y)`          |
| パイプ構文                 | `x \|> f`          | `pipe!(x, f)`      |
| 合成                       | `f >> g`           | `compose!(g, f)`   |
| CE での Let バインディング | `let! x = m`       | `x <= m;`          |
| ラムダ                     | `fun x -> x + 1`   | `\|x\| x + 1`      |
| 型注釈                     | `x: int`           | `x: i32`           |
| ジェネリック型             | `'T`               | `T`                |
| Option                     | `Some x` / `None`  | `Some(x)` / `None` |
| Result                     | `Ok x` / `Error e` | `Ok(x)` / `Err(e)` |

### 概念的な違い

1. **カリー化**: F# の関数はデフォルトでカリー化されますが、Rust は明示的な `curry!` マクロが必要です。

2. **型推論**: F# は Hindley-Milner を使用し、より積極的な推論を行います。Rust は明示的な型注釈が必要になることが多いです。

3. **可変性**: 両方とも不変がデフォルトですが、Rust の所有権モデルは複雑さを追加します。

4. **高階型**: F# も HKT を持ちませんが、インターフェースの動作が異なります。lambars は HKT エミュレーションのために GAT を使用します。

5. **コンピュテーション式**: F# の CE はより柔軟です。lambars の `eff!` はより制限されていますが、一般的なケースをカバーします。

6. **アクティブパターン**: F# のアクティブパターンはより強力です。lambars は同様の機能のために Prism/Optional を使用します。

---

## 移行のヒント

1. **`|>` を `pipe!` に置き換える**: 直接的な変換ですが、マクロのインポートを忘れずに。

2. **`>>` を `compose!` に置き換える**: `compose!` は右から左への順序 (`<<` と同様) を使用することに注意。

3. **`option { }` / `result { }` を `eff!` に置き換える**: `let!` の代わりに `<=` を使用。

4. **F# リストを `PersistentList` または `Vec` に置き換える**: 関数型パターンには `PersistentList`、パフォーマンスには `Vec` を使用。

5. **`Option.map` / `List.map` の代わりに `Functor::fmap` を使用**: 型間で統一されたインターフェース。

6. **`Option.bind` / `Result.bind` の代わりに `Monad::flat_map` を使用**: 同じ動作、異なる名前。

7. **明示的な型注釈を追加**: Rust の型推論は F# ほど積極的ではありません。
