//! Test that a zero-argument closure produces a compile error.

fn main() {
    // curry! requires at least 2 arguments
    let _ = lambars::curry!(|| 42);
}
