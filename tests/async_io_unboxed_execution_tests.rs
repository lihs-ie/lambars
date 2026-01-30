//! Tests for AsyncIO unboxed execution path.
//!
//! Verifies that direct await and run_blocking work correctly with AsyncIO,
//! avoiding unnecessary Box::pin allocation from run_async().

#![cfg(feature = "async")]

use lambars::effect::AsyncIO;
use lambars::effect::async_io::runtime;
use rstest::rstest;

// =============================================================================
// Direct await vs run_async equivalence
// =============================================================================

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn direct_await_and_run_async_produce_same_result() {
    let direct = AsyncIO::pure(42).await;

    #[allow(deprecated)]
    let boxed = AsyncIO::pure(42).run_async().await;

    assert_eq!(direct, boxed);
    assert_eq!(direct, 42);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn direct_await_and_run_async_with_fmap_chain() {
    let direct = AsyncIO::pure(10).fmap(|x| x * 2).fmap(|x| x + 1).await;

    #[allow(deprecated)]
    let boxed = AsyncIO::pure(10)
        .fmap(|x| x * 2)
        .fmap(|x| x + 1)
        .run_async()
        .await;

    assert_eq!(direct, boxed);
    assert_eq!(direct, 21);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn direct_await_and_run_async_with_flat_map_chain() {
    let direct = AsyncIO::pure(10)
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .await;

    #[allow(deprecated)]
    let boxed = AsyncIO::pure(10)
        .flat_map(|x| AsyncIO::pure(x * 2))
        .flat_map(|x| AsyncIO::pure(x + 1))
        .run_async()
        .await;

    assert_eq!(direct, boxed);
    assert_eq!(direct, 21);
}

// =============================================================================
// run_blocking accepts AsyncIO directly
// =============================================================================

#[rstest]
fn run_blocking_accepts_async_io_directly() {
    let direct = runtime::run_blocking(AsyncIO::pure(42));

    #[allow(deprecated)]
    let via_run_async = runtime::run_blocking(AsyncIO::pure(42).run_async());

    assert_eq!(direct, 42);
    assert_eq!(direct, via_run_async);
}

#[rstest]
fn run_blocking_accepts_async_io_with_fmap_chain() {
    let direct = runtime::run_blocking(AsyncIO::pure(10).fmap(|x| x * 2).fmap(|x| x + 1));

    #[allow(deprecated)]
    let via_run_async = runtime::run_blocking(
        AsyncIO::pure(10)
            .fmap(|x| x * 2)
            .fmap(|x| x + 1)
            .run_async(),
    );

    assert_eq!(direct, 21);
    assert_eq!(direct, via_run_async);
}

#[rstest]
fn run_blocking_accepts_async_io_with_flat_map_chain() {
    let direct = runtime::run_blocking(
        AsyncIO::pure(10)
            .flat_map(|x| AsyncIO::pure(x * 2))
            .flat_map(|x| AsyncIO::pure(x + 1)),
    );

    #[allow(deprecated)]
    let via_run_async = runtime::run_blocking(
        AsyncIO::pure(10)
            .flat_map(|x| AsyncIO::pure(x * 2))
            .flat_map(|x| AsyncIO::pure(x + 1))
            .run_async(),
    );

    assert_eq!(direct, 21);
    assert_eq!(direct, via_run_async);
}

#[rstest]
fn try_run_blocking_accepts_async_io_directly() {
    let direct = runtime::try_run_blocking(AsyncIO::pure(42));

    #[allow(deprecated)]
    let via_run_async = runtime::try_run_blocking(AsyncIO::pure(42).run_async());

    assert_eq!(direct, Ok(42));
    assert_eq!(direct, via_run_async);
}
