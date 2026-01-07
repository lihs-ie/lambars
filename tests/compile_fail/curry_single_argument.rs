//! Test that a single-argument closure produces a compile error.

fn main() {
    // curry! requires at least 2 arguments
    let _ = lambars::curry!(|a: i32| a + 1);
}
