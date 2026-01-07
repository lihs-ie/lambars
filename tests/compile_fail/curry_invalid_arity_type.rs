//! Test that passing a non-integer literal for arity produces a compile error.

fn add(first: i32, second: i32) -> i32 {
    first + second
}

fn main() {
    let _ = lambars::curry!(add, "two");
}
