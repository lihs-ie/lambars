# lambars

[English](/README.md)

> **Note**: このドキュメントは AI によって翻訳されました。誤りや不自然な表現がある場合は、Issue または Pull Request でお知らせください。

Rust 向けの関数型プログラミングライブラリです。型クラス、永続データ構造、エフェクトシステムを提供します。

## 概要

lambars は、Rust の標準ライブラリでは提供されていない関数型プログラミングの抽象化を提供します。本ライブラリは Generic Associated Types (GAT) を使用して高階型 (HKT) をエミュレートし、Functor、Applicative、Monad などの強力な抽象化を可能にします。

### 機能

- **型クラス**: Functor, Applicative, Monad, Foldable, Traversable, Semigroup, Monoid
- **関数合成**: `compose!`, `pipe!`, `partial!`, `curry!`, `eff!`, `for_!`, `for_async!` マクロ
- **制御構造**: 遅延評価、スタック安全な再帰のための Trampoline、継続モナド
- **永続データ構造**: 構造共有による不変 Vector, HashMap, HashSet, TreeMap, List
- **Optics**: 不変データ操作のための Lens, Prism, Iso, Optional, Traversal
- **エフェクトシステム**: Reader, Writer, State モナド、IO/AsyncIO モナド、モナド変換子

### 言語比較ガイド

他の関数型プログラミング言語からの移行をお考えの方は、以下のガイドが lambars の概念を理解する手助けになります:

- [Haskell から lambars へ](/docs/external/comparison/Haskell/README.ja.md) - 型クラス、do 記法、optics などを網羅した包括的ガイド
- [Scala から lambars へ](/docs/external/comparison/Scala/README.ja.md) - Cats/Scalaz、Monocle、Scala 標準ライブラリをカバー
- [F# から lambars へ](/docs/external/comparison/F%23/README.ja.md) - F# コアライブラリ、計算式、アクティブパターンをカバー

## 必要条件

- Rust 1.92.0 以降
- Edition 2024

## インストール

`Cargo.toml` に追加:

```toml
[dependencies]
lambars = "0.1.0"
```

または特定の機能のみを指定:

```toml
[dependencies]
lambars = { version = "0.1.0", features = ["typeclass", "persistent", "effect"] }
```

## 機能フラグ

| 機能         | 説明                                        | 依存関係                                                                               |
| ------------ | ------------------------------------------- | -------------------------------------------------------------------------------------- |
| `default`    | 全機能（`full` と同じ）                     | `typeclass`, `compose`, `control`, `persistent`, `optics`, `derive`, `effect`, `async` |
| `full`       | 全機能                                      | `typeclass`, `compose`, `control`, `persistent`, `optics`, `derive`, `effect`, `async` |
| `typeclass`  | 型クラストレイト（Functor, Monad など）     | なし                                                                                   |
| `compose`    | 関数合成ユーティリティ                      | `typeclass`                                                                            |
| `control`    | 制御構造（Lazy, Trampoline）                | `typeclass`                                                                            |
| `persistent` | 永続データ構造                              | `typeclass`, `control`                                                                 |
| `optics`     | Optics（Lens, Prism など）                  | `typeclass`, `persistent`                                                              |
| `derive`     | Lens/Prism の derive マクロ                 | `optics`, `lambars-derive`                                                             |
| `effect`     | エフェクトシステム                          | `typeclass`, `control`                                                                 |
| `async`      | 非同期サポート（AsyncIO）                   | `effect`, `tokio`, `futures`                                                           |
| `arc`        | スレッドセーフな永続データ構造              | なし                                                                                   |
| `rayon`      | 永続データの並列イテレーション              | `arc`, `rayon`                                                                         |
| `serde`      | シリアライゼーション/デシリアライゼーション | `serde`                                                                                |

## クイックスタート

```rust
use lambars::prelude::*;

// 型クラスの使用
let numbers = vec![1, 2, 3, 4, 5];
let doubled: Vec<i32> = numbers.fmap(|x| x * 2);
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);

// 永続データ構造の使用
let vector: PersistentVector<i32> = (0..100).collect();
let updated = vector.update(50, 999).unwrap();
assert_eq!(vector.get(50), Some(&50));     // 元のデータは変更されない
assert_eq!(updated.get(50), Some(&999));   // 新しいバージョン

// 関数合成の使用
let add_one = |x: i32| x + 1;
let double = |x: i32| x * 2;
let composed = compose!(add_one, double);
assert_eq!(composed(5), 11); // add_one(double(5)) = 11
```

## モジュール

### 型クラス (`typeclass`)

関数型プログラミングの抽象化のための基本的な型クラス。

#### TypeConstructor（HKT エミュレーション）

```rust
use lambars::typeclass::TypeConstructor;

// TypeConstructor は型コンストラクタに対する汎用的な抽象化を可能にする
// Option<i32> を Option<String> に変換可能
let option: Option<i32> = Some(42);
```

#### Functor

コンテナ内の値に関数をマップする。

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

コンテナでラップされた関数を別のコンテナの値に適用する。

```rust
use lambars::typeclass::Applicative;

// 純粋な値を持ち上げる
let x: Option<i32> = <Option<()>>::pure(42);
assert_eq!(x, Some(42));

// 2つの Option 値を組み合わせる
let a = Some(1);
let b = Some(2);
let sum = a.map2(b, |x, y| x + y);
assert_eq!(sum, Some(3));
```

#### Monad

計算の逐次的な合成を可能にする。

```rust
use lambars::typeclass::Monad;

let result = Some(10)
    .flat_map(|x| Some(x * 2))
    .flat_map(|x| Some(x + 1));
assert_eq!(result, Some(21));
```

#### Semigroup と Monoid

単位元を持つ結合的二項演算。

```rust
use lambars::typeclass::{Semigroup, Monoid, Sum};

// 文字列連結
let hello = String::from("Hello, ");
let world = String::from("World!");
assert_eq!(hello.combine(world), "Hello, World!");

// Monoid による数値の合計
let numbers = vec![Sum::new(1), Sum::new(2), Sum::new(3)];
assert_eq!(Sum::combine_all(numbers), Sum::new(6));
```

#### Foldable

構造を単一の値に畳み込む。

```rust
use lambars::typeclass::Foldable;

let vec = vec![1, 2, 3, 4, 5];
let sum = vec.fold_left(0, |accumulator, x| accumulator + x);
assert_eq!(sum, 15);

let product = vec.fold_left(1, |accumulator, x| accumulator * x);
assert_eq!(product, 120);
```

#### Traversable

エフェクトを伴って構造をトラバースする。

```rust
use lambars::typeclass::Traversable;

let vec = vec![Some(1), Some(2), Some(3)];
let result = vec.sequence_option();
assert_eq!(result, Some(vec![1, 2, 3]));

let vec_with_none = vec![Some(1), None, Some(3)];
let result = vec_with_none.sequence_option();
assert_eq!(result, None);
```

##### エフェクト型でのトラバース

Traversable は Reader、State、IO、AsyncIO などのエフェクト型もサポートしています:

```rust
use lambars::typeclass::Traversable;
use lambars::effect::{Reader, State, IO};

// traverse_reader: 各要素に Reader を返す関数を適用
#[derive(Clone)]
struct Config { multiplier: i32 }

let numbers = vec![1, 2, 3];
let reader = numbers.traverse_reader(|n| {
    Reader::asks(move |config: &Config| n * config.multiplier)
});
let result = reader.run(Config { multiplier: 10 });
assert_eq!(result, vec![10, 20, 30]);

// traverse_state: 各要素を通じて状態をスレッディング
let items = vec!["a", "b", "c"];
let state = items.traverse_state(|item| {
    State::new(move |index: usize| ((index, item), index + 1))
});
let (result, final_index) = state.run(0);
assert_eq!(result, vec![(0, "a"), (1, "b"), (2, "c")]);
assert_eq!(final_index, 3);

// traverse_io: IO アクションを順次実行
let paths = vec!["a.txt", "b.txt"];
let io = paths.traverse_io(|path| {
    IO::new(move || format!("content of {}", path))
});
let contents = io.run_unsafe();
assert_eq!(contents, vec!["content of a.txt", "content of b.txt"]);
```

### 関数合成 (`compose`)

関数型プログラミングスタイルで関数を合成するためのユーティリティ。

#### compose!（右から左への合成）

```rust
use lambars::compose;

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// compose!(f, g)(x) = f(g(x))
let composed = compose!(add_one, double);
assert_eq!(composed(5), 11); // add_one(double(5)) = add_one(10) = 11
```

#### pipe!（左から右への合成）

```rust
use lambars::pipe;

fn add_one(x: i32) -> i32 { x + 1 }
fn double(x: i32) -> i32 { x * 2 }

// pipe!(x, f, g) = g(f(x))
let result = pipe!(5, double, add_one);
assert_eq!(result, 11); // add_one(double(5)) = 11
```

#### partial!（部分適用）

```rust
use lambars::partial;

fn add(first: i32, second: i32) -> i32 { first + second }

// 残りの引数のプレースホルダーとして __ を使用
let add_five = partial!(add, 5, __);
assert_eq!(add_five(3), 8);
```

#### curry!（カリー化）

複数引数の関数を単一引数関数のチェーンに変換する。

```rust
use lambars::curry;

fn add(first: i32, second: i32) -> i32 { first + second }

// クロージャ形式: curry!(|args...| body)
let curried_add = curry!(|a, b| add(a, b));
let add_five = curried_add(5);
assert_eq!(add_five(3), 8);

// 関数名 + アリティ形式: curry!(function_name, arity)
let curried_add = curry!(add, 2);
let add_ten = curried_add(10);
assert_eq!(add_ten(7), 17);

// 任意の引数数（2以上）で動作
fn sum_four(a: i32, b: i32, c: i32, d: i32) -> i32 { a + b + c + d }
let curried = curry!(sum_four, 4);
let step1 = curried(1);
let step2 = step1(2);
let step3 = step2(3);
assert_eq!(step3(4), 10);

// 部分適用は再利用可能
let add_one = curried_add(1);
assert_eq!(add_one(5), 6);
assert_eq!(add_one(10), 11); // 再利用可能！
```

#### ヘルパー関数

```rust
use lambars::compose::{identity, constant, flip};

// identity: 引数をそのまま返す
assert_eq!(identity(42), 42);

// constant: 常に同じ値を返す関数を作成
let always_five = constant(5);
assert_eq!(always_five(100), 5);

// flip: 2引数関数の引数を入れ替える
let subtract = |a, b| a - b;
let flipped = flip(subtract);
assert_eq!(flipped(3, 10), 7); // 10 - 3 = 7
```

### 制御構造 (`control`)

#### 遅延評価

計算を必要になるまで遅延させ、メモ化する。

```rust
use lambars::control::Lazy;

let lazy = Lazy::new(|| {
    println!("Computing...");
    42
});
// "Computing..." はまだ出力されない

let value = lazy.force();
// ここで "Computing..." が出力され、value は 42
assert_eq!(*value, 42);

// 2回目の呼び出しはキャッシュされた値を使用（再計算なし）
let value2 = lazy.force();
assert_eq!(*value2, 42);
```

#### Trampoline（スタック安全な再帰）

スタックオーバーフローなしで深い再帰を可能にする。

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

// 非常に大きな n でもスタックオーバーフローなしで動作
let result = factorial(10).run();
assert_eq!(result, 3628800);

// 100,000 回の反復でも安全に動作
let large_result = factorial(20).run();
assert_eq!(large_result, 2432902008176640000);
```

#### 継続モナド

高度な制御フローパターン用。

```rust
use lambars::control::Continuation;

let cont = Continuation::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| Continuation::pure(x + 1));

let result = cont.run(|x| x);
assert_eq!(result, 21);
```

### 永続データ構造 (`persistent`)

構造共有による効率的な更新を持つ不変データ構造。

#### PersistentList

O(1) の先頭追加を持つ単方向リンクリスト。

```rust
use lambars::persistent::PersistentList;

let list = PersistentList::new().cons(3).cons(2).cons(1);
assert_eq!(list.head(), Some(&1));

// 構造共有: 元のリストは保持される
let extended = list.cons(0);
assert_eq!(list.len(), 3);     // 元は変更されない
assert_eq!(extended.len(), 4); // 新しいリスト
```

#### PersistentVector

O(log32 N) のランダムアクセスと更新を持つ動的配列。

```rust
use lambars::persistent::PersistentVector;

let vector: PersistentVector<i32> = (0..100).collect();
assert_eq!(vector.get(50), Some(&50));

// 構造共有で元を保持
let updated = vector.update(50, 999).unwrap();
assert_eq!(vector.get(50), Some(&50));     // 元は変更されない
assert_eq!(updated.get(50), Some(&999));   // 新しいバージョン

// プッシュ操作
let pushed = vector.push_back(100);
assert_eq!(pushed.len(), 101);
```

#### PersistentHashMap

HAMT（Hash Array Mapped Trie）を使用した O(log32 N) 操作のハッシュマップ。

```rust
use lambars::persistent::PersistentHashMap;

let map = PersistentHashMap::new()
    .insert("one".to_string(), 1)
    .insert("two".to_string(), 2);
assert_eq!(map.get("one"), Some(&1));

// 構造共有
let updated = map.insert("one".to_string(), 100);
assert_eq!(map.get("one"), Some(&1));       // 元は変更されない
assert_eq!(updated.get("one"), Some(&100)); // 新しいバージョン

// 削除
let removed = map.remove("one");
assert_eq!(removed.get("one"), None);
```

#### PersistentHashSet

集合演算（和集合、積集合、差集合）を持つハッシュセット。

```rust
use lambars::persistent::PersistentHashSet;

let set = PersistentHashSet::new()
    .insert(1)
    .insert(2)
    .insert(3);
assert!(set.contains(&1));

// 集合演算
let other: PersistentHashSet<i32> = [2, 3, 4].into_iter().collect();
let union = set.union(&other);
let intersection = set.intersection(&other);
let difference = set.difference(&other);

assert_eq!(union.len(), 4);        // {1, 2, 3, 4}
assert_eq!(intersection.len(), 2); // {2, 3}
assert_eq!(difference.len(), 1);   // {1}

// HashSetView による遅延評価
let result: PersistentHashSet<i32> = set
    .view()
    .filter(|x| *x % 2 == 1)
    .map(|x| x * 10)
    .collect();
assert!(result.contains(&10));  // 1 * 10
assert!(result.contains(&30));  // 3 * 10
```

#### PersistentTreeMap

赤黒木を使用した O(log N) 操作の順序付きマップ。

```rust
use lambars::persistent::PersistentTreeMap;

let map = PersistentTreeMap::new()
    .insert(3, "three")
    .insert(1, "one")
    .insert(2, "two");

// エントリは常にソート順
let keys: Vec<&i32> = map.keys().collect();
assert_eq!(keys, vec![&1, &2, &3]);

// 範囲クエリ
let range: Vec<(&i32, &&str)> = map.range(1..=2).collect();
assert_eq!(range.len(), 2); // 1 と 2

// 最小/最大アクセス
assert_eq!(map.min(), Some((&1, &"one")));
assert_eq!(map.max(), Some((&3, &"three")));
```

### Optics (`optics`)

不変データ操作のための合成可能なアクセサ。

#### Lens

get/set 操作で単一フィールドにフォーカス。

```rust
use lambars::optics::{Lens, FunctionLens};
use lambars::lens;

#[derive(Clone, PartialEq, Debug)]
struct Address { street: String, city: String }

#[derive(Clone, PartialEq, Debug)]
struct Person { name: String, address: Address }

// マクロを使用してレンズを作成
let address_lens = lens!(Person, address);
let street_lens = lens!(Address, street);

// レンズを合成してネストしたフィールドにフォーカス
let person_street = address_lens.compose(street_lens);

let person = Person {
    name: "Alice".to_string(),
    address: Address {
        street: "Main St".to_string(),
        city: "Tokyo".to_string(),
    },
};

// ネストしたフィールドを取得
assert_eq!(*person_street.get(&person), "Main St");

// ネストしたフィールドを設定（新しい構造を返す）
let updated = person_street.set(person, "Oak Ave".to_string());
assert_eq!(updated.address.street, "Oak Ave");
assert_eq!(updated.address.city, "Tokyo"); // 他のフィールドは変更されない
```

#### Prism

列挙型のバリアントにフォーカス。

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

// プリズムを通じて値を構築
let constructed = circle_prism.review(10.0);
assert!(matches!(constructed, Shape::Circle(r) if (r - 10.0).abs() < 1e-10));
```

#### Iso

双方向の型変換。

```rust
use lambars::optics::FunctionIso;

// String <-> Vec<char> の同型
let string_chars_iso = FunctionIso::new(
    |s: String| s.chars().collect::<Vec<_>>(),
    |chars: Vec<char>| chars.into_iter().collect::<String>(),
);

let original = "hello".to_string();
let chars = string_chars_iso.get(original.clone());
assert_eq!(chars, vec!['h', 'e', 'l', 'l', 'o']);

// 往復
let back = string_chars_iso.reverse_get(chars);
assert_eq!(back, original);
```

#### Traversal

複数の要素にフォーカス。

```rust
use lambars::optics::{Traversal, VecTraversal};

let traversal = VecTraversal::<i32>::new();
let vec = vec![1, 2, 3, 4, 5];

// 全ての要素を取得
let all: Vec<&i32> = traversal.get_all(&vec);
assert_eq!(all, vec![&1, &2, &3, &4, &5]);

// 全ての要素を変更
let doubled = traversal.modify(vec, |x| x * 2);
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
```

### エフェクトシステム (`effect`)

モナドと変換子による型安全な副作用処理。

#### IO モナド

副作用を明示的に実行されるまで遅延させる。

```rust
use lambars::effect::IO;

// IO アクションを作成してチェーン
let io = IO::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| IO::pure(x + 1));

// run_unsafe が呼ばれるまで副作用は発生しない
assert_eq!(io.run_unsafe(), 21);

// 実際の副作用を持つ IO
let print_io = IO::print_line("Hello, World!");
print_io.run_unsafe(); // "Hello, World!" を出力
```

#### AsyncIO モナド

Tokio のような非同期ランタイムとの統合のための IO の非同期版。

```rust
use lambars::effect::AsyncIO;

// 非同期 IO アクションを作成してチェーン
let async_io = AsyncIO::pure(10)
    .fmap(|x| x * 2)
    .flat_map(|x| AsyncIO::pure(x + 1));

// 非同期で実行
let result = async_io.run_async().await;
assert_eq!(result, 21);

// 同期 IO を非同期に変換
use lambars::effect::IO;
let sync_io = IO::pure(42);
let async_io = sync_io.to_async();
let result = async_io.run_async().await;
assert_eq!(result, 42);
```

#### eff_async! マクロ

AsyncIO 計算のための do 記法。

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

#### Reader モナド

環境から読み取る計算。

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

#### State モナド

可変状態を持つ計算。

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

#### Writer モナド

出力を蓄積する計算。

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

#### RWS モナド

Reader + Writer + State の 3 つのエフェクト全てを必要とする計算のための統合モナド。

```rust
use lambars::effect::RWS;

#[derive(Clone)]
struct Config { multiplier: i32 }

// RWS は環境読み取り、ログ蓄積、状態管理を組み合わせる
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

エラーハンドリングの抽象化。

```rust
use lambars::effect::MonadError;

let computation: Result<i32, String> = Err("error".to_string());
let recovered = <Result<i32, String>>::catch_error(computation, |e| {
    Ok(e.len() as i32)
});
assert_eq!(recovered, Ok(5));
```

#### 代数効果

n^2 問題を解決するモナド変換子の代替。

```rust
use lambars::effect::algebraic::{
    Eff, Effect, Handler, ReaderEffect, ReaderHandler, StateEffect, StateHandler,
    WriterEffect, ErrorEffect, EffectRow, Member, Here, There,
};

// エフェクト行を使用してエフェクトを定義
type MyEffects = EffectRow!(ReaderEffect<String>, StateEffect<i32>);

// 複数のエフェクトを持つ計算を作成
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

// ハンドラで実行
let eff = computation();
let with_reader = ReaderHandler::new("hello".to_string()).run(eff);
let (result, final_state) = StateHandler::new(10).run(with_reader);
// result = 11, final_state = 15
```

**主な機能:**

- **n^2 問題なし**: 新しいエフェクトを追加しても新しい lift 実装が不要
- **型安全な合成**: エフェクト行が利用可能なエフェクトを追跡
- **スタック安全**: 深い `flat_map` チェーンでもスタックオーバーフローしない
- **標準エフェクト**: Reader, State, Writer, Error
- **カスタムエフェクト**: `define_effect!` マクロで独自のエフェクトを定義

```rust
use lambars::define_effect;
use lambars::effect::algebraic::{Effect, Eff};

// カスタムログエフェクトを定義
define_effect! {
    /// カスタムログエフェクト
    effect Log {
        /// メッセージをログ出力
        fn log(message: String) -> ();
    }
}

// マクロが生成するもの:
// - Effect を実装する LogEffect 構造体
// - LogEffect::log(message) -> Eff<LogEffect, ()>
// - fn log(&mut self, message: String) -> () を持つ LogHandler トレイト

// エフェクトを使用する計算を作成
fn log_computation() -> Eff<LogEffect, i32> {
    LogEffect::log("Hello".to_string())
        .then(LogEffect::log("World".to_string()))
        .then(Eff::pure(42))
}
```

#### モナド変換子

変換子でエフェクトをスタックする。

```rust
use lambars::effect::{ReaderT, StateT};

// ReaderT は Option に Reader 機能を追加
let reader_t = ReaderT::<i32, Option<i32>>::ask_option()
    .flat_map_option(|env| ReaderT::pure_option(env * 2));
let result = reader_t.run_option(21);
assert_eq!(result, Some(42));

// StateT は Result に State 機能を追加
let state_t = StateT::<i32, Result<i32, String>>::get_result()
    .flat_map_result(|s| StateT::pure_result(s * 2));
let (result, state) = state_t.run_result(10).unwrap();
assert_eq!(result, 20);
assert_eq!(state, 10);

// AsyncIO サポート付き ReaderT（"async" 機能が必要）
use lambars::effect::AsyncIO;

async fn example() {
    let reader_t = ReaderT::<i32, AsyncIO<i32>>::ask_async_io()
        .flat_map_async_io(|env| ReaderT::pure_async_io(env * 2));
    let result = reader_t.run_async_io(21).run_async().await;
    assert_eq!(result, 42);
}
```

#### eff! マクロ（do 記法）

モナド計算のための便利な構文。

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

// None で短絡する
let result = eff! {
    x <= Some(5);
    y <= None::<i32>;
    Some(x + y)
};
assert_eq!(result, None);
```

#### for\_! マクロ（リスト内包表記）

Vec とイテレータのための Scala/Haskell スタイルのリスト内包表記。

```rust
use lambars::for_;

// 基本的なリスト内包表記
let doubled: Vec<i32> = for_! {
    x <= vec![1, 2, 3, 4, 5];
    yield x * 2
};
assert_eq!(doubled, vec![2, 4, 6, 8, 10]);

// ネストした内包表記（直積）
let xs = vec![1, 2];
let ys = vec![10, 20];
let cartesian: Vec<i32> = for_! {
    x <= xs;
    y <= ys.clone();  // 内側のイテレーションにはクローンが必要
    yield x + y
};
assert_eq!(cartesian, vec![11, 21, 12, 22]);

// let 束縛付き
let result: Vec<i32> = for_! {
    x <= vec![1, 2, 3];
    let doubled = x * 2;
    yield doubled + 1
};
assert_eq!(result, vec![3, 5, 7]);
```

#### for_async! マクロ（非同期リスト内包表記）

非同期操作を伴うリスト内包表記のための `for_!` の非同期版。遅延評価のために `AsyncIO<Vec<T>>` を返す。

```rust
use lambars::for_async;
use lambars::effect::AsyncIO;

async fn example() {
    // 基本的な非同期リスト内包表記
    let urls = vec!["http://a.com", "http://b.com"];
    let result: AsyncIO<Vec<String>> = for_async! {
        url <= urls;
        yield url.to_uppercase()
    };
    let uppercase_urls = result.run_async().await;
    assert_eq!(uppercase_urls, vec!["HTTP://A.COM", "HTTP://B.COM"]);

    // <~ 演算子を使用した AsyncIO 束縛
    let result: AsyncIO<Vec<i32>> = for_async! {
        x <= vec![1, 2, 3];
        doubled <~ AsyncIO::pure(x * 2);  // <~ は AsyncIO からバインド
        yield doubled + 1
    };
    let values = result.run_async().await;
    assert_eq!(values, vec![3, 5, 7]);

    // 非同期を伴うネストしたイテレーション
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

**構文:**

- `pattern <= collection;` - IntoIterator からバインド（for ループ）
- `pattern <~ async_io;` - AsyncIO からバインド（await）
- `let pattern = expr;` - 純粋な let 束縛
- `yield expr` - 終端式（Vec に収集）

#### eff! vs for\_! vs for_async! : どれを使うか

| シナリオ               | マクロ       | 理由                             |
| ---------------------- | ------------ | -------------------------------- |
| Option/Result チェーン | `eff!`       | None/Err で短絡                  |
| IO/State/Reader/Writer | `eff!`       | FnOnce ベースのモナド            |
| Vec/Iterator 生成      | `for_!`      | FnMut ベース、yield を使用       |
| 直積                   | `for_!`      | 複数のイテレーション             |
| 非同期モナドチェーン   | `eff_async!` | 逐次的な非同期操作               |
| 非同期リスト生成       | `for_async!` | yield を伴う非同期イテレーション |

## 安全性

本ライブラリは安全性を念頭に構築されています:

- `#![forbid(unsafe_code)]` - unsafe コードなし
- `#![warn(clippy::all, clippy::pedantic, clippy::nursery)]` - 厳格なリント
- プロパティベーステストによる包括的なテストカバレッジ

## ライセンス

以下のいずれかでライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好みで選択してください。

## コントリビューション

コントリビューションを歓迎します！お気軽にプルリクエストを送信してください。
