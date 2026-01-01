# lambars / F# / Scala API 対応表

このドキュメントは lambars の API と、F# および Scala の対応する機能を比較したものです。

---

## 型クラス（Type Classes）

### TypeConstructor（HKT エミュレーション）

| lambars                 | F#                       | Scala                                             |
| ----------------------- | ------------------------ | ------------------------------------------------- |
| `TypeConstructor` trait | 該当なし（言語機能なし） | `Kind[F[_], A]`（kind-projector）/ Scala 3 の HKT |
| `WithType<B>` (GAT)     | 該当なし                 | `F[B]`                                            |

### Functor

| lambars                | F#                       | Scala (Cats)     |
| ---------------------- | ------------------------ | ---------------- |
| `Functor::fmap`        | `Option.map`, `List.map` | `Functor[F].map` |
| `FunctorMut::fmap_mut` | 該当なし                 | 該当なし         |

**対応型:**

- `Option<A>` → F#: `option<'a>` / Scala: `Option[A]`
- `Result<A, E>` → F#: `Result<'a, 'e>` / Scala: `Either[E, A]`
- `Vec<A>` → F#: `list<'a>` / Scala: `List[A]`
- `Box<A>` → F#: 該当なし / Scala: `Id[A]`
- `Identity<A>` → F#: 該当なし / Scala: `Id[A]`

### Applicative

| lambars                | F#                    | Scala (Cats)             |
| ---------------------- | --------------------- | ------------------------ |
| `Applicative::pure`    | `Some`, `Ok`          | `Applicative[F].pure`    |
| `Applicative::apply`   | 該当なし（CE で代替） | `Applicative[F].ap`      |
| `Applicative::map2`    | 該当なし（CE で代替） | `Applicative[F].map2`    |
| `Applicative::map3`    | 該当なし              | `Applicative[F].map3`    |
| `Applicative::product` | 該当なし              | `Applicative[F].product` |

### Monad

| lambars                  | F#                           | Scala (Cats)                 |
| ------------------------ | ---------------------------- | ---------------------------- |
| `Monad::flat_map`        | `Option.bind`, `Result.bind` | `Monad[F].flatMap`           |
| `Monad::and_then`        | `Option.bind`                | `flatMap`                    |
| `Monad::then`            | 該当なし                     | `>>` (flatMap ignoring left) |
| `MonadVec::flat_map_vec` | `List.collect`               | `List.flatMap`               |

### Foldable

| lambars                | F#                      | Scala (Cats)                   |
| ---------------------- | ----------------------- | ------------------------------ |
| `Foldable::fold_left`  | `List.fold`, `Seq.fold` | `Foldable[F].foldLeft`         |
| `Foldable::fold_right` | `List.foldBack`         | `Foldable[F].foldRight` (Eval) |
| `Foldable::fold_map`   | 該当なし                | `Foldable[F].foldMap`          |
| `Foldable::find`       | `List.tryFind`          | `Foldable[F].find`             |
| `Foldable::exists`     | `List.exists`           | `Foldable[F].exists`           |
| `Foldable::for_all`    | `List.forall`           | `Foldable[F].forall`           |
| `Foldable::length`     | `List.length`           | `Foldable[F].size`             |
| `Foldable::is_empty`   | `List.isEmpty`          | `Foldable[F].isEmpty`          |
| `Foldable::to_vec`     | `List.toArray`          | `Foldable[F].toList`           |

### Traversable

| lambars                        | F#                                  | Scala (Cats)           |
| ------------------------------ | ----------------------------------- | ---------------------- |
| `Traversable::traverse_option` | `List.traverseOptionM` (FSharpPlus) | `Traverse[F].traverse` |
| `Traversable::traverse_result` | `List.traverseResultM` (FSharpPlus) | `Traverse[F].traverse` |
| `Traversable::sequence_option` | `Option.sequenceList` (FSharpPlus)  | `Traverse[F].sequence` |
| `Traversable::sequence_result` | `Result.sequenceList` (FSharpPlus)  | `Traverse[F].sequence` |

**注:** Rust の型システム制約により、汎用的な `traverse<F: Applicative>` は実装できないため、`traverse_option` / `traverse_result` として個別提供。

### Semigroup

| lambars                | F#                       | Scala (Cats)            |
| ---------------------- | ------------------------ | ----------------------- |
| `Semigroup::combine`   | 該当なし（演算子で代替） | `Semigroup[A].combine`  |
| `Semigroup::combine_n` | 該当なし                 | `Semigroup[A].combineN` |

**対応型:**

- `String` → F#: `+` / Scala: `|+|`
- `Vec<A>` → F#: `@` / Scala: `|+|`
- `Option<A: Semigroup>` → F#: 該当なし / Scala: `|+|`
- `Sum<N>`, `Product<N>` → F#: 該当なし / Scala: 該当なし（個別実装）
- `Max<N>`, `Min<N>` → F#: 該当なし / Scala: 該当なし

### Monoid

| lambars                  | F#       | Scala (Cats)           |
| ------------------------ | -------- | ---------------------- |
| `Monoid::empty`          | 該当なし | `Monoid[A].empty`      |
| `Monoid::combine_all`    | 該当なし | `Monoid[A].combineAll` |
| `Monoid::is_empty_value` | 該当なし | 該当なし               |

---

## 関数合成ユーティリティ

### 関数合成

| lambars             | F#                                 | Scala                                          |
| ------------------- | ---------------------------------- | ---------------------------------------------- |
| `compose!(f, g, h)` | `f >> g >> h` または `h << g << f` | `f compose g compose h`                        |
| `pipe!(x, f, g, h)` | `x \|> f \|> g \|> h`              | `x.pipe(f).pipe(g).pipe(h)` または拡張メソッド |

### 部分適用・カリー化

| lambars                      | F#                               | Scala       |
| ---------------------------- | -------------------------------- | ----------- |
| `partial!(f, a, __)`         | 言語機能（デフォルトでカリー化） | `f(a, _)`   |
| `curry2!(f)` 〜 `curry6!(f)` | 言語機能（自動カリー化）         | `f.curried` |

### ユーティリティ関数

| lambars       | F#                  | Scala                              |
| ------------- | ------------------- | ---------------------------------- |
| `identity`    | `id`                | `identity`                         |
| `constant(x)` | `fun _ -> x`        | `const(x)` または `_ => x`         |
| `flip(f)`     | `flip` (FSharpPlus) | `Function.untupled(_.swap).tupled` |

---

## 制御構造

### Either

| lambars               | F#                         | Scala                |
| --------------------- | -------------------------- | -------------------- |
| `Either<L, R>`        | `Choice<'a, 'b>`           | `Either[L, R]`       |
| `Either::Left(l)`     | `Choice1Of2 l`             | `Left(l)`            |
| `Either::Right(r)`    | `Choice2Of2 r`             | `Right(r)`           |
| `either.is_left()`    | `Choice.isChoice1Of2`      | `either.isLeft`      |
| `either.is_right()`   | `Choice.isChoice2Of2`      | `either.isRight`     |
| `either.map_left(f)`  | 該当なし（パターンマッチ） | `either.left.map(f)` |
| `either.map_right(f)` | 該当なし（パターンマッチ） | `either.map(f)`      |

### Lazy（遅延評価）

| lambars                   | F#                    | Scala                                      |
| ------------------------- | --------------------- | ------------------------------------------ |
| `Lazy::new(\|\| expr)`    | `lazy expr`           | `lazy { expr }` または `Eval.later` (Cats) |
| `lazy_val.force()`        | `Lazy.force lazy_val` | `lazyVal` (暗黙評価) または `eval.value`   |
| `lazy_val.is_evaluated()` | `Lazy.isValueCreated` | 該当なし                                   |
| `Lazy::evaluated(x)`      | 該当なし              | `Eval.now(x)`                              |

**注:** lambars の `Lazy` は `RefCell` ベースの内部可変性でメモ化を実現（unsafe 不使用）。

### Trampoline（スタック安全再帰）

| lambars                       | F#       | Scala (Cats)            |
| ----------------------------- | -------- | ----------------------- |
| `Trampoline::done(x)`         | 該当なし | `Trampoline.done(x)`    |
| `Trampoline::suspend(\|\| t)` | 該当なし | `Trampoline.defer(t)`   |
| `Trampoline::flat_map(f)`     | 該当なし | `Trampoline.flatMap(f)` |
| `trampoline.run()`            | 該当なし | `trampoline.run`        |
| `trampoline.resume()`         | 該当なし | 該当なし                |

**注:** F# は末尾再帰最適化（TCO）を言語レベルでサポートするため、Trampoline は不要。

### Continuation（継続モナド）

| lambars                               | F#                          | Scala (Cats)   |
| ------------------------------------- | --------------------------- | -------------- |
| `Continuation::new(f)`                | `cont { ... }` (FSharpPlus) | `Cont[R, A]`   |
| `Continuation::pure(x)`               | `cont { return x }`         | `Cont.pure(x)` |
| `continuation.run(k)`                 | `Cont.run k cont`           | `cont.run(k)`  |
| `call_cc(f)`                          | `callCC` (FSharpPlus)       | `Cont.callCC`  |
| `call_with_current_continuation_once` | 該当なし                    | 該当なし       |

---

## 永続データ構造

### PersistentList

| lambars                         | F#             | Scala           |
| ------------------------------- | -------------- | --------------- |
| `PersistentList::empty()`       | `[]`           | `List.empty`    |
| `PersistentList::cons(x, list)` | `x :: list`    | `x :: list`     |
| `list.head()`                   | `List.head`    | `list.head`     |
| `list.tail()`                   | `List.tail`    | `list.tail`     |
| `list.is_empty()`               | `List.isEmpty` | `list.isEmpty`  |
| `list.len()`                    | `List.length`  | `list.length`   |
| `list.reverse()`                | `List.rev`     | `list.reverse`  |
| `list.append(other)`            | `list @ other` | `list ++ other` |

### PersistentVector

| lambars                     | F#                       | Scala                      |
| --------------------------- | ------------------------ | -------------------------- | --------------------- | -------------- |
| `PersistentVector::empty()` | `[                       |                            | ]`または`Array.empty` | `Vector.empty` |
| `PersistentVector::new()`   | `[                       |                            | ]`                    | `Vector()`     |
| `vector.push_back(x)`       | 該当なし（不変配列なし） | `vector :+ x`              |
| `vector.push_front(x)`      | 該当なし                 | `x +: vector`              |
| `vector.get(i)`             | `arr.[i]`                | `vector(i)`                |
| `vector.set(i, x)`          | 該当なし                 | `vector.updated(i, x)`     |
| `vector.pop_back()`         | 該当なし                 | `vector.init`              |
| `vector.pop_front()`        | 該当なし                 | `vector.tail`              |
| `vector.len()`              | `Array.length`           | `vector.length`            |
| `vector.is_empty()`         | `Array.isEmpty`          | `vector.isEmpty`           |
| `vector.concat(other)`      | `Array.append`           | `vector ++ other`          |
| `vector.split_at(i)`        | `Array.splitAt`          | `vector.splitAt(i)`        |
| `vector.take(n)`            | `Array.take`             | `vector.take(n)`           |
| `vector.drop(n)`            | `Array.skip`             | `vector.drop(n)`           |
| `vector.slice(start, end)`  | `arr.[start..end]`       | `vector.slice(start, end)` |
| `vector.iter()`             | `Array.toSeq`            | `vector.iterator`          |

**注:** lambars の `PersistentVector` は Radix Balanced Tree（32 分岐トライ）+ tail 最適化で実装。Scala の `Vector` と同等のアルゴリズム。

### PersistentHashMap

| lambars                       | F#                                | Scala                       |
| ----------------------------- | --------------------------------- | --------------------------- |
| `PersistentHashMap::empty()`  | `Map.empty`                       | `HashMap.empty`             |
| `PersistentHashMap::new()`    | `Map.empty`                       | `HashMap()`                 |
| `map.insert(k, v)`            | `Map.add k v map`                 | `map + (k -> v)`            |
| `map.get(k)`                  | `Map.tryFind k map`               | `map.get(k)`                |
| `map.contains_key(k)`         | `Map.containsKey k map`           | `map.contains(k)`           |
| `map.remove(k)`               | `Map.remove k map`                | `map - k`                   |
| `map.len()`                   | `Map.count map`                   | `map.size`                  |
| `map.is_empty()`              | `Map.isEmpty map`                 | `map.isEmpty`               |
| `map.keys()`                  | `Map.keys`                        | `map.keys`                  |
| `map.values()`                | `Map.values`                      | `map.values`                |
| `map.iter()`                  | `Map.toSeq`                       | `map.iterator`              |
| `map.merge(other)`            | 該当なし                          | `map ++ other`              |
| `map.get_or_else(k, default)` | `Map.findOrDefault k default map` | `map.getOrElse(k, default)` |

**注:** lambars の `PersistentHashMap` は HAMT（Hash Array Mapped Trie）で実装。

### PersistentHashSet

| lambars                           | F#                         | Scala                             |
| --------------------------------- | -------------------------- | --------------------------------- |
| `PersistentHashSet::empty()`      | `Set.empty`                | `HashSet.empty`                   |
| `set.insert(x)`                   | `Set.add x set`            | `set + x`                         |
| `set.contains(x)`                 | `Set.contains x set`       | `set.contains(x)`                 |
| `set.remove(x)`                   | `Set.remove x set`         | `set - x`                         |
| `set.len()`                       | `Set.count set`            | `set.size`                        |
| `set.is_empty()`                  | `Set.isEmpty set`          | `set.isEmpty`                     |
| `set.union(other)`                | `Set.union set other`      | `set \| other`                    |
| `set.intersection(other)`         | `Set.intersect set other`  | `set & other`                     |
| `set.difference(other)`           | `Set.difference set other` | `set -- other`                    |
| `set.symmetric_difference(other)` | 該当なし                   | `(set \| other) -- (set & other)` |
| `set.is_subset(other)`            | `Set.isSubset set other`   | `set.subsetOf(other)`             |
| `set.is_superset(other)`          | `Set.isSuperset set other` | `other.subsetOf(set)`             |
| `set.iter()`                      | `Set.toSeq`                | `set.iterator`                    |

### PersistentTreeMap

| lambars                      | F#                            | Scala                    |
| ---------------------------- | ----------------------------- | ------------------------ |
| `PersistentTreeMap::empty()` | `Map.empty`                   | `TreeMap.empty`          |
| `map.insert(k, v)`           | `Map.add k v map`             | `map + (k -> v)`         |
| `map.get(k)`                 | `Map.tryFind k map`           | `map.get(k)`             |
| `map.remove(k)`              | `Map.remove k map`            | `map - k`                |
| `map.min_key()`              | `Map.minKeyValue map \|> fst` | `map.firstKey`           |
| `map.max_key()`              | `Map.maxKeyValue map \|> fst` | `map.lastKey`            |
| `map.range(start, end)`      | 該当なし                      | `map.range(start, end)`  |
| `map.floor_key(k)`           | 該当なし                      | `map.to(k).lastOption`   |
| `map.ceiling_key(k)`         | 該当なし                      | `map.from(k).headOption` |

**注:** lambars の `PersistentTreeMap` は永続赤黒木で実装。F# の `Map` も内部的には赤黒木だが、一部のメソッドが異なる。

---

## Optics

### Lens

| lambars               | F#                          | Scala (Monocle)          |
| --------------------- | --------------------------- | ------------------------ |
| `Lens` trait          | `Lens` (FSharpPlus)         | `Lens[S, A]`             |
| `lens!(Type, field)`  | `Lens.create getter setter` | `GenLens[S](_.field)`    |
| `lens.get(&s)`        | `Lens.get lens s`           | `lens.get(s)`            |
| `lens.set(s, a)`      | `Lens.set lens a s`         | `lens.replace(a)(s)`     |
| `lens.modify(s, f)`   | `Lens.over lens f s`        | `lens.modify(f)(s)`      |
| `lens.compose(other)` | `lens >-> other`            | `lens.andThen(other)`    |
| `#[derive(Lenses)]`   | 該当なし                    | `@Lenses` アノテーション |

### Prism

| lambars                     | F#                                  | Scala (Monocle)                      |
| --------------------------- | ----------------------------------- | ------------------------------------ |
| `Prism` trait               | `Prism` (FSharpPlus)                | `Prism[S, A]`                        |
| `prism!(Type, Variant)`     | `Prism.create getOption reverseGet` | `Prism[S, A](getOption)(reverseGet)` |
| `prism.preview(&s)`         | `Prism.getOption prism s`           | `prism.getOption(s)`                 |
| `prism.review(a)`           | `Prism.reverseGet prism a`          | `prism.reverseGet(a)`                |
| `prism.set(s, a)`           | `Prism.set prism a s`               | `prism.replace(a)(s)`                |
| `prism.modify_option(s, f)` | `Prism.over prism f s`              | `prism.modify(f)(s)`                 |
| `prism.compose(other)`      | `prism >-> other`                   | `prism.andThen(other)`               |
| `#[derive(Prisms)]`         | 該当なし                            | `@Prisms` アノテーション             |

### Iso

| lambars                              | F#                          | Scala (Monocle)              |
| ------------------------------------ | --------------------------- | ---------------------------- |
| `Iso` trait                          | `Iso` (FSharpPlus)          | `Iso[S, A]`                  |
| `FunctionIso::new(get, reverse_get)` | `Iso.create get reverseGet` | `Iso[S, A](get)(reverseGet)` |
| `iso.get(&s)`                        | `Iso.get iso s`             | `iso.get(s)`                 |
| `iso.reverse_get(a)`                 | `Iso.reverseGet iso a`      | `iso.reverseGet(a)`          |
| `iso.reverse()`                      | `Iso.reverse iso`           | `iso.reverse`                |
| `iso.compose(other)`                 | `iso >-> other`             | `iso.andThen(other)`         |
| `iso_identity()`                     | 該当なし                    | `Iso.id`                     |
| `iso_swap()`                         | 該当なし                    | 該当なし                     |

### Optional

| lambars                        | F#                         | Scala (Monocle)          |
| ------------------------------ | -------------------------- | ------------------------ |
| `Optional` trait               | `Optional` (FSharpPlus)    | `Optional[S, A]`         |
| `optional.get_option(&s)`      | `Optional.getOption opt s` | `optional.getOption(s)`  |
| `optional.set(s, a)`           | `Optional.set opt a s`     | `optional.replace(a)(s)` |
| `optional.modify_option(s, f)` | `Optional.over opt f s`    | `optional.modify(f)(s)`  |
| `optional.is_present(&s)`      | 該当なし                   | `optional.nonEmpty(s)`   |
| `lens.compose_prism(prism)`    | `lens >?> prism`           | `lens.andThen(prism)`    |

### Traversal

| lambars                       | F#                        | Scala (Monocle)            |
| ----------------------------- | ------------------------- | -------------------------- |
| `Traversal` trait             | `Traversal` (FSharpPlus)  | `Traversal[S, A]`          |
| `VecTraversal::new()`         | 該当なし                  | `Traversal.fromTraverse`   |
| `traversal.get_all(&s)`       | `Traversal.getAll trav s` | `traversal.getAll(s)`      |
| `traversal.modify_all(s, f)`  | `Traversal.over trav f s` | `traversal.modify(f)(s)`   |
| `traversal.set_all(s, a)`     | `Traversal.set trav a s`  | `traversal.replace(a)(s)`  |
| `traversal.fold(&s)`          | `Traversal.fold trav s`   | `traversal.fold(s)`        |
| `traversal.find(&s, pred)`    | 該当なし                  | `traversal.find(pred)(s)`  |
| `traversal.exists(&s, pred)`  | 該当なし                  | `traversal.exist(pred)(s)` |
| `traversal.for_all(&s, pred)` | 該当なし                  | `traversal.all(pred)(s)`   |

---

## Effect System

### MTL 型クラス

| lambars          | F#                       | Scala (Cats MTL)    |
| ---------------- | ------------------------ | ------------------- |
| `MonadReader<R>` | `Reader` CE (FSharpPlus) | `MonadReader[F, R]` |
| `MonadState<S>`  | `State` CE (FSharpPlus)  | `MonadState[F, S]`  |
| `MonadWriter<W>` | `Writer` CE (FSharpPlus) | `MonadWriter[F, W]` |
| `MonadError<E>`  | `Result` CE              | `MonadError[F, E]`  |

#### MonadReader

| lambars       | F# (FSharpPlus)    | Scala (Cats MTL)                |
| ------------- | ------------------ | ------------------------------- |
| `ask()`       | `Reader.ask`       | `MonadReader[F, R].ask`         |
| `asks(f)`     | `Reader.asks f`    | `MonadReader[F, R].reader(f)`   |
| `local(f, m)` | `Reader.local f m` | `MonadReader[F, R].local(f)(m)` |

#### MonadState

| lambars     | F# (FSharpPlus)  | Scala (Cats MTL)              |
| ----------- | ---------------- | ----------------------------- |
| `get()`     | `State.get`      | `MonadState[F, S].get`        |
| `put(s)`    | `State.put s`    | `MonadState[F, S].set(s)`     |
| `modify(f)` | `State.modify f` | `MonadState[F, S].modify(f)`  |
| `gets(f)`   | `State.gets f`   | `MonadState[F, S].inspect(f)` |

#### MonadWriter

| lambars        | F# (FSharpPlus)     | Scala (Cats MTL)                 |
| -------------- | ------------------- | -------------------------------- |
| `tell(w)`      | `Writer.tell w`     | `MonadWriter[F, W].tell(w)`      |
| `listen(m)`    | `Writer.listen m`   | `MonadWriter[F, W].listen(m)`    |
| `pass(m)`      | `Writer.pass m`     | `MonadWriter[F, W].pass(m)`      |
| `writer(a, w)` | `Writer.create a w` | `MonadWriter[F, W].writer(a, w)` |

#### MonadError

| lambars              | F#           | Scala (Cats)                         |
| -------------------- | ------------ | ------------------------------------ |
| `raise_error(e)`     | `Error e`    | `MonadError[F, E].raiseError(e)`     |
| `handle_error(m, f)` | `try...with` | `MonadError[F, E].handleError(m)(f)` |
| `attempt(m)`         | `Result.try` | `MonadError[F, E].attempt(m)`        |

### ベースモナド

| lambars        | F# (FSharpPlus)  | Scala (Cats)                                     |
| -------------- | ---------------- | ------------------------------------------------ |
| `Reader<R, A>` | `Reader<'r, 'a>` | `Reader[R, A]` (= `Kleisli[Id, R, A]`)           |
| `State<S, A>`  | `State<'s, 'a>`  | `State[S, A]` (= `IndexedStateT[Eval, S, S, A]`) |
| `Writer<W, A>` | `Writer<'w, 'a>` | `Writer[W, A]` (= `WriterT[Id, W, A]`)           |
| `IO<A>`        | `IO<'a>` (FsIO)  | `IO[A]` (Cats Effect)                            |

#### Reader

| lambars              | F# (FSharpPlus)       | Scala               |
| -------------------- | --------------------- | ------------------- |
| `Reader::new(f)`     | `Reader f`            | `Reader(f)`         |
| `Reader::pure(a)`    | `Reader.Return a`     | `Reader.pure(a)`    |
| `reader.run(r)`      | `Reader.run r reader` | `reader.run(r)`     |
| `reader.flat_map(f)` | `reader >>= f`        | `reader.flatMap(f)` |
| `reader.map(f)`      | `reader \|>> f`       | `reader.map(f)`     |

#### State

| lambars          | F# (FSharpPlus)      | Scala                 |
| ---------------- | -------------------- | --------------------- |
| `State::new(f)`  | `State f`            | `State(f)`            |
| `State::pure(a)` | `State.Return a`     | `State.pure(a)`       |
| `state.run(s)`   | `State.run s state`  | `state.run(s).value`  |
| `state.eval(s)`  | `State.eval s state` | `state.runA(s).value` |
| `state.exec(s)`  | `State.exec s state` | `state.runS(s).value` |

#### Writer

| lambars             | F# (FSharpPlus)       | Scala             |
| ------------------- | --------------------- | ----------------- |
| `Writer::new(a, w)` | `Writer (a, w)`       | `Writer(w, a)`    |
| `Writer::pure(a)`   | `Writer.Return a`     | `Writer.value(a)` |
| `writer.run()`      | `Writer.run writer`   | `writer.run`      |
| `writer.value()`    | `Writer.value writer` | `writer.value`    |
| `writer.written()`  | `Writer.log writer`   | `writer.written`  |

#### IO

| lambars                          | F#                           | Scala (Cats Effect)                           |
| -------------------------------- | ---------------------------- | --------------------------------------------- |
| `IO::new(f)`                     | `IO.create f`                | `IO.delay(f)`                                 |
| `IO::pure(a)`                    | `IO.Return a`                | `IO.pure(a)`                                  |
| `IO::suspend(f)`                 | `IO.defer f`                 | `IO.defer(f)`                                 |
| `io.run()`                       | `IO.run io`                  | `io.unsafeRunSync()`                          |
| `io.flat_map(f)`                 | `io >>= f`                   | `io.flatMap(f)`                               |
| `IO::read_line()`                | `IO.readLine`                | `IO.readLine`                                 |
| `IO::print_line(s)`              | `IO.printLine s`             | `IO.println(s)`                               |
| `IO::read_file(path)`            | `IO.readFile path`           | `IO.blocking(Source.fromFile(path).mkString)` |
| `IO::write_file(path, contents)` | `IO.writeFile path contents` | `IO.blocking(...)`                            |

### Monad Transformer

| lambars            | F# (FSharpPlus)       | Scala (Cats)       |
| ------------------ | --------------------- | ------------------ |
| `ReaderT<R, M, A>` | `ReaderT<'r, 'm, 'a>` | `Kleisli[M, R, A]` |
| `StateT<S, M, A>`  | `StateT<'s, 'm, 'a>`  | `StateT[M, S, A]`  |
| `WriterT<W, M, A>` | `WriterT<'w, 'm, 'a>` | `WriterT[M, W, A]` |
| `ExceptT<E, M, A>` | `ResultT<'e, 'm, 'a>` | `EitherT[M, E, A]` |

#### ReaderT

| lambars            | F# (FSharpPlus)    | Scala              |
| ------------------ | ------------------ | ------------------ |
| `ReaderT::new(f)`  | `ReaderT f`        | `Kleisli(f)`       |
| `ReaderT::pure(a)` | `ReaderT.Return a` | `Kleisli.pure(a)`  |
| `ReaderT::lift(m)` | `ReaderT.lift m`   | `Kleisli.liftF(m)` |
| `reader_t.run(r)`  | `ReaderT.run r rt` | `readerT.run(r)`   |

#### StateT

| lambars           | F# (FSharpPlus)   | Scala             |
| ----------------- | ----------------- | ----------------- |
| `StateT::new(f)`  | `StateT f`        | `StateT(f)`       |
| `StateT::pure(a)` | `StateT.Return a` | `StateT.pure(a)`  |
| `StateT::lift(m)` | `StateT.lift m`   | `StateT.liftF(m)` |
| `state_t.run(s)`  | `StateT.run s st` | `stateT.run(s)`   |

#### WriterT

| lambars            | F# (FSharpPlus)    | Scala              |
| ------------------ | ------------------ | ------------------ |
| `WriterT::new(m)`  | `WriterT m`        | `WriterT(m)`       |
| `WriterT::pure(a)` | `WriterT.Return a` | `WriterT.value(a)` |
| `WriterT::lift(m)` | `WriterT.lift m`   | `WriterT.liftF(m)` |
| `writer_t.run()`   | `WriterT.run wt`   | `writerT.run`      |

#### ExceptT

| lambars                    | F# (FSharpPlus)      | Scala                    |
| -------------------------- | -------------------- | ------------------------ |
| `ExceptT::new(m)`          | `ResultT m`          | `EitherT(m)`             |
| `ExceptT::pure(a)`         | `ResultT.Return a`   | `EitherT.pure(a)`        |
| `ExceptT::lift(m)`         | `ResultT.lift m`     | `EitherT.liftF(m)`       |
| `ExceptT::throw_error(e)`  | `ResultT.Error e`    | `EitherT.leftT(e)`       |
| `except_t.run()`           | `ResultT.run et`     | `exceptT.value`          |
| `except_t.handle_error(f)` | `ResultT.catch f et` | `exceptT.handleError(f)` |

### do 記法 / Computation Expression

| lambars        | F#                                 | Scala               |
| -------------- | ---------------------------------- | ------------------- |
| `eff! { ... }` | `monad { ... }` / `result { ... }` | `for { ... } yield` |

```rust
// lambars
let result = eff! {
    x <= get_value();
    y <= compute(x);
    pure(x + y)
};
```

```fsharp
// F#
let result = monad {
    let! x = getValue()
    let! y = compute x
    return x + y
}
```

```scala
// Scala
val result = for {
  x <- getValue()
  y <- compute(x)
} yield x + y
```

---

## 備考

### 表記規則

- `該当なし` : 言語/ライブラリに対応する機能が存在しない
- `(FSharpPlus)` : F# の標準ライブラリではなく FSharpPlus ライブラリが必要
- `(Cats)` / `(Cats MTL)` / `(Cats Effect)` / `(Monocle)` : Scala の Cats エコシステムのライブラリ

### 主な違い

1. **HKT**: F# は HKT をサポートしていないため、型クラスの抽象化が限定的。Scala 3 は完全な HKT サポート。lambars は GAT でエミュレーション。

2. **カリー化**: F# は言語レベルで自動カリー化。Scala は `.curried` メソッドで変換。lambars はマクロで対応。

3. **TCO**: F# は言語レベルで末尾再帰最適化をサポート。Scala は `@tailrec` アノテーション。Rust は TCO 保証なしのため Trampoline が必要。

4. **Effect System**: Scala (Cats Effect) が最も充実。F# は Computation Expression で対応。lambars は MTL スタイルを採用。

5. **Optics**: Scala (Monocle) が最も充実。lambars は derive マクロでボイラープレートを削減。
