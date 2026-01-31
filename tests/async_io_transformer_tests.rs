#![cfg(feature = "async")]
//! Tests for AsyncIO with Monad Transformers (ReaderT, StateT).
//!
//! This module tests the integration of AsyncIO with the monad transformer types.

use lambars::effect::{AsyncIO, ReaderT, StateT};
use rstest::rstest;

// =============================================================================
// ReaderT with AsyncIO Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_reader_transformer_pure_async_io() {
    // ReaderT::pure_async_io は環境を無視して値を返す
    let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::pure_async_io(42);
    let async_io = reader.run(999);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_new_with_async_io() {
    // ReaderT::new で環境から AsyncIO を生成
    let reader: ReaderT<i32, AsyncIO<i32>> =
        ReaderT::new(|environment| AsyncIO::pure(environment * 2));
    let async_io = reader.run(21);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_fmap_async_io() {
    // fmap_async_io で値を変換
    let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::new(AsyncIO::pure);
    let mapped = reader.fmap_async_io(|value| value * 2);
    let async_io = mapped.run(21);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_flat_map_async_io() {
    // flat_map_async_io でチェーン
    let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::new(AsyncIO::pure);
    let chained = reader.flat_map_async_io(|value| {
        ReaderT::new(move |environment| AsyncIO::pure(value + environment))
    });
    let async_io = chained.run(10);
    let result = async_io.await;
    assert_eq!(result, 20); // 10 + 10
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_ask_async_io() {
    // ask_async_io で環境を取得
    let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::ask_async_io();
    let async_io = reader.run(42);
    let result = async_io.await;
    assert_eq!(result, 42);
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_local_async_io() {
    // local_async_io で環境を変更
    let reader: ReaderT<i32, AsyncIO<i32>> =
        ReaderT::new(|environment| AsyncIO::pure(environment * 2));
    let modified = ReaderT::local_async_io(|environment| environment + 10, reader);
    let async_io = modified.run(5);
    let result = async_io.await;
    assert_eq!(result, 30); // (5 + 10) * 2
}

#[rstest]
#[tokio::test]
async fn test_reader_transformer_chain_multiple() {
    // 複数のチェーン
    let reader: ReaderT<i32, AsyncIO<i32>> = ReaderT::ask_async_io();
    let chained = reader
        .flat_map_async_io(|env1| ReaderT::new(move |env2| AsyncIO::pure(env1 + env2)))
        .fmap_async_io(|sum| sum * 2);
    let async_io = chained.run(10);
    let result = async_io.await;
    assert_eq!(result, 40); // (10 + 10) * 2
}

// =============================================================================
// StateT with AsyncIO Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_state_transformer_pure_async_io() {
    // StateT::pure_async_io は状態を変更せずに値を返す
    let state: StateT<i32, AsyncIO<(String, i32)>> = StateT::pure_async_io("hello".to_string());
    let async_io = state.run(42);
    let result = async_io.await;
    assert_eq!(result, ("hello".to_string(), 42));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_new_with_async_io() {
    // StateT::new で状態遷移を定義
    let state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(|current_state| AsyncIO::pure((current_state * 2, current_state + 1)));
    let async_io = state.run(10);
    let result = async_io.await;
    assert_eq!(result, (20, 11));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_eval_async() {
    // eval_async は結果値のみを返す
    let state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(|current_state| AsyncIO::pure((current_state * 2, current_state + 1)));
    let result = state.eval_async(10).await;
    assert_eq!(result, 20);
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_exec_async() {
    // exec_async は最終状態のみを返す
    let state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(|current_state| AsyncIO::pure((current_state * 2, current_state + 1)));
    let result = state.exec_async(10).await;
    assert_eq!(result, 11);
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_fmap_async_io() {
    // fmap_async_io で値を変換
    let state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(|current_state| AsyncIO::pure((current_state, current_state + 1)));
    let mapped = state.fmap_async_io(|value| value * 2);
    let result = mapped.run(10).await;
    assert_eq!(result, (20, 11));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_flat_map_async_io() {
    // flat_map_async_io でチェーン
    let state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(|current_state| AsyncIO::pure((current_state, current_state + 1)));
    let chained = state.flat_map_async_io(|value| {
        StateT::new(move |current_state| AsyncIO::pure((value + current_state, current_state * 2)))
    });
    // Initial state 10: first (10, 11), then (10 + 11, 22) = (21, 22)
    let result = chained.run(10).await;
    assert_eq!(result, (21, 22));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_get_async_io() {
    // get_async_io で現在の状態を取得
    let state: StateT<i32, AsyncIO<(i32, i32)>> = StateT::get_async_io();
    let result = state.run(42).await;
    assert_eq!(result, (42, 42));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_put_async_io() {
    // put_async_io で状態を置換
    let state: StateT<i32, AsyncIO<((), i32)>> =
        StateT::<i32, AsyncIO<((), i32)>>::put_async_io(100);
    let result = state.run(42).await;
    assert_eq!(result, ((), 100));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_modify_async_io() {
    // modify_async_io で状態を変更
    let state: StateT<i32, AsyncIO<((), i32)>> =
        StateT::<i32, AsyncIO<((), i32)>>::modify_async_io(|current_state| current_state * 2);
    let result = state.run(21).await;
    assert_eq!(result, ((), 42));
}

#[rstest]
#[tokio::test]
async fn test_state_transformer_chain_multiple() {
    // 複数のチェーン
    let increment: StateT<i32, AsyncIO<((), i32)>> =
        StateT::<i32, AsyncIO<((), i32)>>::modify_async_io(|count| count + 1);

    let computation = increment
        .flat_map_async_io(|_| {
            StateT::<i32, AsyncIO<((), i32)>>::modify_async_io(|count| count + 1)
        })
        .flat_map_async_io(|_| StateT::<i32, AsyncIO<(i32, i32)>>::get_async_io());

    let result = computation.run(0).await;
    assert_eq!(result, (2, 2));
}

// =============================================================================
// Combined ReaderT and StateT Tests
// =============================================================================

#[rstest]
#[tokio::test]
async fn test_combined_reader_and_state_workflow() {
    // 環境から値を読み取り、状態を更新するワークフロー

    // 設定：環境は乗数、状態はカウンター
    #[derive(Clone)]
    struct Config {
        multiplier: i32,
    }

    // ReaderT で環境を読み取り
    let read_config: ReaderT<Config, AsyncIO<i32>> =
        ReaderT::new(|config: Config| AsyncIO::pure(config.multiplier));

    let async_io = read_config.run(Config { multiplier: 5 });
    let multiplier = async_io.await;

    // StateT で状態を更新
    let update_state: StateT<i32, AsyncIO<(i32, i32)>> =
        StateT::new(move |current| AsyncIO::pure((current * multiplier, current + 1)));

    let result = update_state.run(10).await;
    assert_eq!(result, (50, 11)); // 10 * 5, 10 + 1
}
