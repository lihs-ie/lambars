#![cfg(feature = "effect")]

use lambars::effect::RWS;
use rstest::rstest;

mod basic_structure {
    use super::*;

    #[rstest]
    fn rws_new_and_run_basic() {
        let rws: RWS<i32, Vec<String>, i32, i32> = RWS::new(|environment, state| {
            let result = environment + state;
            let new_state = state + 1;
            let output = vec![format!("computed: {}", result)];
            (result, new_state, output)
        });
        let (result, final_state, output) = rws.run(10, 5);
        assert_eq!(result, 15);
        assert_eq!(final_state, 6);
        assert_eq!(output, vec!["computed: 15"]);
    }

    #[rstest]
    fn rws_pure_creates_constant() {
        let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(42);
        let (result, final_state, output) = rws.run(0, 0);
        assert_eq!(result, 42);
        assert_eq!(final_state, 0);
        assert!(output.is_empty());
    }

    #[rstest]
    fn rws_eval_returns_result_and_output() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
        let (result, output) = rws.eval(0, 0);
        assert_eq!(result, 42);
        assert_eq!(output, String::new());
    }

    #[rstest]
    fn rws_exec_returns_state_and_output() {
        let rws: RWS<i32, String, i32, ()> =
            RWS::new(|_, state| ((), state + 1, "incremented".to_string()));
        let (final_state, output) = rws.exec(0, 10);
        assert_eq!(final_state, 11);
        assert_eq!(output, "incremented");
    }
}

mod functor_monad {
    use super::*;

    #[rstest]
    fn rws_fmap_transforms_result() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(21);
        let mapped = rws.fmap(|x| x * 2);
        let (result, _, _) = mapped.run(0, 0);
        assert_eq!(result, 42);
    }

    #[rstest]
    fn rws_fmap_preserves_state_and_output() {
        let rws: RWS<i32, Vec<String>, i32, i32> =
            RWS::new(|_, state| (10, state + 1, vec!["log".to_string()]));
        let mapped = rws.fmap(|x| x * 2);
        let (result, final_state, output) = mapped.run(0, 5);
        assert_eq!(result, 20);
        assert_eq!(final_state, 6);
        assert_eq!(output, vec!["log"]);
    }

    #[rstest]
    fn rws_flat_map_chains_rws() {
        let rws1: RWS<i32, Vec<String>, i32, i32> =
            RWS::new(|environment, state| (environment, state, vec!["first".to_string()]));
        let rws2 = rws1.flat_map(|x| {
            RWS::new(move |_, state| (x + state, state + 1, vec!["second".to_string()]))
        });
        let (result, final_state, output) = rws2.run(10, 5);
        assert_eq!(result, 15); // 10 + 5
        assert_eq!(final_state, 6); // 5 + 1
        assert_eq!(output, vec!["first", "second"]);
    }

    #[rstest]
    fn rws_and_then_is_alias_for_flat_map() {
        let rws1: RWS<i32, Vec<String>, i32, i32> = RWS::pure(10);
        let rws2 = rws1.and_then(|x| RWS::pure(x * 2));
        let (result, _, _) = rws2.run(0, 0);
        assert_eq!(result, 20);
    }

    #[rstest]
    fn rws_then_sequences() {
        let log1: RWS<(), Vec<String>, (), ()> =
            RWS::new(|_, _| ((), (), vec!["step 1".to_string()]));
        let log2: RWS<(), Vec<String>, (), i32> =
            RWS::new(|_, _| (42, (), vec!["step 2".to_string()]));
        let combined = log1.then(log2);
        let (result, _, output) = combined.run((), ());
        assert_eq!(result, 42);
        assert_eq!(output, vec!["step 1", "step 2"]);
    }

    #[rstest]
    fn rws_map2_combines() {
        let rws1: RWS<i32, String, i32, i32> = RWS::pure(10);
        let rws2: RWS<i32, String, i32, i32> = RWS::pure(20);
        let combined = rws1.map2(rws2, |a, b| a + b);
        let (result, _, _) = combined.run(0, 0);
        assert_eq!(result, 30);
    }

    #[rstest]
    fn rws_map2_combines_outputs() {
        let rws1: RWS<i32, Vec<String>, i32, i32> =
            RWS::new(|_, state| (10, state, vec!["first".to_string()]));
        let rws2: RWS<i32, Vec<String>, i32, i32> =
            RWS::new(|_, state| (20, state + 1, vec!["second".to_string()]));
        let combined = rws1.map2(rws2, |a, b| a + b);
        let (result, final_state, output) = combined.run(0, 5);
        assert_eq!(result, 30);
        assert_eq!(final_state, 6);
        assert_eq!(output, vec!["first", "second"]);
    }

    #[rstest]
    fn rws_product_creates_tuple() {
        let rws1: RWS<i32, String, i32, i32> = RWS::pure(42);
        let rws2: RWS<i32, String, i32, &str> = RWS::pure("hello");
        let product = rws1.product(rws2);
        let ((first, second), _, _) = product.run(0, 0);
        assert_eq!(first, 42);
        assert_eq!(second, "hello");
    }

    #[rstest]
    fn rws_apply_applies_function() {
        let function_rws: RWS<i32, String, i32, fn(i32) -> i32> = RWS::pure(|x| x * 2);
        let value_rws: RWS<i32, String, i32, i32> = RWS::pure(21);
        let applied = function_rws.apply(value_rws);
        let (result, _, _) = applied.run(0, 0);
        assert_eq!(result, 42);
    }
}

mod monad_reader {
    use super::*;

    #[rstest]
    fn rws_ask_returns_environment() {
        let rws: RWS<i32, String, (), i32> = RWS::ask();
        let (result, _, _) = rws.run(42, ());
        assert_eq!(result, 42);
    }

    #[rstest]
    fn rws_ask_does_not_modify_state_or_output() {
        let rws: RWS<i32, String, i32, i32> = RWS::ask();
        let (result, final_state, output) = rws.run(42, 10);
        assert_eq!(result, 42);
        assert_eq!(final_state, 10);
        assert_eq!(output, String::new());
    }

    #[rstest]
    fn rws_asks_projects_environment() {
        #[derive(Clone)]
        struct Config {
            port: u16,
        }
        let rws: RWS<Config, String, (), u16> = RWS::asks(|c: Config| c.port);
        let (result, _, _) = rws.run(Config { port: 8080 }, ());
        assert_eq!(result, 8080);
    }

    #[rstest]
    fn rws_local_modifies_environment() {
        let rws: RWS<i32, String, (), i32> = RWS::ask();
        let modified = RWS::local(|environment| environment * 2, rws);
        let (result, _, _) = modified.run(21, ());
        assert_eq!(result, 42);
    }

    #[rstest]
    fn rws_local_does_not_affect_outer_environment() {
        let inner: RWS<i32, String, (), i32> = RWS::ask();
        let local_computation = RWS::local(|environment| environment * 2, inner);
        let outer: RWS<i32, String, (), i32> = RWS::ask();

        let (local_result, _, _) = local_computation.run(21, ());
        let (outer_result, _, _) = outer.run(21, ());

        assert_eq!(local_result, 42);
        assert_eq!(outer_result, 21);
    }
}

mod monad_writer {
    use super::*;

    #[rstest]
    fn rws_tell_appends_output() {
        let rws: RWS<(), Vec<String>, (), ()> = RWS::tell(vec!["log message".to_string()]);
        let (_, _, output) = rws.run((), ());
        assert_eq!(output, vec!["log message"]);
    }

    #[rstest]
    fn rws_tell_chained() {
        let rws1: RWS<(), Vec<String>, (), ()> = RWS::tell(vec!["first".to_string()]);
        let rws2: RWS<(), Vec<String>, (), ()> = RWS::tell(vec!["second".to_string()]);
        let combined = rws1.then(rws2);
        let (_, _, output) = combined.run((), ());
        assert_eq!(output, vec!["first", "second"]);
    }

    #[rstest]
    fn rws_listen_captures_output() {
        let rws: RWS<(), Vec<String>, (), i32> =
            RWS::new(|_, _| (42, (), vec!["computed".to_string()]));
        let listened = RWS::listen(rws);
        let ((result, captured_output), _, total_output) = listened.run((), ());
        assert_eq!(result, 42);
        assert_eq!(captured_output, vec!["computed"]);
        assert_eq!(total_output, vec!["computed"]);
    }

    #[rstest]
    #[allow(clippy::type_complexity)]
    fn rws_pass_modifies_output() {
        let rws: RWS<(), Vec<String>, (), (i32, fn(Vec<String>) -> Vec<String>)> =
            RWS::new(|_, _| {
                let modifier: fn(Vec<String>) -> Vec<String> =
                    |output| output.into_iter().map(|s| s.to_uppercase()).collect();
                ((42, modifier), (), vec!["hello".to_string()])
            });
        let passed = RWS::pass(rws);
        let (result, _, output) = passed.run((), ());
        assert_eq!(result, 42);
        assert_eq!(output, vec!["HELLO"]);
    }

    #[rstest]
    fn rws_censor_modifies_output() {
        let rws: RWS<(), Vec<String>, (), i32> =
            RWS::new(|_, _| (42, (), vec!["hello".to_string()]));
        let censored = RWS::censor(
            |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
            rws,
        );
        let (result, _, output) = censored.run((), ());
        assert_eq!(result, 42);
        assert_eq!(output, vec!["HELLO"]);
    }

    #[rstest]
    fn rws_listens_projects_output() {
        let rws: RWS<(), Vec<String>, (), i32> =
            RWS::new(|_, _| (42, (), vec!["a".to_string(), "b".to_string()]));
        let listened = RWS::listens(|output: &Vec<String>| output.len(), rws);
        let ((result, count), _, output) = listened.run((), ());
        assert_eq!(result, 42);
        assert_eq!(count, 2);
        assert_eq!(output, vec!["a", "b"]);
    }
}

mod monad_state {
    use super::*;

    #[rstest]
    fn rws_get_returns_state() {
        let rws: RWS<(), String, i32, i32> = RWS::get();
        let (result, final_state, _) = rws.run((), 42);
        assert_eq!(result, 42);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn rws_put_sets_state() {
        let rws: RWS<(), String, i32, ()> = RWS::put(100);
        let (_, final_state, _) = rws.run((), 42);
        assert_eq!(final_state, 100);
    }

    #[rstest]
    fn rws_state_transitions() {
        let rws: RWS<(), String, i32, String> = RWS::state(|s| (format!("was: {}", s), s + 1));
        let (result, final_state, _) = rws.run((), 41);
        assert_eq!(result, "was: 41");
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn rws_modify_transforms_state() {
        let rws: RWS<(), String, i32, ()> = RWS::modify(|x| x * 2);
        let (_, final_state, _) = rws.run((), 21);
        assert_eq!(final_state, 42);
    }

    #[rstest]
    fn rws_gets_projects_state() {
        #[derive(Clone)]
        struct AppState {
            counter: i32,
        }
        let rws: RWS<(), String, AppState, i32> = RWS::gets(|s: &AppState| s.counter);
        let (result, _, _) = rws.run((), AppState { counter: 42 });
        assert_eq!(result, 42);
    }

    #[rstest]
    fn rws_state_operations_chain() {
        let rws: RWS<(), String, i32, i32> =
            RWS::get().flat_map(|current| RWS::put(current + 10).then(RWS::get()));
        let (result, final_state, _) = rws.run((), 5);
        assert_eq!(result, 15);
        assert_eq!(final_state, 15);
    }
}

mod utilities {
    use super::*;

    #[rstest]
    fn rws_map_rws_transforms_all() {
        let rws: RWS<i32, String, i32, i32> =
            RWS::new(|environment, state| (environment + state, state, "log".to_string()));
        let mapped = rws.map_rws(|(result, state, output): (i32, i32, String)| {
            (result * 2, state + 1, output.to_uppercase())
        });
        let (result, final_state, output) = mapped.run(10, 5);
        assert_eq!(result, 30); // (10 + 5) * 2
        assert_eq!(final_state, 6); // 5 + 1
        assert_eq!(output, "LOG");
    }

    #[rstest]
    fn rws_with_rws_transforms_input() {
        let rws: RWS<i32, String, i32, i32> = RWS::ask();
        let with_transformed =
            rws.with_rws(|environment: String, state| (environment.len() as i32, state));
        let (result, _, _) = with_transformed.run("hello".to_string(), 0);
        assert_eq!(result, 5);
    }
}

mod standard_traits {
    use super::*;

    #[rstest]
    fn rws_clone_works() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
        let cloned = rws.clone();
        let (result1, _, _) = rws.run(0, 0);
        let (result2, _, _) = cloned.run(0, 0);
        assert_eq!(result1, result2);
    }

    #[rstest]
    fn rws_display_shows_type() {
        let rws: RWS<i32, String, i32, i32> = RWS::pure(42);
        assert_eq!(format!("{}", rws), "<RWS>");
    }
}

mod integration {
    use super::*;

    #[rstest]
    fn rws_combined_reader_writer_state() {
        #[derive(Clone)]
        struct Config {
            multiplier: i32,
        }

        let computation: RWS<Config, Vec<String>, i32, i32> =
            RWS::ask().flat_map(|config: Config| {
                RWS::get().flat_map(move |state| {
                    let result = state * config.multiplier;
                    RWS::put(state + 1).then(
                        RWS::tell(vec![format!(
                            "multiplied {} by {} = {}",
                            state, config.multiplier, result
                        )])
                        .then(RWS::pure(result)),
                    )
                })
            });

        let (result, final_state, output) = computation.run(Config { multiplier: 3 }, 10);
        assert_eq!(result, 30);
        assert_eq!(final_state, 11);
        assert_eq!(output, vec!["multiplied 10 by 3 = 30"]);
    }

    #[rstest]
    fn rws_multiple_operations() {
        let computation: RWS<i32, Vec<String>, i32, i32> = RWS::tell(vec!["start".to_string()])
            .then(RWS::ask())
            .flat_map(|environment| {
                RWS::get().flat_map(move |state| {
                    let sum = environment + state;
                    RWS::put(sum)
                        .then(RWS::tell(vec![format!("computed sum: {}", sum)]))
                        .then(RWS::pure(sum))
                })
            });

        let (result, final_state, output) = computation.run(100, 50);
        assert_eq!(result, 150);
        assert_eq!(final_state, 150);
        assert_eq!(output, vec!["start", "computed sum: 150"]);
    }

    #[rstest]
    fn rws_local_with_state_and_writer() {
        let inner: RWS<i32, Vec<String>, i32, i32> = RWS::ask().flat_map(|environment| {
            RWS::get().flat_map(move |state| {
                RWS::tell(vec![format!("env={}, state={}", environment, state)])
                    .then(RWS::pure(environment + state))
            })
        });

        let modified = RWS::local(|environment| environment * 2, inner);
        let (result, _, output) = modified.run(10, 5);
        assert_eq!(result, 25); // (10 * 2) + 5
        assert_eq!(output, vec!["env=20, state=5"]);
    }
}
