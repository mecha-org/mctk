//! Capture stack traces and log performance of an app. Requires feature "instrumented" to be active.
//!
//! Traces are captured in the format used by <https://superluminal.eu/>. Logs are output using [log], which can be set up with any of many loggers.
//!
//! Lemna itself outputs spans relating to key phases, such as event handling, drawing, and rendering.

use std::cell::UnsafeCell;
use std::time::Instant;

#[cfg(feature = "instrumented")]
use log::info;

#[allow(dead_code)]
type Inst = (String, Instant);

thread_local!(
    static INST_STACK: UnsafeCell<Vec<Inst>> = {
        UnsafeCell::new(vec![])
    }
);

#[allow(dead_code)]
fn inst_stack_push(name: &str, instant: Instant) {
    INST_STACK.with(|r| unsafe { r.get().as_mut().unwrap().push((name.to_string(), instant)) })
}

#[allow(dead_code)]
fn inst_stack_pop() -> Inst {
    INST_STACK.with(|r| unsafe { r.get().as_mut().unwrap().pop().unwrap() })
}

#[cfg(feature = "instrumented")]
pub fn inst(name: &str) {
    superluminal_perf::begin_event(name);
    let now = Instant::now();
    info!("{:?} {} START", &now, name);
    inst_stack_push(name, now);
}

/// Start an instrumented span with the given name.
#[cfg(not(feature = "instrumented"))]
pub fn inst(_name: &str) {}

#[cfg(feature = "instrumented")]
pub fn inst_end() {
    superluminal_perf::end_event();
    let (name, prev) = inst_stack_pop();
    let now = Instant::now();
    info!(
        "{:?} {} END; Took {}μs",
        now,
        name,
        now.duration_since(prev).as_micros()
    );
}

/// Ends the last instrumentation span that was started, logging the time it took.
#[cfg(not(feature = "instrumented"))]
pub fn inst_end() {}

#[cfg(feature = "instrumented")]
pub fn evt(name: &str) {
    let now = Instant::now();
    info!("{:?} {}", now, name);
}

/// Log an event with the given name.
#[cfg(not(feature = "instrumented"))]
pub fn evt(_name: &str) {}
