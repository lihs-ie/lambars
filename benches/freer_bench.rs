//! Benchmark for Freer monad.
//!
//! Measures the performance of Freer monad operations including DSL construction,
//! interpretation, and comparison with other control structures.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lambars::control::{Freer, Trampoline};
use std::hint::black_box;

// =============================================================================
// DSL Definitions
// =============================================================================

/// State DSL - common pattern for stateful computations
enum StateCommand {
    Get,
    Put(i32),
    Modify(Box<dyn FnOnce(i32) -> i32 + Send>),
}

fn state_get() -> Freer<StateCommand, i32> {
    Freer::<StateCommand, ()>::lift_instruction(StateCommand::Get, |result| {
        *result.downcast::<i32>().expect("Get must return i32")
    })
}

fn state_put(value: i32) -> Freer<StateCommand, ()> {
    Freer::<StateCommand, ()>::lift_instruction(StateCommand::Put(value), |_| ())
}

fn state_modify<F>(f: F) -> Freer<StateCommand, ()>
where
    F: FnOnce(i32) -> i32 + Send + 'static,
{
    Freer::<StateCommand, ()>::lift_instruction(StateCommand::Modify(Box::new(f)), |_| ())
}

fn run_state_dsl<A: 'static>(program: Freer<StateCommand, A>, initial_state: i32) -> (A, i32) {
    let mut state = initial_state;
    let result = program.interpret(|command| match command {
        StateCommand::Get => Box::new(state),
        StateCommand::Put(value) => {
            state = value;
            Box::new(())
        }
        StateCommand::Modify(f) => {
            state = f(state);
            Box::new(())
        }
    });
    (result, state)
}

/// Console DSL - common pattern for I/O operations
#[derive(Debug)]
enum ConsoleCommand {
    ReadLine,
    PrintLine(String),
}

fn console_print(message: String) -> Freer<ConsoleCommand, ()> {
    Freer::<ConsoleCommand, ()>::lift_instruction(ConsoleCommand::PrintLine(message), |_| ())
}

fn console_read() -> Freer<ConsoleCommand, String> {
    Freer::<ConsoleCommand, ()>::lift_instruction(ConsoleCommand::ReadLine, |result| {
        *result
            .downcast::<String>()
            .expect("ReadLine must return String")
    })
}

/// Counter DSL - simple increment/decrement operations
#[derive(Debug, Clone, Copy)]
enum CounterCommand {
    Increment,
    Decrement,
    GetCount,
}

fn counter_inc() -> Freer<CounterCommand, ()> {
    Freer::<CounterCommand, ()>::lift_instruction(CounterCommand::Increment, |_| ())
}

fn counter_dec() -> Freer<CounterCommand, ()> {
    Freer::<CounterCommand, ()>::lift_instruction(CounterCommand::Decrement, |_| ())
}

fn counter_get() -> Freer<CounterCommand, i32> {
    Freer::<CounterCommand, ()>::lift_instruction(CounterCommand::GetCount, |result| {
        *result.downcast::<i32>().expect("GetCount must return i32")
    })
}

fn run_counter_dsl<A: 'static>(program: Freer<CounterCommand, A>, initial: i32) -> (A, i32) {
    let mut count = initial;
    let result = program.interpret(|command| match command {
        CounterCommand::Increment => {
            count += 1;
            Box::new(())
        }
        CounterCommand::Decrement => {
            count -= 1;
            Box::new(())
        }
        CounterCommand::GetCount => Box::new(count),
    });
    (result, count)
}

// =============================================================================
// 1. Basic Operations
// =============================================================================

fn benchmark_freer_pure(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_pure");

    group.bench_function("pure_i32", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(black_box(42));
            let result = freer.interpret(|_| Box::new(()));
            black_box(result)
        });
    });

    group.bench_function("pure_string", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), String> = Freer::pure(black_box("hello".to_string()));
            let result = freer.interpret(|_| Box::new(()));
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_freer_lift_instruction(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_lift_instruction");

    group.bench_function("single_instruction", |bencher| {
        bencher.iter(|| {
            let freer = counter_inc();
            let (result, count) = run_counter_dsl(freer, black_box(0));
            black_box((result, count))
        });
    });

    group.bench_function("instruction_with_result", |bencher| {
        bencher.iter(|| {
            let freer = counter_get();
            let (result, count) = run_counter_dsl(freer, black_box(42));
            black_box((result, count))
        });
    });

    group.finish();
}

// =============================================================================
// 2. Functor Operations (map)
// =============================================================================

fn benchmark_freer_map(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_map");

    group.bench_function("map_pure", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(black_box(21));
            let mapped = freer.map(|x| x * 2);
            let result = mapped.interpret(|_| Box::new(()));
            black_box(result)
        });
    });

    for chain_length in [2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("map_chain", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut freer: Freer<(), i32> = Freer::pure(1);
                    for _ in 0..length {
                        freer = freer.map(|x| x + 1);
                    }
                    let result = freer.interpret(|_| Box::new(()));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 3. Monad Operations (flat_map, and_then, then)
// =============================================================================

fn benchmark_freer_flat_map(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_flat_map");

    group.bench_function("flat_map_pure_to_pure", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(black_box(21));
            let result = freer
                .flat_map(|x| Freer::pure(x * 2))
                .interpret(|_| Box::new(()));
            black_box(result)
        });
    });

    for chain_length in [2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("flat_map_chain", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut freer: Freer<(), i32> = Freer::pure(1);
                    for _ in 0..length {
                        freer = freer.flat_map(|x| Freer::pure(x + 1));
                    }
                    let result = freer.interpret(|_| Box::new(()));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_freer_then(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_then");

    group.bench_function("then_two_operations", |bencher| {
        bencher.iter(|| {
            let freer = counter_inc().then(counter_get());
            let (result, count) = run_counter_dsl(freer, black_box(0));
            black_box((result, count))
        });
    });

    for chain_length in [2, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("then_chain", chain_length),
            &chain_length,
            |bencher, &length| {
                bencher.iter(|| {
                    let mut freer: Freer<CounterCommand, ()> = counter_inc();
                    for _ in 1..length {
                        freer = freer.then(counter_inc());
                    }
                    let (result, count) = run_counter_dsl(freer, black_box(0));
                    black_box((result, count))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 4. Interpretation Performance
// =============================================================================

fn benchmark_freer_interpret(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_interpret");

    group.bench_function("interpret_pure", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(black_box(42));
            let result = freer.interpret(|_| Box::new(()));
            black_box(result)
        });
    });

    group.bench_function("interpret_single_instruction", |bencher| {
        bencher.iter(|| {
            let freer = state_get();
            let (result, state) = run_state_dsl(freer, black_box(42));
            black_box((result, state))
        });
    });

    for instruction_count in [5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("interpret_instruction_chain", instruction_count),
            &instruction_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let mut freer: Freer<CounterCommand, ()> = counter_inc();
                    for _ in 1..count {
                        freer = freer.then(counter_inc());
                    }
                    let freer = freer.then(counter_get()).map(|x| x * 2);
                    let (result, count) = run_counter_dsl(freer, black_box(0));
                    black_box((result, count))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 5. Stack Safety
// =============================================================================

fn benchmark_freer_stack_safety(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_stack_safety");
    group.sample_size(10);

    for depth in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("deep_flat_map_pure", depth),
            &depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    let mut freer: Freer<(), i32> = Freer::pure(0);
                    for _ in 0..depth {
                        freer = freer.flat_map(|x| Freer::pure(x + 1));
                    }
                    let result = freer.interpret(|_| Box::new(()));
                    black_box(result)
                });
            },
        );
    }

    for depth in [100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("deep_flat_map_with_instructions", depth),
            &depth,
            |bencher, &depth| {
                bencher.iter(|| {
                    let mut freer: Freer<CounterCommand, i32> = counter_get();
                    for _ in 0..depth {
                        freer = freer.flat_map(|_| counter_inc().then(counter_get()));
                    }
                    let (result, count) = run_counter_dsl(freer, black_box(0));
                    black_box((result, count))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// 6. Comparison with Trampoline
// =============================================================================

fn sum_freer(n: i32) -> Freer<(), i32> {
    if n <= 0 {
        Freer::pure(0)
    } else {
        Freer::pure(n).flat_map(move |x| sum_freer(n - 1).map(move |rest| x + rest))
    }
}

fn sum_trampoline(n: i32) -> Trampoline<i32> {
    sum_trampoline_helper(n, 0)
}

fn sum_trampoline_helper(n: i32, accumulator: i32) -> Trampoline<i32> {
    if n <= 0 {
        Trampoline::done(accumulator)
    } else {
        Trampoline::suspend(move || sum_trampoline_helper(n - 1, accumulator + n))
    }
}

fn benchmark_freer_vs_trampoline(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_vs_trampoline");

    for n in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("freer_sum", n), &n, |bencher, &n| {
            bencher.iter(|| {
                let result = sum_freer(black_box(n)).interpret(|_| Box::new(()));
                black_box(result)
            });
        });

        group.bench_with_input(BenchmarkId::new("trampoline_sum", n), &n, |bencher, &n| {
            bencher.iter(|| {
                let result = sum_trampoline(black_box(n)).run();
                black_box(result)
            });
        });
    }

    group.finish();
}

// =============================================================================
// 7. Practical DSL Scenarios
// =============================================================================

fn benchmark_state_dsl_practical(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("state_dsl_practical");

    group.bench_function("get_modify_get", |bencher| {
        bencher.iter(|| {
            let program = state_get()
                .flat_map(|x| state_modify(move |s| s + x))
                .then(state_get());
            let (result, state) = run_state_dsl(program, black_box(10));
            black_box((result, state))
        });
    });

    group.bench_function("counter_simulation", |bencher| {
        bencher.iter(|| {
            let program = state_get()
                .flat_map(|initial| {
                    state_put(initial + 1)
                        .then(state_get())
                        .flat_map(|x| state_put(x * 2).then(state_get()))
                })
                .flat_map(|x| state_modify(move |s| s + x).then(state_get()));
            let (result, state) = run_state_dsl(program, black_box(5));
            black_box((result, state))
        });
    });

    for ops in [5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("increment_chain", ops),
            &ops,
            |bencher, &ops| {
                bencher.iter(|| {
                    let mut program: Freer<StateCommand, ()> = state_modify(|x| x + 1);
                    for _ in 1..ops {
                        program = program.then(state_modify(|x| x + 1));
                    }
                    let program = program.then(state_get());
                    let (result, state) = run_state_dsl(program, black_box(0));
                    black_box((result, state))
                });
            },
        );
    }

    group.finish();
}

fn benchmark_console_dsl_practical(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("console_dsl_practical");

    group.bench_function("greeting_program", |bencher| {
        bencher.iter(|| {
            let program = console_print("What is your name?".to_string())
                .then(console_read())
                .flat_map(|name| {
                    console_print(format!("Hello, {}!", name)).then(Freer::pure(name.len()))
                });

            let mut output = Vec::new();
            let result = program.interpret(|command| match command {
                ConsoleCommand::ReadLine => Box::new("Alice".to_string()),
                ConsoleCommand::PrintLine(msg) => {
                    output.push(msg);
                    Box::new(())
                }
            });
            black_box((result, output))
        });
    });

    group.bench_function("multi_step_dialog", |bencher| {
        bencher.iter(|| {
            let program = console_print("Step 1: Enter first number".to_string())
                .then(console_read())
                .flat_map(|a| {
                    console_print("Step 2: Enter second number".to_string())
                        .then(console_read())
                        .map(move |b| (a, b))
                })
                .flat_map(|(a, b)| {
                    let sum = a.parse::<i32>().unwrap_or(0) + b.parse::<i32>().unwrap_or(0);
                    console_print(format!("Result: {}", sum)).then(Freer::pure(sum))
                });

            let inputs = ["10", "20"];
            let mut input_index = 0;
            let mut output = Vec::new();

            let result = program.interpret(|command| match command {
                ConsoleCommand::ReadLine => {
                    let value = inputs[input_index].to_string();
                    input_index += 1;
                    Box::new(value)
                }
                ConsoleCommand::PrintLine(msg) => {
                    output.push(msg);
                    Box::new(())
                }
            });
            black_box((result, output))
        });
    });

    group.finish();
}

fn benchmark_counter_dsl_practical(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("counter_dsl_practical");

    group.bench_function("increment_to_target", |bencher| {
        bencher.iter(|| {
            fn increment_until(target: i32) -> Freer<CounterCommand, i32> {
                counter_get().flat_map(move |current| {
                    if current >= target {
                        Freer::pure(current)
                    } else {
                        counter_inc().then(increment_until(target))
                    }
                })
            }

            let program = increment_until(black_box(10));
            let (result, count) = run_counter_dsl(program, 0);
            black_box((result, count))
        });
    });

    group.bench_function("mixed_operations", |bencher| {
        bencher.iter(|| {
            let program = counter_inc()
                .then(counter_inc())
                .then(counter_inc())
                .then(counter_get())
                .flat_map(|x| {
                    if x > 2 {
                        counter_dec().then(counter_get())
                    } else {
                        counter_inc().then(counter_get())
                    }
                });
            let (result, count) = run_counter_dsl(program, black_box(0));
            black_box((result, count))
        });
    });

    group.finish();
}

// =============================================================================
// 8. Memory and Allocation Patterns
// =============================================================================

fn benchmark_freer_allocation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("freer_allocation");

    group.bench_function("pure_no_alloc", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(black_box(42));
            black_box(freer)
        });
    });

    group.bench_function("lift_instruction_alloc", |bencher| {
        bencher.iter(|| {
            let freer = counter_get();
            black_box(freer)
        });
    });

    group.bench_function("flat_map_alloc", |bencher| {
        bencher.iter(|| {
            let freer: Freer<(), i32> = Freer::pure(1);
            let result = freer.flat_map(|x| Freer::pure(x + 1));
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
    // 1. Basic Operations
    benchmark_freer_pure,
    benchmark_freer_lift_instruction,
    // 2. Functor Operations
    benchmark_freer_map,
    // 3. Monad Operations
    benchmark_freer_flat_map,
    benchmark_freer_then,
    // 4. Interpretation Performance
    benchmark_freer_interpret,
    // 5. Stack Safety
    benchmark_freer_stack_safety,
    // 6. Comparison with Trampoline
    benchmark_freer_vs_trampoline,
    // 7. Practical DSL Scenarios
    benchmark_state_dsl_practical,
    benchmark_console_dsl_practical,
    benchmark_counter_dsl_practical,
    // 8. Memory and Allocation
    benchmark_freer_allocation,
);

criterion_main!(benches);
