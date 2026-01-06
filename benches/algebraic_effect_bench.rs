//! Benchmark for Algebraic Effects system.
//!
//! Measures the performance and usability of the algebraic effects system
//! including Eff operations, individual effects, effect composition, and
//! stack safety.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::EffectRow;
use lambars::effect::algebraic::{
    Eff, EffCons, EffNil, ErrorEffect, ErrorHandler, Handler, NoEffect, PureHandler, ReaderEffect,
    ReaderHandler, StateEffect, StateHandler, WriterEffect, WriterHandler, attempt, catch, listen,
    run_local,
};
use lambars::effect::{Reader, State, Writer};
use std::hint::black_box;

// =============================================================================
// 1. Eff Basic Operations
// =============================================================================

fn benchmark_eff_pure(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("eff_pure");

    group.bench_function("pure_i32", |bencher| {
        bencher.iter(|| {
            let computation: Eff<NoEffect, i32> = Eff::pure(black_box(42));
            let result = PureHandler.run(computation);
            black_box(result)
        });
    });

    group.bench_function("pure_string", |bencher| {
        bencher.iter(|| {
            let computation: Eff<NoEffect, String> = Eff::pure(black_box("hello".to_string()));
            let result = PureHandler.run(computation);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_eff_fmap_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("eff_fmap_chain");

    for chain_length in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut computation: Eff<NoEffect, i32> = Eff::pure(1);
                    for _ in 0..length {
                        computation = computation.fmap(|x| x + 1);
                    }
                    let result = PureHandler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_eff_flat_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("eff_flat_map_chain");

    for chain_length in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut computation: Eff<NoEffect, i32> = Eff::pure(1);
                    for _ in 0..length {
                        computation = computation.flat_map(|x| Eff::pure(x + 1));
                    }
                    let result = PureHandler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_eff_map2_product(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("eff_map2_product");

    group.bench_function("map2", |bencher| {
        bencher.iter(|| {
            let first: Eff<NoEffect, i32> = Eff::pure(black_box(10));
            let second: Eff<NoEffect, i32> = Eff::pure(black_box(20));
            let computation = first.map2(second, |a, b| a + b);
            let result = PureHandler.run(computation);
            black_box(result)
        });
    });

    group.bench_function("product", |bencher| {
        bencher.iter(|| {
            let first: Eff<NoEffect, i32> = Eff::pure(black_box(10));
            let second: Eff<NoEffect, i32> = Eff::pure(black_box(20));
            let computation = first.product(second);
            let result = PureHandler.run(computation);
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// 2. Reader Effect
// =============================================================================

fn benchmark_reader_effect(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_effect");

    group.bench_function("ask", |bencher| {
        bencher.iter(|| {
            let computation = ReaderEffect::<i32>::ask();
            let result = ReaderHandler::new(black_box(42)).run(computation);
            black_box(result)
        });
    });

    group.bench_function("asks", |bencher| {
        bencher.iter(|| {
            let computation = ReaderEffect::<String>::asks(|s: String| s.len());
            let result = ReaderHandler::new(black_box("hello".to_string())).run(computation);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_reader_effect_local(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_effect_local");

    for nest_depth in [1, 3, 5] {
        group.bench_with_input(
            BenchmarkId::from_parameter(nest_depth),
            &nest_depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    let handler = ReaderHandler::new(black_box(10));
                    let mut computation =
                        run_local(|x: i32| x * 2, ReaderEffect::<i32>::ask(), Eff::pure);
                    for _ in 1..depth {
                        let inner = computation;
                        computation = run_local(|x: i32| x + 1, inner, Eff::pure);
                    }
                    let result = handler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_reader_effect_flat_map_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_effect_flat_map_chain");

    for chain_length in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let handler = ReaderHandler::new(black_box(10));
                    let mut computation: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
                    for _ in 0..length {
                        computation =
                            computation.flat_map(|x| ReaderEffect::ask().fmap(move |y| x + y));
                    }
                    let result = handler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 3. State Effect
// =============================================================================

fn benchmark_state_effect(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_effect");

    group.bench_function("get", |bencher| {
        bencher.iter(|| {
            let computation = StateEffect::<i32>::get();
            let (result, final_state) = StateHandler::new(black_box(42)).run(computation);
            black_box((result, final_state))
        });
    });

    group.bench_function("put", |bencher| {
        bencher.iter(|| {
            let computation = StateEffect::put(black_box(100));
            let (result, final_state) = StateHandler::new(0).run(computation);
            black_box((result, final_state))
        });
    });

    group.finish();
}

fn benchmark_state_effect_modify(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_effect_modify");

    for modify_count in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(modify_count),
            &modify_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let handler = StateHandler::new(black_box(0));
                    let mut computation: Eff<StateEffect<i32>, ()> = Eff::pure(());
                    for _ in 0..count {
                        computation = computation.then(StateEffect::modify(|x: i32| x + 1));
                    }
                    let ((), final_state) = handler.run(computation);
                    black_box(final_state)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_state_effect_get_modify_get(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_effect_get_modify_get");

    for repeat_count in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::from_parameter(repeat_count),
            &repeat_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let handler = StateHandler::new(black_box(0));
                    let mut computation: Eff<StateEffect<i32>, i32> = StateEffect::get();
                    for _ in 0..count {
                        computation = computation.flat_map(|_| {
                            StateEffect::modify(|x: i32| x + 1).then(StateEffect::get())
                        });
                    }
                    let (result, final_state) = handler.run(computation);
                    black_box((result, final_state))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 4. Writer Effect
// =============================================================================

fn benchmark_writer_effect(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("writer_effect");

    group.bench_function("tell", |bencher| {
        bencher.iter(|| {
            let computation = WriterEffect::tell(black_box("log".to_string()));
            let ((), log) = WriterHandler::<String>::new().run(computation);
            black_box(log)
        });
    });

    group.bench_function("listen", |bencher| {
        bencher.iter(|| {
            let computation =
                listen(WriterEffect::tell("inner".to_string()).then(Eff::pure(black_box(42))));
            let ((result, inner_log), total_log) = WriterHandler::<String>::new().run(computation);
            black_box((result, inner_log, total_log))
        });
    });

    group.finish();
}

fn benchmark_writer_effect_tell_chain(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("writer_effect_tell_chain");

    for tell_count in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(tell_count),
            &tell_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let handler = WriterHandler::<Vec<i32>>::new();
                    let mut computation: Eff<WriterEffect<Vec<i32>>, ()> = Eff::pure(());
                    for index in 0..count {
                        let index_copy = index;
                        computation = computation.then(WriterEffect::tell(vec![index_copy]));
                    }
                    let ((), log) = handler.run(computation);
                    black_box(log)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 5. Error Effect
// =============================================================================

fn benchmark_error_effect(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("error_effect");

    group.bench_function("success_path", |bencher| {
        bencher.iter(|| {
            let computation: Eff<ErrorEffect<String>, i32> = Eff::pure(black_box(42));
            let result = ErrorHandler::<String>::new().run(computation);
            black_box(result)
        });
    });

    group.bench_function("throw", |bencher| {
        bencher.iter(|| {
            let computation: Eff<ErrorEffect<String>, i32> =
                ErrorEffect::throw(black_box("error".to_string()));
            let result = ErrorHandler::<String>::new().run(computation);
            black_box(result)
        });
    });

    group.bench_function("catch_recovery", |bencher| {
        bencher.iter(|| {
            let computation = catch(ErrorEffect::throw(black_box("error".to_string())), |_| {
                Eff::pure(42)
            });
            let result = ErrorHandler::<String>::new().run(computation);
            black_box(result)
        });
    });

    group.bench_function("attempt", |bencher| {
        bencher.iter(|| {
            let computation = attempt(ErrorEffect::<String>::throw::<i32>(black_box(
                "error".to_string(),
            )));
            let result = ErrorHandler::<String>::new().run(computation);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_error_effect_short_circuit(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("error_effect_short_circuit");

    for chain_length in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let handler = ErrorHandler::<String>::new();
                    let error_computation: Eff<ErrorEffect<String>, i32> =
                        ErrorEffect::throw(black_box("early error".to_string()));
                    let mut computation = error_computation;
                    for _ in 0..length {
                        computation = computation.flat_map(|x| Eff::pure(x + 1));
                    }
                    let result = handler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 6. Comparison with Traditional Monads
// =============================================================================

fn benchmark_reader_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("reader_comparison");

    group.bench_function("traditional_reader", |bencher| {
        bencher.iter(|| {
            let reader: Reader<i32, i32> = Reader::ask()
                .flat_map(|x: i32| Reader::new(move |y| x + y))
                .fmap(|x| x * 2);
            let result = reader.run(black_box(10));
            black_box(result)
        });
    });

    group.bench_function("eff_reader_effect", |bencher| {
        bencher.iter(|| {
            let computation = ReaderEffect::<i32>::ask()
                .flat_map(|x| ReaderEffect::ask().fmap(move |y| x + y))
                .fmap(|x| x * 2);
            let result = ReaderHandler::new(black_box(10)).run(computation);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_state_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_comparison");

    group.bench_function("traditional_state", |bencher| {
        bencher.iter(|| {
            let state: State<i32, i32> = State::get()
                .flat_map(|x: i32| State::put(x + 1).then(State::get()))
                .fmap(|x| x * 2);
            let (result, final_state) = state.run(black_box(10));
            black_box((result, final_state))
        });
    });

    group.bench_function("eff_state_effect", |bencher| {
        bencher.iter(|| {
            let computation = StateEffect::<i32>::get()
                .flat_map(|x| StateEffect::put(x + 1).then(StateEffect::get()))
                .fmap(|x| x * 2);
            let (result, final_state) = StateHandler::new(black_box(10)).run(computation);
            black_box((result, final_state))
        });
    });

    group.finish();
}

fn benchmark_writer_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("writer_comparison");

    group.bench_function("traditional_writer", |bencher| {
        bencher.iter(|| {
            let writer: Writer<Vec<String>, i32> = Writer::tell(vec!["step1".to_string()])
                .then(Writer::tell(vec!["step2".to_string()]))
                .then(Writer::pure(42));
            let (result, log) = writer.run();
            black_box((result, log))
        });
    });

    group.bench_function("eff_writer_effect", |bencher| {
        bencher.iter(|| {
            let computation = WriterEffect::tell(vec!["step1".to_string()])
                .then(WriterEffect::tell(vec!["step2".to_string()]))
                .then(Eff::pure(42));
            let (result, log) = WriterHandler::<Vec<String>>::new().run(computation);
            black_box((result, log))
        });
    });

    group.finish();
}

// =============================================================================
// 7. Composite Effects (EffectRow)
// =============================================================================

fn benchmark_effect_row_two_effects(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("effect_row_two_effects");

    type ReaderStateRow = EffectRow![ReaderEffect<i32>, StateEffect<i32>];

    group.bench_function("reader_only_from_row", |bencher| {
        bencher.iter(|| {
            let computation = ReaderEffect::<i32>::ask()
                .flat_map(|env| ReaderEffect::asks(move |x: i32| env + x));

            let result = ReaderHandler::new(black_box(10)).run(computation);
            black_box(result)
        });
    });

    group.bench_function("state_only_from_row", |bencher| {
        bencher.iter(|| {
            let computation = StateEffect::<i32>::get()
                .flat_map(|current| StateEffect::put(current + 1).then(StateEffect::get()));

            let (result, final_state) = StateHandler::new(black_box(10)).run(computation);
            black_box((result, final_state))
        });
    });

    group.bench_function("effect_row_type_construction", |bencher| {
        bencher.iter(|| {
            fn verify_row_effect<E: lambars::effect::algebraic::Effect>() {}
            verify_row_effect::<ReaderStateRow>();
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_effect_row_three_effects(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("effect_row_three_effects");

    type ThreeEffectRow = EffCons<
        ReaderEffect<i32>,
        EffCons<StateEffect<String>, EffCons<WriterEffect<Vec<i32>>, EffNil>>,
    >;

    group.bench_function("writer_only_from_row", |bencher| {
        bencher.iter(|| {
            let computation = WriterEffect::tell(vec![1, 2, 3])
                .then(WriterEffect::tell(vec![4, 5, 6]))
                .then(Eff::pure(42));

            let (result, log) = WriterHandler::<Vec<i32>>::new().run(computation);
            black_box((result, log))
        });
    });

    group.bench_function("effect_row_three_type_construction", |bencher| {
        bencher.iter(|| {
            fn verify_row_effect<E: lambars::effect::algebraic::Effect>() {}
            verify_row_effect::<ThreeEffectRow>();
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_effect_switching_pattern(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("effect_switching_pattern");

    group.bench_function("sequential_reader_then_state", |bencher| {
        bencher.iter(|| {
            let reader_computation = ReaderEffect::<i32>::ask().fmap(|env| env * 2);
            let reader_result = ReaderHandler::new(black_box(10)).run(reader_computation);

            let state_computation = StateEffect::put(reader_result)
                .then(StateEffect::modify(|x: i32| x + 1))
                .then(StateEffect::get());
            let (state_result, final_state) = StateHandler::new(0).run(state_computation);

            black_box((reader_result, state_result, final_state))
        });
    });

    group.bench_function("sequential_state_then_writer", |bencher| {
        bencher.iter(|| {
            let state_computation = StateEffect::<i32>::get()
                .flat_map(|x| StateEffect::put(x + 10).then(StateEffect::get()));
            let (state_result, _) = StateHandler::new(black_box(5)).run(state_computation);

            let writer_computation = WriterEffect::tell(vec![format!("Result: {}", state_result)])
                .then(Eff::pure(state_result));
            let (writer_result, log) = WriterHandler::<Vec<String>>::new().run(writer_computation);

            black_box((writer_result, log))
        });
    });

    group.bench_function("sequential_error_then_reader", |bencher| {
        bencher.iter(|| {
            let error_computation = catch(
                ErrorEffect::<String>::throw::<i32>("error".to_string()),
                |_| Eff::pure(42),
            );
            let error_result = ErrorHandler::<String>::new().run(error_computation);

            let value = error_result.unwrap_or(0);
            let reader_computation = ReaderEffect::<i32>::ask().fmap(move |env| env + value);
            let reader_result = ReaderHandler::new(black_box(10)).run(reader_computation);

            black_box(reader_result)
        });
    });

    group.finish();
}

// =============================================================================
// 8. Stack Safety Verification
// =============================================================================

fn benchmark_stack_safety(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("stack_safety");
    group.sample_size(10);

    for depth in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(depth),
            &depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    let handler = ReaderHandler::new(black_box(0));
                    let mut computation: Eff<ReaderEffect<i32>, i32> = ReaderEffect::ask();
                    for _ in 0..depth {
                        computation =
                            computation.flat_map(|x| ReaderEffect::ask().fmap(move |y| x + y));
                    }
                    let result = handler.run(computation);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 9. Practical Scenarios
// =============================================================================

fn benchmark_practical_config_reading(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("practical_config_reading");

    #[derive(Clone)]
    struct AppConfig {
        database_url: String,
        max_connections: i32,
        timeout_seconds: i32,
    }

    group.bench_function("web_app_config_pattern", |bencher| {
        let config = AppConfig {
            database_url: "postgres://localhost:5432/db".to_string(),
            max_connections: 100,
            timeout_seconds: 30,
        };

        bencher.iter(|| {
            let computation =
                ReaderEffect::<AppConfig>::asks(|c: AppConfig| c.database_url.clone())
                    .flat_map(|url| {
                        ReaderEffect::asks(move |c: AppConfig| (url, c.max_connections))
                    })
                    .flat_map(|(url, connections)| {
                        ReaderEffect::asks(move |c: AppConfig| {
                            (url, connections, c.timeout_seconds)
                        })
                    });
            let result = ReaderHandler::new(black_box(config.clone())).run(computation);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_practical_logging(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("practical_logging");

    group.bench_function("five_step_logging", |bencher| {
        bencher.iter(|| {
            let handler = WriterHandler::<Vec<String>>::new();
            let computation = WriterEffect::tell(vec!["Step 1: Initializing".to_string()])
                .then(Eff::pure(10))
                .flat_map(|x| {
                    WriterEffect::tell(vec![format!("Step 2: Processing value {}", x)])
                        .then(Eff::pure(x * 2))
                })
                .flat_map(|x| {
                    WriterEffect::tell(vec![format!("Step 3: Transformed to {}", x)])
                        .then(Eff::pure(x + 5))
                })
                .flat_map(|x| {
                    WriterEffect::tell(vec![format!("Step 4: Added offset, now {}", x)])
                        .then(Eff::pure(x))
                })
                .flat_map(|x| {
                    WriterEffect::tell(vec!["Step 5: Completed".to_string()]).then(Eff::pure(x))
                });
            let (result, log) = handler.run(computation);
            black_box((result, log))
        });
    });

    group.finish();
}

fn benchmark_practical_error_recovery(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("practical_error_recovery");

    fn validate_positive(x: i32) -> Eff<ErrorEffect<String>, i32> {
        if x > 0 {
            Eff::pure(x)
        } else {
            ErrorEffect::throw("Value must be positive".to_string())
        }
    }

    fn validate_less_than_100(x: i32) -> Eff<ErrorEffect<String>, i32> {
        if x < 100 {
            Eff::pure(x)
        } else {
            ErrorEffect::throw("Value must be less than 100".to_string())
        }
    }

    group.bench_function("nested_catch_pattern", |bencher| {
        bencher.iter(|| {
            let handler = ErrorHandler::<String>::new();
            let computation = catch(
                validate_positive(black_box(-5))
                    .flat_map(validate_less_than_100)
                    .fmap(|x| x * 2),
                |err| {
                    catch(
                        validate_positive(black_box(50))
                            .flat_map(validate_less_than_100)
                            .fmap(|x| x * 2),
                        move |_| Eff::pure(format!("All validations failed: {}", err).len() as i32),
                    )
                },
            );
            let result = handler.run(computation);
            black_box(result)
        });
    });

    group.bench_function("validation_chain_pattern", |bencher| {
        bencher.iter(|| {
            let handler = ErrorHandler::<String>::new();
            let computation = validate_positive(black_box(50))
                .flat_map(validate_less_than_100)
                .flat_map(|x| {
                    if x > 10 {
                        Eff::pure(x)
                    } else {
                        ErrorEffect::throw("Value must be greater than 10".to_string())
                    }
                })
                .fmap(|x| x * 2);
            let result = handler.run(computation);
            black_box(result)
        });
    });

    group.finish();
}

// =============================================================================
// Criterion Group and Main
// =============================================================================

criterion_group!(
    benches,
    // 1. Eff Basic Operations
    benchmark_eff_pure,
    benchmark_eff_fmap_chain,
    benchmark_eff_flat_map_chain,
    benchmark_eff_map2_product,
    // 2. Reader Effect
    benchmark_reader_effect,
    benchmark_reader_effect_local,
    benchmark_reader_effect_flat_map_chain,
    // 3. State Effect
    benchmark_state_effect,
    benchmark_state_effect_modify,
    benchmark_state_effect_get_modify_get,
    // 4. Writer Effect
    benchmark_writer_effect,
    benchmark_writer_effect_tell_chain,
    // 5. Error Effect
    benchmark_error_effect,
    benchmark_error_effect_short_circuit,
    // 6. Comparison with Traditional Monads
    benchmark_reader_comparison,
    benchmark_state_comparison,
    benchmark_writer_comparison,
    // 7. Composite Effects (EffectRow)
    benchmark_effect_row_two_effects,
    benchmark_effect_row_three_effects,
    benchmark_effect_switching_pattern,
    // 8. Stack Safety
    benchmark_stack_safety,
    // 9. Practical Scenarios
    benchmark_practical_config_reading,
    benchmark_practical_logging,
    benchmark_practical_error_recovery,
);

criterion_main!(benches);
