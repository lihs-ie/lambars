#![cfg(feature = "typeclass")]

use lambars::effect::RWS;
use lambars::typeclass::Semigroup;
use proptest::prelude::*;

mod functor_laws {
    use super::*;

    proptest! {
        /// Functor Identity: fmap(id) == id
        #[test]
        fn identity(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(value);
            let mapped = rws.clone().fmap(|x| x);
            let (result1, state1, output1) = rws.run(env, state);
            let (result2, state2, output2) = mapped.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Functor Composition: fmap(g . f) == fmap(g) . fmap(f)
        #[test]
        fn composition(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(value);
            let function1 = |x: i32| x.saturating_add(1);
            let function2 = |x: i32| x.saturating_mul(2);

            let composed = rws.clone().fmap(move |x| function2(function1(x)));
            let sequential = rws.fmap(function1).fmap(function2);

            let (result1, state1, output1) = composed.run(env, state);
            let (result2, state2, output2) = sequential.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }
    }
}

mod monad_laws {
    use super::*;

    proptest! {
        /// Left Identity: pure(a).flat_map(f) == f(a)
        #[test]
        fn left_identity(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let function = |x: i32| -> RWS<i32, Vec<String>, i32, i32> {
                RWS::new(move |_, current_state: i32| {
                    (x.saturating_mul(2), current_state.saturating_add(1), vec![format!("doubled: {}", x)])
                })
            };

            let left: RWS<i32, Vec<String>, i32, i32> = RWS::pure(value).flat_map(function);
            let right = function(value);

            let (result1, state1, output1) = left.run(env, state);
            let (result2, state2, output2) = right.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Right Identity: m.flat_map(pure) == m
        #[test]
        fn right_identity(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::new(move |environment, current_state| {
                (value.saturating_add(environment), current_state, vec!["original".to_string()])
            });

            let flatmapped = rws.clone().flat_map(RWS::pure);

            let (result1, state1, output1) = rws.run(env, state);
            let (result2, state2, output2) = flatmapped.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
        #[test]
        fn associativity(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(value);

            let function1 = |x: i32| -> RWS<i32, Vec<String>, i32, i32> {
                RWS::new(move |_, current_state: i32| (x.saturating_add(1), current_state, vec!["f".to_string()]))
            };
            let function2 = |x: i32| -> RWS<i32, Vec<String>, i32, i32> {
                RWS::new(move |_, current_state: i32| (x.saturating_mul(2), current_state, vec!["g".to_string()]))
            };

            let left = rws.clone().flat_map(function1).flat_map(function2);
            let right = rws.flat_map(move |x| function1(x).flat_map(function2));

            let (result1, state1, output1) = left.run(env, state);
            let (result2, state2, output2) = right.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }
    }
}

mod monad_reader_laws {
    use super::*;

    proptest! {
        /// Ask Law: ask returns the environment unmodified
        #[test]
        fn ask_returns_environment(env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::ask();
            let (result, _, _) = rws.run(env, state);
            prop_assert_eq!(result, env);
        }

        /// Local Identity: local(id, m) == m
        #[test]
        fn local_identity(value in any::<i32>(), env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::pure(value);
            let local_rws = RWS::local(|x| x, rws.clone());

            let (result1, state1, output1) = rws.run(env, state);
            let (result2, state2, output2) = local_rws.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Local Composition: local(f, local(g, m)) == local(g . f, m)
        #[test]
        fn local_composition(env in any::<i32>(), state in any::<i32>()) {
            let rws: RWS<i32, Vec<String>, i32, i32> = RWS::ask();
            let function1 = |x: i32| x.saturating_add(1);
            let function2 = |x: i32| x.saturating_mul(2);

            let nested = RWS::local(function1, RWS::local(function2, rws.clone()));
            let composed = RWS::local(move |x| function2(function1(x)), rws);

            let (result1, state1, output1) = nested.run(env, state);
            let (result2, state2, output2) = composed.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Asks is equivalent to ask().fmap(f)
        #[test]
        fn asks_equivalence(env in any::<i32>(), state in any::<i32>()) {
            let projection = |x: i32| x.saturating_mul(2);

            let asks_rws: RWS<i32, Vec<String>, i32, i32> = RWS::asks(projection);
            let ask_fmap_rws: RWS<i32, Vec<String>, i32, i32> = RWS::ask().fmap(projection);

            let (result1, state1, output1) = asks_rws.run(env, state);
            let (result2, state2, output2) = ask_fmap_rws.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }
    }
}

mod monad_writer_laws {
    use super::*;

    proptest! {
        /// Tell Monoid Law: tell(w1).then(tell(w2)) == tell(w1.combine(w2))
        #[test]
        fn tell_monoid_law(
            w1 in prop::collection::vec(any::<String>(), 0..5),
            w2 in prop::collection::vec(any::<String>(), 0..5),
            env in any::<i32>(),
            state in any::<i32>()
        ) {
            let sequential: RWS<i32, Vec<String>, i32, ()> =
                RWS::tell(w1.clone()).then(RWS::tell(w2.clone()));
            let combined: RWS<i32, Vec<String>, i32, ()> =
                RWS::tell(w1.combine(w2));

            let (_, _, output1) = sequential.run(env, state);
            let (_, _, output2) = combined.run(env, state);
            prop_assert_eq!(output1, output2);
        }

        /// Listen captures the output correctly
        #[test]
        fn listen_captures_output(
            value in any::<i32>(),
            log in prop::collection::vec(any::<String>(), 0..5),
            env in any::<i32>(),
            state in any::<i32>()
        ) {
            let rws: RWS<i32, Vec<String>, i32, i32> =
                RWS::new(move |_, current_state: i32| (value, current_state, log.clone()));

            let listened = RWS::listen(rws.clone());
            let ((result, captured_output), _, total_output) = listened.run(env, state);
            let (expected_result, _, expected_output) = rws.run(env, state);

            prop_assert_eq!(result, expected_result);
            prop_assert_eq!(captured_output, expected_output.clone());
            prop_assert_eq!(total_output, expected_output);
        }

        /// Censor modifies output with the function
        #[test]
        fn censor_modifies_output(
            value in any::<i32>(),
            env in any::<i32>(),
            state in any::<i32>()
        ) {
            let rws: RWS<i32, Vec<String>, i32, i32> =
                RWS::new(move |_, current_state: i32| (value, current_state, vec!["original".to_string()]));

            let censored = RWS::censor(
                |output: Vec<String>| output.into_iter().map(|s| s.to_uppercase()).collect(),
                rws.clone(),
            );

            let (result1, state1, output1) = rws.run(env, state);
            let (result2, state2, output2) = censored.run(env, state);

            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output2.clone(), vec!["ORIGINAL"]);
            prop_assert_ne!(output1, output2);
        }
    }
}

mod monad_state_laws {
    use super::*;

    proptest! {
        /// Get Put Law: get().flat_map(put) == pure(())
        #[test]
        fn get_put_law(env in any::<i32>(), state in any::<i32>()) {
            let get_put: RWS<i32, Vec<String>, i32, ()> =
                RWS::get().flat_map(RWS::put);
            let pure_unit: RWS<i32, Vec<String>, i32, ()> = RWS::pure(());

            let (result1, state1, output1) = get_put.run(env, state);
            let (result2, state2, output2) = pure_unit.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Put Get Law: put(s).then(get()) returns s
        #[test]
        fn put_get_law(new_state in any::<i32>(), env in any::<i32>(), initial_state in any::<i32>()) {
            let put_get: RWS<i32, Vec<String>, i32, i32> =
                RWS::put(new_state).then(RWS::get());

            let (result, final_state, _) = put_get.run(env, initial_state);
            prop_assert_eq!(result, new_state);
            prop_assert_eq!(final_state, new_state);
        }

        /// Put Put Law: put(s1).then(put(s2)) == put(s2)
        #[test]
        fn put_put_law(state1_val in any::<i32>(), state2_val in any::<i32>(), env in any::<i32>(), initial_state in any::<i32>()) {
            let put_put: RWS<i32, Vec<String>, i32, ()> =
                RWS::put(state1_val).then(RWS::put(state2_val));
            let just_put: RWS<i32, Vec<String>, i32, ()> = RWS::put(state2_val);

            let (result1, final_state1, output1) = put_put.run(env, initial_state);
            let (result2, final_state2, output2) = just_put.run(env, initial_state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(final_state1, final_state2);
            prop_assert_eq!(output1, output2);
        }

        /// Modify Composition: modify(f).then(modify(g)) == modify(|s| g(f(s)))
        #[test]
        fn modify_composition(env in any::<i32>(), state in any::<i32>()) {
            let function1 = |x: i32| x.saturating_add(1);
            let function2 = |x: i32| x.saturating_mul(2);

            let sequential: RWS<i32, Vec<String>, i32, ()> =
                RWS::modify(function1).then(RWS::modify(function2));
            let composed: RWS<i32, Vec<String>, i32, ()> =
                RWS::modify(move |s| function2(function1(s)));

            let (result1, state1, output1) = sequential.run(env, state);
            let (result2, state2, output2) = composed.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }

        /// Gets is equivalent to get().fmap(f)
        #[test]
        fn gets_equivalence(env in any::<i32>(), state in any::<i32>()) {
            let projection = |s: &i32| s.saturating_mul(2);

            let gets_rws: RWS<i32, Vec<String>, i32, i32> = RWS::gets(projection);
            let get_fmap_rws: RWS<i32, Vec<String>, i32, i32> = RWS::get().fmap(move |s| projection(&s));

            let (result1, state1, output1) = gets_rws.run(env, state);
            let (result2, state2, output2) = get_fmap_rws.run(env, state);
            prop_assert_eq!(result1, result2);
            prop_assert_eq!(state1, state2);
            prop_assert_eq!(output1, output2);
        }
    }
}

mod combined_effects_laws {
    use super::*;

    proptest! {
        /// Effects are independent: Reader does not affect Writer or State.
        #[test]
        fn reader_independence(env in any::<i32>(), state in any::<i32>()) {
            let ask_rws: RWS<i32, Vec<String>, i32, i32> = RWS::ask();
            let (result, final_state, output) = ask_rws.run(env, state);

            prop_assert_eq!(result, env);
            prop_assert_eq!(final_state, state);
            prop_assert!(output.is_empty());
        }

        /// State operations do not affect Reader or Writer (output).
        #[test]
        fn state_independence(new_state in any::<i32>(), env in any::<i32>(), initial_state in any::<i32>()) {
            let put_rws: RWS<i32, Vec<String>, i32, ()> = RWS::put(new_state);
            let (result, final_state, output) = put_rws.run(env, initial_state);

            prop_assert_eq!(result, ());
            prop_assert_eq!(final_state, new_state);
            prop_assert!(output.is_empty());
        }

        /// Writer operations do not affect Reader or State.
        #[test]
        fn writer_independence(
            log in prop::collection::vec(any::<String>(), 0..5),
            env in any::<i32>(),
            state in any::<i32>()
        ) {
            let tell_rws: RWS<i32, Vec<String>, i32, ()> = RWS::tell(log.clone());
            let (result, final_state, output) = tell_rws.run(env, state);

            prop_assert_eq!(result, ());
            prop_assert_eq!(final_state, state);
            prop_assert_eq!(output, log);
        }
    }
}
