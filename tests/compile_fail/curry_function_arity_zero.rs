//! Test that arity of 0 produces a compile error.

fn constant() -> i32 {
    42
}

fn main() {
    let _ = lambars::curry!(constant, 0);
}
