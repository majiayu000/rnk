//! Hooks system benchmarks

use rnk::hooks::{HookContext, use_callback, use_effect, use_memo, use_signal, with_hooks};
use std::sync::{Arc, RwLock};

fn main() {
    divan::main();
}

#[divan::bench]
fn hook_context_creation() {
    let _ctx = HookContext::new();
}

#[divan::bench]
fn signal_creation() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        let _signal = use_signal(|| 0i32);
    });
}

#[divan::bench]
fn signal_get() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    let signal = with_hooks(ctx.clone(), || use_signal(|| 42i32));

    for _ in 0..100 {
        divan::black_box(signal.get());
    }
}

#[divan::bench]
fn signal_set() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    let signal = with_hooks(ctx.clone(), || use_signal(|| 0i32));

    for i in 0..100 {
        signal.set(i);
    }
}

#[divan::bench]
fn signal_update() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    let signal = with_hooks(ctx.clone(), || use_signal(|| 0i32));

    for _ in 0..100 {
        signal.update(|v| *v += 1);
    }
}

#[divan::bench]
fn signal_with() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    let signal = with_hooks(ctx.clone(), || use_signal(|| vec![1, 2, 3, 4, 5]));

    for _ in 0..100 {
        divan::black_box(signal.with(|v| v.iter().sum::<i32>()));
    }
}

#[divan::bench(args = [1, 5, 10, 20])]
fn multiple_signals(count: usize) {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        for _ in 0..count {
            let _signal = use_signal(|| 0i32);
        }
    });
}

#[divan::bench]
fn signal_persistence_across_renders() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    // Simulate multiple renders
    for i in 0..100 {
        with_hooks(ctx.clone(), || {
            let signal = use_signal(|| 0i32);
            if i == 0 {
                signal.set(42);
            }
            divan::black_box(signal.get());
        });
    }
}

#[divan::bench]
fn use_memo_simple() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        let _value = use_memo(|| expensive_computation(), ());
    });
}

#[divan::bench]
fn use_memo_with_deps() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    for i in 0..100 {
        with_hooks(ctx.clone(), || {
            let _value = use_memo(|| expensive_computation(), (i % 10,));
        });
    }
}

#[divan::bench]
fn use_callback_creation() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        let _cb = use_callback(|x: i32| x * 2, ());
    });
}

#[divan::bench]
fn use_effect_registration() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        use_effect(
            || {
                // Effect body
                None
            },
            (),
        );
    });
}

#[divan::bench]
fn use_effect_with_deps_change() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    for i in 0..100 {
        with_hooks(ctx.clone(), || {
            use_effect(
                || {
                    // Effect body
                    None
                },
                (i,),
            );
        });
    }
}

#[divan::bench]
fn complex_hook_composition() {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        let count = use_signal(|| 0i32);
        let name = use_signal(|| String::from("test"));

        let _doubled = use_memo(|| count.get() * 2, count.get());

        let _callback = use_callback(
            {
                let count = count.clone();
                move |delta: i32| count.update(|v| *v += delta)
            },
            (),
        );

        use_effect(
            || {
                // Side effect
                None
            },
            (count.get(), name.get()),
        );
    });
}

#[divan::bench(args = [10, 50, 100])]
fn many_hooks_in_component(count: usize) {
    let ctx = Arc::new(RwLock::new(HookContext::new()));

    with_hooks(ctx, || {
        for i in 0..count {
            let _signal = use_signal(|| i);
        }
    });
}

fn expensive_computation() -> i32 {
    // Simulate some computation
    (0..100).sum()
}
