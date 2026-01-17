//! Macros for defining algebraic effects.
//!
//! This module provides the `define_effect!` macro for declaratively defining
//! effects with their operations and generating the necessary boilerplate code.
//!
//! # Thread-Safe Tag Generation
//!
//! Operation tags are generated using an atomic counter to ensure uniqueness
//! across threads and modules. Each operation defined by `define_effect!`
//! receives a unique tag at initialization time.
//!
//! # Examples
//!
//! ```rust
//! use lambars::define_effect;
//! use lambars::effect::algebraic::{Effect, Eff};
//!
//! define_effect! {
//!     /// A counter effect for tracking count operations.
//!     effect Counter {
//!         /// Increments the counter.
//!         fn increment() -> ();
//!         /// Gets the current count value.
//!         fn get_count() -> i32;
//!     }
//! }
//!
//! // The macro generates:
//! // - CounterEffect struct implementing Effect
//! // - CounterEffect::increment() and CounterEffect::get_count() methods
//! // - CounterHandler trait for implementing handlers
//!
//! assert_eq!(CounterEffect::NAME, "Counter");
//! ```

use std::sync::atomic::{AtomicU32, Ordering};

use super::eff::OperationTag;

/// Global counter for operation tags.
///
/// This counter starts at 1000 to avoid conflicts with manually defined
/// operation tags in the standard effects (Reader, State, Writer, Error).
static NEXT_OPERATION_TAG_COUNTER: AtomicU32 = AtomicU32::new(1000);

/// Generates a new unique operation tag.
///
/// This function is thread-safe and generates monotonically increasing tags.
/// It is used by the `define_effect!` macro to assign unique tags to each
/// operation.
///
/// # Examples
///
/// ```rust
/// use lambars::effect::algebraic::macros::next_operation_tag;
///
/// let tag1 = next_operation_tag();
/// let tag2 = next_operation_tag();
/// assert_ne!(tag1, tag2);
/// ```
#[must_use]
#[inline]
pub fn next_operation_tag() -> OperationTag {
    let tag_value = NEXT_OPERATION_TAG_COUNTER.fetch_add(1, Ordering::SeqCst);
    OperationTag::new(tag_value)
}

/// Defines an algebraic effect with its operations.
///
/// This macro generates:
/// 1. An effect struct named `{Name}Effect` implementing [`Effect`](crate::effect::algebraic::Effect)
/// 2. Operation methods on the effect struct returning `Eff<{Name}Effect, ReturnType>`
/// 3. A handler trait named `{Name}Handler` for implementing effect handlers
///
/// # Syntax
///
/// ```text
/// define_effect! {
///     /// Optional doc comment
///     effect EffectName {
///         /// Operation doc comment
///         fn operation_name(param1: Type1, param2: Type2) -> ReturnType;
///         // ... more operations
///     }
/// }
/// ```
///
/// # Generated Code
///
/// For an effect named `Foo` with operation `bar(x: i32) -> String`:
///
/// - `FooEffect` struct implementing `Effect` with `NAME = "Foo"`
/// - `FooEffect::bar(x: i32) -> Eff<FooEffect, String>` method
/// - `FooHandler` trait with `fn bar(&mut self, x: i32) -> String`
///
/// # Examples
///
/// ## Simple Effect (No Parameters)
///
/// ```rust
/// use lambars::define_effect;
/// use lambars::effect::algebraic::{Effect, Eff};
///
/// define_effect! {
///     effect Random {
///         fn next_int() -> i32;
///     }
/// }
///
/// assert_eq!(RandomEffect::NAME, "Random");
/// ```
///
/// ## Effect with Parameters
///
/// ```rust
/// use lambars::define_effect;
/// use lambars::effect::algebraic::{Effect, Eff};
///
/// define_effect! {
///     /// Console I/O effect.
///     effect Console {
///         /// Prints a line to the console.
///         fn print_line(message: String) -> ();
///         /// Reads a line from the console.
///         fn read_line() -> String;
///     }
/// }
///
/// assert_eq!(ConsoleEffect::NAME, "Console");
/// ```
///
/// ## Multiple Operations
///
/// ```rust
/// use lambars::define_effect;
/// use lambars::effect::algebraic::{Effect, Eff};
///
/// define_effect! {
///     effect FileSystem {
///         fn read_file(path: String) -> String;
///         fn write_file(path: String, content: String) -> ();
///         fn delete_file(path: String) -> bool;
///     }
/// }
///
/// assert_eq!(FileSystemEffect::NAME, "FileSystem");
/// ```
#[macro_export]
macro_rules! define_effect {
    (
        $(#[$meta:meta])*
        effect $name:ident {
            $(
                $(#[$op_meta:meta])*
                fn $op_name:ident($($param:ident: $param_ty:ty),* $(,)?) -> $ret_ty:ty;
            )*
        }
    ) => {
        $crate::paste::paste! {
            $(#[$meta])*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
            pub struct [<$name Effect>];

            impl $crate::effect::algebraic::Effect for [<$name Effect>] {
                const NAME: &'static str = stringify!($name);
            }

            #[doc(hidden)]
            #[allow(non_upper_case_globals, dead_code)]
            mod [<__ $name:snake _operations>] {
                use std::sync::LazyLock;
                use $crate::effect::algebraic::OperationTag;

                $(
                    pub static [<$op_name:upper _TAG>]: LazyLock<OperationTag> =
                        LazyLock::new(|| {
                            $crate::effect::algebraic::macros::next_operation_tag()
                        });
                )*
            }

            impl [<$name Effect>] {
                $(
                    $(#[$op_meta])*
                    #[must_use]
                    pub fn $op_name($($param: $param_ty),*) -> $crate::effect::algebraic::Eff<Self, $ret_ty> {
                        $crate::effect::algebraic::Eff::<Self, $ret_ty>::perform_raw::<$ret_ty>(
                            *[<__ $name:snake _operations>]::[<$op_name:upper _TAG>],
                            ($($param,)*)
                        )
                    }
                )*
            }

            $(#[$meta])*
            pub trait [<$name Handler>] {
                $(
                    $(#[$op_meta])*
                    fn $op_name(&mut self $(, $param: $param_ty)*) -> $ret_ty;
                )*
            }
        }
    };
}

#[cfg(test)]
#[allow(
    dead_code,
    clippy::no_effect_underscore_binding,
    clippy::default_constructed_unit_structs,
    clippy::unused_unit,
    clippy::doc_markdown
)]
mod tests {
    use super::*;
    use crate::effect::algebraic::Effect;
    use rstest::rstest;

    // next_operation_tag Tests

    #[rstest]
    fn next_operation_tag_generates_unique_tags() {
        let tag1 = next_operation_tag();
        let tag2 = next_operation_tag();
        let tag3 = next_operation_tag();

        assert_ne!(tag1, tag2);
        assert_ne!(tag2, tag3);
        assert_ne!(tag1, tag3);
    }

    #[rstest]
    fn next_operation_tag_is_monotonically_increasing() {
        let tag1 = next_operation_tag();
        let tag2 = next_operation_tag();

        // Tags should increase (comparing internal values)
        assert!(tag1.0 < tag2.0);
    }

    #[rstest]
    fn next_operation_tag_starts_at_or_above_1000() {
        let tag = next_operation_tag();
        assert!(tag.0 >= 1000);
    }

    // define_effect! Macro - Simple Effect Tests

    define_effect! {
        /// A simple test effect with no parameters.
        effect SimpleTest {
            /// Gets a value.
            fn get_value() -> i32;
        }
    }

    #[rstest]
    fn define_effect_creates_effect_struct() {
        let _effect = SimpleTestEffect;
        assert_eq!(SimpleTestEffect::NAME, "SimpleTest");
    }

    #[rstest]
    fn define_effect_effect_is_debug() {
        let effect = SimpleTestEffect;
        let debug_string = format!("{effect:?}");
        assert!(debug_string.contains("SimpleTestEffect"));
    }

    #[rstest]
    fn define_effect_effect_is_clone() {
        let effect = SimpleTestEffect;
        let _cloned = effect;
    }

    #[rstest]
    fn define_effect_effect_is_copy() {
        let effect = SimpleTestEffect;
        let _copied = effect;
    }

    #[rstest]
    fn define_effect_effect_is_eq() {
        let effect1 = SimpleTestEffect;
        let effect2 = SimpleTestEffect;
        assert_eq!(effect1, effect2);
    }

    #[rstest]
    fn define_effect_effect_is_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let effect = SimpleTestEffect;
        let mut hasher = DefaultHasher::new();
        effect.hash(&mut hasher);
        let _hash = hasher.finish();
    }

    #[rstest]
    fn define_effect_effect_is_default() {
        let _effect = SimpleTestEffect::default();
    }

    #[rstest]
    fn define_effect_creates_operation_method() {
        let _computation = SimpleTestEffect::get_value();
    }

    // define_effect! Macro - Effect with Parameters Tests

    define_effect! {
        /// An effect with parameters.
        effect WithParams {
            /// Adds two numbers.
            fn add(left: i32, right: i32) -> i32;
            /// Greets with a message.
            fn greet(name: String) -> String;
        }
    }

    #[rstest]
    fn define_effect_with_params_creates_effect_struct() {
        assert_eq!(WithParamsEffect::NAME, "WithParams");
    }

    #[rstest]
    fn define_effect_with_params_creates_operation_methods() {
        let _add_computation = WithParamsEffect::add(1, 2);
        let _greet_computation = WithParamsEffect::greet("World".to_string());
    }

    // define_effect! Macro - Handler Trait Tests

    #[rstest]
    fn define_effect_creates_handler_trait() {
        // Verify that the handler trait exists and can be implemented
        struct TestHandler;

        impl SimpleTestHandler for TestHandler {
            fn get_value(&mut self) -> i32 {
                42
            }
        }

        let mut handler = TestHandler;
        assert_eq!(SimpleTestHandler::get_value(&mut handler), 42);
    }

    #[rstest]
    fn define_effect_with_params_creates_handler_trait() {
        struct TestWithParamsHandler;

        impl WithParamsHandler for TestWithParamsHandler {
            fn add(&mut self, left: i32, right: i32) -> i32 {
                left + right
            }

            fn greet(&mut self, name: String) -> String {
                format!("Hello, {name}!")
            }
        }

        let mut handler = TestWithParamsHandler;
        assert_eq!(WithParamsHandler::add(&mut handler, 2, 3), 5);
        assert_eq!(
            WithParamsHandler::greet(&mut handler, "Alice".to_string()),
            "Hello, Alice!"
        );
    }

    // define_effect! Macro - Operation Tag Uniqueness Tests

    define_effect! {
        effect EffectA {
            fn operation_a() -> ();
        }
    }

    define_effect! {
        effect EffectB {
            fn operation_b() -> ();
        }
    }

    #[rstest]
    fn operation_tags_are_unique_across_effects() {
        let tag_a = *__effect_a_operations::OPERATION_A_TAG;
        let tag_b = *__effect_b_operations::OPERATION_B_TAG;
        assert_ne!(tag_a, tag_b);
    }

    define_effect! {
        effect MultiOp {
            fn op_one() -> ();
            fn op_two() -> ();
            fn op_three() -> ();
        }
    }

    #[rstest]
    fn operation_tags_are_unique_within_effect() {
        let tag_one = *__multi_op_operations::OP_ONE_TAG;
        let tag_two = *__multi_op_operations::OP_TWO_TAG;
        let tag_three = *__multi_op_operations::OP_THREE_TAG;

        assert_ne!(tag_one, tag_two);
        assert_ne!(tag_two, tag_three);
        assert_ne!(tag_one, tag_three);
    }

    // define_effect! Macro - Complex Effect Tests

    define_effect! {
        /// File system effect for I/O operations.
        effect FileSystem {
            /// Reads the contents of a file.
            fn read_file(path: String) -> String;
            /// Writes content to a file.
            fn write_file(path: String, content: String) -> ();
            /// Checks if a file exists.
            fn file_exists(path: String) -> bool;
            /// Deletes a file.
            fn delete_file(path: String) -> bool;
        }
    }

    #[rstest]
    fn define_effect_complex_effect_has_correct_name() {
        assert_eq!(FileSystemEffect::NAME, "FileSystem");
    }

    #[rstest]
    fn define_effect_complex_effect_has_all_operations() {
        let _read = FileSystemEffect::read_file("test.txt".to_string());
        let _write = FileSystemEffect::write_file("test.txt".to_string(), "content".to_string());
        let _exists = FileSystemEffect::file_exists("test.txt".to_string());
        let _delete = FileSystemEffect::delete_file("test.txt".to_string());
    }

    #[rstest]
    fn define_effect_complex_handler_can_be_implemented() {
        struct MockFileSystem {
            files: std::collections::HashMap<String, String>,
        }

        impl FileSystemHandler for MockFileSystem {
            fn read_file(&mut self, path: String) -> String {
                self.files.get(&path).cloned().unwrap_or_default()
            }

            fn write_file(&mut self, path: String, content: String) -> () {
                self.files.insert(path, content);
            }

            fn file_exists(&mut self, path: String) -> bool {
                self.files.contains_key(&path)
            }

            fn delete_file(&mut self, path: String) -> bool {
                self.files.remove(&path).is_some()
            }
        }

        let mut fs = MockFileSystem {
            files: std::collections::HashMap::new(),
        };

        assert!(!FileSystemHandler::file_exists(
            &mut fs,
            "test.txt".to_string()
        ));

        FileSystemHandler::write_file(&mut fs, "test.txt".to_string(), "Hello".to_string());
        assert!(FileSystemHandler::file_exists(
            &mut fs,
            "test.txt".to_string()
        ));
        assert_eq!(
            FileSystemHandler::read_file(&mut fs, "test.txt".to_string()),
            "Hello"
        );

        assert!(FileSystemHandler::delete_file(
            &mut fs,
            "test.txt".to_string()
        ));
        assert!(!FileSystemHandler::file_exists(
            &mut fs,
            "test.txt".to_string()
        ));
    }

    // define_effect! Macro - Trailing Comma Tests

    define_effect! {
        effect TrailingComma {
            fn with_trailing(value: i32,) -> i32;
        }
    }

    #[rstest]
    fn define_effect_handles_trailing_comma() {
        let _computation = TrailingCommaEffect::with_trailing(42);
    }

    // define_effect! Macro - Unit Return Type Tests

    define_effect! {
        effect UnitReturn {
            fn do_something() -> ();
            fn do_with_param(value: i32) -> ();
        }
    }

    #[rstest]
    fn define_effect_handles_unit_return_type() {
        let _computation1 = UnitReturnEffect::do_something();
        let _computation2 = UnitReturnEffect::do_with_param(42);
    }

    // define_effect! Macro - Handler Integration Tests

    define_effect! {
        /// Counter effect for testing handler integration.
        effect TestableCounter {
            /// Increments the counter.
            fn increment() -> ();
            /// Decrements the counter.
            fn decrement() -> ();
            /// Gets the current value.
            fn get_value() -> i32;
            /// Resets the counter to a value.
            fn reset(value: i32) -> ();
        }
    }

    struct TestableCounterHandlerImpl {
        counter: std::cell::RefCell<i32>,
    }

    impl TestableCounterHandlerImpl {
        fn new(initial: i32) -> Self {
            Self {
                counter: std::cell::RefCell::new(initial),
            }
        }

        fn run<A: 'static>(
            &self,
            computation: crate::effect::algebraic::Eff<TestableCounterEffect, A>,
        ) -> A {
            use crate::effect::algebraic::eff::{EffInner, EffQueueStack, OperationTag};
            use std::any::Any;

            enum LoopState {
                ExecuteOperation {
                    operation_tag: OperationTag,
                    arguments: Box<dyn Any + Send + Sync>,
                    queue_stack: EffQueueStack<TestableCounterEffect>,
                },
                ApplyContinuation {
                    value: Box<dyn Any>,
                    queue_stack: EffQueueStack<TestableCounterEffect>,
                },
            }

            let mut loop_state = match computation.inner {
                EffInner::Pure(a) => return a,
                EffInner::Impure(operation) => LoopState::ExecuteOperation {
                    operation_tag: operation.operation_tag,
                    arguments: operation.arguments,
                    queue_stack: EffQueueStack::new(operation.queue),
                },
            };

            loop {
                loop_state = match loop_state {
                    LoopState::ExecuteOperation {
                        operation_tag,
                        arguments,
                        queue_stack,
                    } => {
                        let result: Box<dyn Any> = if operation_tag
                            == *__testable_counter_operations::INCREMENT_TAG
                        {
                            *self.counter.borrow_mut() += 1;
                            Box::new(())
                        } else if operation_tag == *__testable_counter_operations::DECREMENT_TAG {
                            *self.counter.borrow_mut() -= 1;
                            Box::new(())
                        } else if operation_tag == *__testable_counter_operations::GET_VALUE_TAG {
                            let value = *self.counter.borrow();
                            Box::new(value)
                        } else if operation_tag == *__testable_counter_operations::RESET_TAG {
                            let new_value = *arguments.downcast::<(i32,)>().unwrap();
                            *self.counter.borrow_mut() = new_value.0;
                            Box::new(())
                        } else {
                            panic!("Unknown TestableCounter operation");
                        };
                        LoopState::ApplyContinuation {
                            value: result,
                            queue_stack,
                        }
                    }
                    LoopState::ApplyContinuation {
                        value,
                        mut queue_stack,
                    } => match queue_stack.pop() {
                        None => return *value.downcast::<A>().expect("Final result type mismatch"),
                        Some(arrow) => match arrow.apply(value).inner {
                            EffInner::Pure(boxed) => LoopState::ApplyContinuation {
                                value: boxed,
                                queue_stack,
                            },
                            EffInner::Impure(operation) => {
                                queue_stack.push_queue(operation.queue);
                                LoopState::ExecuteOperation {
                                    operation_tag: operation.operation_tag,
                                    arguments: operation.arguments,
                                    queue_stack,
                                }
                            }
                        },
                    },
                };
            }
        }

        fn get_final_value(&self) -> i32 {
            *self.counter.borrow()
        }
    }

    #[rstest]
    fn defined_effect_can_be_handled() {
        let handler = TestableCounterHandlerImpl::new(0);
        let computation = TestableCounterEffect::get_value();
        let result = handler.run(computation);
        assert_eq!(result, 0);
    }

    #[rstest]
    fn defined_effect_increment_works() {
        let handler = TestableCounterHandlerImpl::new(0);
        let computation =
            TestableCounterEffect::increment().then(TestableCounterEffect::get_value());
        let result = handler.run(computation);
        assert_eq!(result, 1);
    }

    #[rstest]
    fn defined_effect_decrement_works() {
        let handler = TestableCounterHandlerImpl::new(10);
        let computation =
            TestableCounterEffect::decrement().then(TestableCounterEffect::get_value());
        let result = handler.run(computation);
        assert_eq!(result, 9);
    }

    #[rstest]
    fn defined_effect_reset_works() {
        let handler = TestableCounterHandlerImpl::new(0);
        let computation =
            TestableCounterEffect::reset(100).then(TestableCounterEffect::get_value());
        let result = handler.run(computation);
        assert_eq!(result, 100);
    }

    #[rstest]
    fn defined_effect_chained_operations_work() {
        let handler = TestableCounterHandlerImpl::new(0);
        let computation = TestableCounterEffect::increment()
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::decrement())
            .then(TestableCounterEffect::get_value());
        let result = handler.run(computation);
        assert_eq!(result, 2); // 0 + 1 + 1 + 1 - 1 = 2
    }

    #[rstest]
    fn defined_effect_with_flat_map() {
        let handler = TestableCounterHandlerImpl::new(5);
        let computation = TestableCounterEffect::get_value().flat_map(|current| {
            TestableCounterEffect::reset(current * 2).then(TestableCounterEffect::get_value())
        });
        let result = handler.run(computation);
        assert_eq!(result, 10); // 5 * 2 = 10
    }

    #[rstest]
    fn defined_effect_with_fmap() {
        let handler = TestableCounterHandlerImpl::new(42);
        let computation = TestableCounterEffect::get_value().fmap(|x| x * 2);
        let result = handler.run(computation);
        assert_eq!(result, 84);
    }

    #[rstest]
    fn defined_effect_preserves_final_state() {
        let handler = TestableCounterHandlerImpl::new(0);
        let computation = TestableCounterEffect::increment()
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::increment());
        handler.run(computation);
        assert_eq!(handler.get_final_value(), 3);
    }

    #[rstest]
    fn defined_effect_complex_scenario() {
        let handler = TestableCounterHandlerImpl::new(0);

        // Scenario: increment 5 times, get value, double it via reset, get final value
        let computation = TestableCounterEffect::increment()
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::increment())
            .then(TestableCounterEffect::get_value())
            .flat_map(|count| {
                TestableCounterEffect::reset(count * 10).then(TestableCounterEffect::get_value())
            });

        let result = handler.run(computation);
        assert_eq!(result, 50); // 5 * 10 = 50
        assert_eq!(handler.get_final_value(), 50);
    }

    // define_effect! Macro - Doc Comment Preservation Tests

    define_effect! {
        /// This is the outer doc comment for the effect.
        effect Documented {
            /// This is the doc comment for operation_one.
            fn operation_one() -> i32;
            /// This is the doc comment for operation_two.
            /// It has multiple lines.
            fn operation_two(input: String) -> String;
        }
    }

    #[rstest]
    fn define_effect_preserves_effect_name_with_docs() {
        assert_eq!(DocumentedEffect::NAME, "Documented");
    }

    #[rstest]
    fn define_effect_operation_methods_exist_with_docs() {
        let _computation1 = DocumentedEffect::operation_one();
        let _computation2 = DocumentedEffect::operation_two("test".to_string());
    }
}
