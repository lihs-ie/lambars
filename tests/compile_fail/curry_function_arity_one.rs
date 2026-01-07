//! Test that arity of 1 produces a compile error.

fn identity(value: i32) -> i32 {
    value
}

fn main() {
    let _ = lambars::curry!(identity, 1);
}
