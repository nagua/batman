//! Terribly unsafe per-thread trapping exceptions for floating point operations.

#![feature(asm_const)]
#![feature(backtrace)]
#![deny(clippy::all)]

use log::info;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

mod backtrace;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

thread_local! {
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
}
static HANDLING: AtomicBool = AtomicBool::new(false);

/// Enable floating point unit exceptions.
///
/// FPU exception configuration is only allowed once per-thread; subsequent calls will be a no-op.
/// The configuration is thread-local. This function configures the environment in the following
/// manner:
///
/// - FPU "divide by zero" and "invalid operation" exceptions are enabled
///
/// Specifically, `batman` does not concern itself with details like precision loss, rounding
/// behavior, overflow/underflow, or handling subnormal numbers.
///
/// Threads inherit the FPU configuration from their parent (default disabled). Once enabled,
/// exceptions cannot be disabled on the thread (at least not by `batman`; other `unsafe` code
/// can disable exceptions at any time).
///
/// This function is a no-op when debug assertions are disabled.
///
/// # Safety
///
/// This function mutates global state (namely signal handlers).
pub unsafe fn signal() {
    INITIALIZED.with(|init| {
        let exch = init.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire);
        if exch.is_ok() {
            let id = thread::current().id();
            info!("Enabling FPU exceptions on thread {id:?}");

            let exch = HANDLING.compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire);
            if exch.is_ok() {
                #[cfg(unix)]
                unix::install_signal_handler();

                #[cfg(windows)]
                windows::install_exception_handler();
            }

            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::enable_fp_exceptions();
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            compile_error!("Unsupported platform");

            info!("FPU exceptions enabled on thread {id:?}");
        }
    });
}
