//! Tests for Monad Transformer Laws.
//!
//! This module tests that all monad transformers satisfy the fundamental laws:
//!
//! 1. **Left Identity**: `pure(a) >>= f === f(a)`
//! 2. **Right Identity**: `m >>= pure === m`
//! 3. **Associativity**: `(m >>= f) >>= g === m >>= (|x| f(x) >>= g)`
//!
//! Additionally, we test lift laws:
//! - **Lift Pure**: `lift(pure(a)) === pure(a)`
//! - **Lift Bind**: `lift(m >>= f) === lift(m) >>= (|x| lift(f(x)))`

use functional_rusty::effect::{ExceptT, ReaderT, StateT, WriterT};
use functional_rusty::typeclass::Monoid;
use rstest::rstest;

// =============================================================================
// ReaderT Monad Laws
// =============================================================================

mod reader_transformer_laws {
    use super::*;

    #[rstest]
    fn left_identity_option() {
        // pure(a) >>= f === f(a)
        let value = 5;
        let f = |x: i32| -> ReaderT<i32, Option<i32>> {
            ReaderT::new(move |environment: i32| Some(x + environment))
        };

        let left: ReaderT<i32, Option<i32>> = ReaderT::pure_option(value).flat_map_option(f);
        let right = f(value);

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn right_identity_option() {
        // m >>= pure === m
        let m: ReaderT<i32, Option<i32>> = ReaderT::new(|environment: i32| Some(environment * 2));

        let left = m.clone().flat_map_option(|x| ReaderT::pure_option(x));
        let right = m;

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn associativity_option() {
        // (m >>= f) >>= g === m >>= (|x| f(x) >>= g)
        let m: ReaderT<i32, Option<i32>> = ReaderT::new(|environment: i32| Some(environment));
        let f = |x: i32| -> ReaderT<i32, Option<i32>> {
            ReaderT::new(move |environment: i32| Some(x + environment))
        };
        let g = |x: i32| -> ReaderT<i32, Option<i32>> {
            ReaderT::new(move |_environment: i32| Some(x * 2))
        };

        let left = m.clone().flat_map_option(f).flat_map_option(g);
        let right = m.flat_map_option(move |x| f(x).flat_map_option(g));

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn left_identity_result() {
        let value = 5;
        let f = |x: i32| -> ReaderT<i32, Result<i32, String>> {
            ReaderT::new(move |environment: i32| Ok(x + environment))
        };

        let left: ReaderT<i32, Result<i32, String>> =
            ReaderT::pure_result(value).flat_map_result(f);
        let right = f(value);

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn right_identity_result() {
        let m: ReaderT<i32, Result<i32, String>> =
            ReaderT::new(|environment: i32| Ok(environment * 2));

        let left = m.clone().flat_map_result(|x| ReaderT::pure_result(x));
        let right = m;

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn associativity_result() {
        let m: ReaderT<i32, Result<i32, String>> = ReaderT::new(|environment: i32| Ok(environment));
        let f = |x: i32| -> ReaderT<i32, Result<i32, String>> {
            ReaderT::new(move |environment: i32| Ok(x + environment))
        };
        let g = |x: i32| -> ReaderT<i32, Result<i32, String>> {
            ReaderT::new(move |_environment: i32| Ok(x * 2))
        };

        let left = m.clone().flat_map_result(f).flat_map_result(g);
        let right = m.flat_map_result(move |x| f(x).flat_map_result(g));

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }
}

// =============================================================================
// StateT Monad Laws
// =============================================================================

mod state_transformer_laws {
    use super::*;

    #[rstest]
    fn left_identity_option() {
        // pure(a) >>= f === f(a)
        let value = 5;
        let f = |x: i32| -> StateT<i32, Option<(i32, i32)>> {
            StateT::new(move |state: i32| Some((x + state, state + 1)))
        };

        let left: StateT<i32, Option<(i32, i32)>> = StateT::pure_option(value).flat_map_option(f);
        let right = f(value);

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn right_identity_option() {
        // m >>= pure === m
        let m: StateT<i32, Option<(i32, i32)>> =
            StateT::new(|state: i32| Some((state * 2, state + 1)));

        let left = m.clone().flat_map_option(|x| StateT::pure_option(x));
        let right = m;

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn associativity_option() {
        // (m >>= f) >>= g === m >>= (|x| f(x) >>= g)
        let m: StateT<i32, Option<(i32, i32)>> = StateT::new(|state: i32| Some((state, state + 1)));
        let f = |x: i32| -> StateT<i32, Option<(i32, i32)>> {
            StateT::new(move |state: i32| Some((x + state, state + 1)))
        };
        let g = |x: i32| -> StateT<i32, Option<(i32, i32)>> {
            StateT::new(move |state: i32| Some((x * 2, state)))
        };

        let left = m.clone().flat_map_option(f).flat_map_option(g);
        let right = m.flat_map_option(move |x| f(x).flat_map_option(g));

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn left_identity_result() {
        let value = 5;
        let f = |x: i32| -> StateT<i32, Result<(i32, i32), String>> {
            StateT::new(move |state: i32| Ok((x + state, state + 1)))
        };

        let left: StateT<i32, Result<(i32, i32), String>> =
            StateT::pure_result(value).flat_map_result(f);
        let right = f(value);

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn right_identity_result() {
        let m: StateT<i32, Result<(i32, i32), String>> =
            StateT::new(|state: i32| Ok((state * 2, state + 1)));

        let left = m.clone().flat_map_result(|x| StateT::pure_result(x));
        let right = m;

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn associativity_result() {
        let m: StateT<i32, Result<(i32, i32), String>> =
            StateT::new(|state: i32| Ok((state, state + 1)));
        let f = |x: i32| -> StateT<i32, Result<(i32, i32), String>> {
            StateT::new(move |state: i32| Ok((x + state, state + 1)))
        };
        let g = |x: i32| -> StateT<i32, Result<(i32, i32), String>> {
            StateT::new(move |state: i32| Ok((x * 2, state)))
        };

        let left = m.clone().flat_map_result(f).flat_map_result(g);
        let right = m.flat_map_result(move |x| f(x).flat_map_result(g));

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }
}

// =============================================================================
// WriterT Monad Laws
// =============================================================================

mod writer_transformer_laws {
    use super::*;

    #[rstest]
    fn left_identity_option() {
        // pure(a) >>= f === f(a)
        let value = 5;
        let f = |x: i32| -> WriterT<Vec<String>, Option<(i32, Vec<String>)>> {
            WriterT::new(Some((x * 2, vec!["doubled".to_string()])))
        };

        let left: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::pure_option(value).flat_map_option(f);
        let right = f(value);

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn right_identity_option() {
        // m >>= pure === m
        let m: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((42, vec!["log".to_string()])));

        let left = m.clone().flat_map_option(|x| WriterT::pure_option(x));
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn associativity_option() {
        // (m >>= f) >>= g === m >>= (|x| f(x) >>= g)
        let m: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((10, vec!["start".to_string()])));
        let f = |x: i32| -> WriterT<Vec<String>, Option<(i32, Vec<String>)>> {
            WriterT::new(Some((x + 5, vec!["added".to_string()])))
        };
        let g = |x: i32| -> WriterT<Vec<String>, Option<(i32, Vec<String>)>> {
            WriterT::new(Some((x * 2, vec!["doubled".to_string()])))
        };

        let left = m.clone().flat_map_option(f).flat_map_option(g);
        let right = m.flat_map_option(move |x| f(x).flat_map_option(g));

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn left_identity_result() {
        let value = 5;
        let f = |x: i32| -> WriterT<Vec<String>, Result<(i32, Vec<String>), String>> {
            WriterT::new(Ok((x * 2, vec!["doubled".to_string()])))
        };

        let left: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
            WriterT::pure_result(value).flat_map_result(f);
        let right = f(value);

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn right_identity_result() {
        let m: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
            WriterT::new(Ok((42, vec!["log".to_string()])));

        let left = m.clone().flat_map_result(|x| WriterT::pure_result(x));
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn associativity_result() {
        let m: WriterT<Vec<String>, Result<(i32, Vec<String>), String>> =
            WriterT::new(Ok((10, vec!["start".to_string()])));
        let f = |x: i32| -> WriterT<Vec<String>, Result<(i32, Vec<String>), String>> {
            WriterT::new(Ok((x + 5, vec!["added".to_string()])))
        };
        let g = |x: i32| -> WriterT<Vec<String>, Result<(i32, Vec<String>), String>> {
            WriterT::new(Ok((x * 2, vec!["doubled".to_string()])))
        };

        let left = m.clone().flat_map_result(f).flat_map_result(g);
        let right = m.flat_map_result(move |x| f(x).flat_map_result(g));

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn writer_output_combines_correctly() {
        // Verify that outputs are combined using Monoid::combine
        let m: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((1, vec!["first".to_string()])));

        let chained = m
            .flat_map_option(|x| WriterT::new(Some((x + 1, vec!["second".to_string()]))))
            .flat_map_option(|x| WriterT::new(Some((x + 1, vec!["third".to_string()]))));

        let result = chained.run();
        assert_eq!(
            result,
            Some((
                3,
                vec![
                    "first".to_string(),
                    "second".to_string(),
                    "third".to_string()
                ]
            ))
        );
    }

    #[rstest]
    fn pure_produces_empty_output() {
        // pure should produce empty output (Monoid::empty)
        let pure_value: WriterT<Vec<String>, Option<(i32, Vec<String>)>> = WriterT::pure_option(42);

        let result = pure_value.run();
        assert_eq!(result, Some((42, Vec::<String>::empty())));
    }
}

// =============================================================================
// ExceptT Monad Laws
// =============================================================================

mod except_transformer_laws {
    use super::*;

    #[rstest]
    fn left_identity_option() {
        // pure(a) >>= f === f(a)
        let value = 5;
        let f = |x: i32| -> ExceptT<String, Option<Result<i32, String>>> {
            ExceptT::new(Some(Ok(x * 2)))
        };

        let left: ExceptT<String, Option<Result<i32, String>>> =
            ExceptT::pure_option(value).flat_map_option(f);
        let right = f(value);

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn right_identity_option() {
        // m >>= pure === m
        let m: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(42)));

        let left = m.clone().flat_map_option(|x| ExceptT::pure_option(x));
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn associativity_option() {
        // (m >>= f) >>= g === m >>= (|x| f(x) >>= g)
        let m: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(10)));
        let f = |x: i32| -> ExceptT<String, Option<Result<i32, String>>> {
            ExceptT::new(Some(Ok(x + 5)))
        };
        let g = |x: i32| -> ExceptT<String, Option<Result<i32, String>>> {
            ExceptT::new(Some(Ok(x * 2)))
        };

        let left = m.clone().flat_map_option(f).flat_map_option(g);
        let right = m.flat_map_option(move |x| f(x).flat_map_option(g));

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn left_identity_result() {
        let value = 5;
        let f = |x: i32| -> ExceptT<String, Result<Result<i32, String>, String>> {
            ExceptT::new(Ok(Ok(x * 2)))
        };

        let left: ExceptT<String, Result<Result<i32, String>, String>> =
            ExceptT::pure_result(value).flat_map_result(f);
        let right = f(value);

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn right_identity_result() {
        let m: ExceptT<String, Result<Result<i32, String>, String>> = ExceptT::new(Ok(Ok(42)));

        let left = m.clone().flat_map_result(|x| ExceptT::pure_result(x));
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn associativity_result() {
        let m: ExceptT<String, Result<Result<i32, String>, String>> = ExceptT::new(Ok(Ok(10)));
        let f = |x: i32| -> ExceptT<String, Result<Result<i32, String>, String>> {
            ExceptT::new(Ok(Ok(x + 5)))
        };
        let g = |x: i32| -> ExceptT<String, Result<Result<i32, String>, String>> {
            ExceptT::new(Ok(Ok(x * 2)))
        };

        let left = m.clone().flat_map_result(f).flat_map_result(g);
        let right = m.flat_map_result(move |x| f(x).flat_map_result(g));

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn error_short_circuits_left() {
        // throw(e) >>= f === throw(e)
        let error: ExceptT<String, Option<Result<i32, String>>> =
            ExceptT::<String, Option<Result<i32, String>>>::throw_option("error".to_string());

        let left = error
            .clone()
            .flat_map_option(|x| ExceptT::pure_option(x * 2));
        let right = error;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn catch_recovers_from_error() {
        // catch(throw(e), handler) === handler(e)
        let error: ExceptT<String, Option<Result<i32, String>>> =
            ExceptT::<String, Option<Result<i32, String>>>::throw_option("error".to_string());
        let handler = |e: String| -> ExceptT<String, Option<Result<i32, String>>> {
            ExceptT::pure_option(e.len() as i32)
        };

        let left = ExceptT::catch_option(error, handler);
        let right = handler("error".to_string());

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn catch_preserves_success() {
        // catch(pure(a), handler) === pure(a)
        let success: ExceptT<String, Option<Result<i32, String>>> = ExceptT::pure_option(42);
        let handler = |e: String| -> ExceptT<String, Option<Result<i32, String>>> {
            ExceptT::pure_option(e.len() as i32)
        };

        let left = ExceptT::catch_option(success.clone(), handler);
        let right = success;

        assert_eq!(left.run(), right.run());
    }
}

// =============================================================================
// Functor Laws for Transformers
// =============================================================================

mod functor_laws {
    use super::*;

    // Functor Law 1: fmap(id) === id
    // Functor Law 2: fmap(f . g) === fmap(f) . fmap(g)

    #[rstest]
    fn reader_transformer_functor_identity_option() {
        let m: ReaderT<i32, Option<i32>> = ReaderT::new(|environment: i32| Some(environment * 2));
        let identity = |x: i32| x;

        let left = m.clone().fmap_option(identity);
        let right = m;

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn reader_transformer_functor_composition_option() {
        let m: ReaderT<i32, Option<i32>> = ReaderT::new(|environment: i32| Some(environment));
        let f = |x: i32| x + 1;
        let g = |x: i32| x * 2;

        let left = m.clone().fmap_option(move |x| g(f(x)));
        let right = m.fmap_option(f).fmap_option(g);

        let environment = 10;
        assert_eq!(left.run(environment), right.run(environment));
    }

    #[rstest]
    fn state_transformer_functor_identity_option() {
        let m: StateT<i32, Option<(i32, i32)>> =
            StateT::new(|state: i32| Some((state * 2, state + 1)));
        let identity = |x: i32| x;

        let left = m.clone().fmap_option(identity);
        let right = m;

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn state_transformer_functor_composition_option() {
        let m: StateT<i32, Option<(i32, i32)>> = StateT::new(|state: i32| Some((state, state + 1)));
        let f = |x: i32| x + 1;
        let g = |x: i32| x * 2;

        let left = m.clone().fmap_option(move |x| g(f(x)));
        let right = m.fmap_option(f).fmap_option(g);

        let initial_state = 10;
        assert_eq!(left.run(initial_state), right.run(initial_state));
    }

    #[rstest]
    fn writer_transformer_functor_identity_option() {
        let m: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((42, vec!["log".to_string()])));
        let identity = |x: i32| x;

        let left = m.clone().fmap_option(identity);
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn writer_transformer_functor_composition_option() {
        let m: WriterT<Vec<String>, Option<(i32, Vec<String>)>> =
            WriterT::new(Some((10, vec!["log".to_string()])));
        let f = |x: i32| x + 1;
        let g = |x: i32| x * 2;

        let left = m.clone().fmap_option(move |x| g(f(x)));
        let right = m.fmap_option(f).fmap_option(g);

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn except_transformer_functor_identity_option() {
        let m: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(42)));
        let identity = |x: i32| x;

        let left = m.clone().fmap_option(identity);
        let right = m;

        assert_eq!(left.run(), right.run());
    }

    #[rstest]
    fn except_transformer_functor_composition_option() {
        let m: ExceptT<String, Option<Result<i32, String>>> = ExceptT::new(Some(Ok(10)));
        let f = |x: i32| x + 1;
        let g = |x: i32| x * 2;

        let left = m.clone().fmap_option(move |x| g(f(x)));
        let right = m.fmap_option(f).fmap_option(g);

        assert_eq!(left.run(), right.run());
    }
}
