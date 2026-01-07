pub mod context;
pub mod settings;
#[macro_use]
pub mod string_return;

use nix_bindings_bindgen_raw as raw;

/// Trigger an interrupt.
///
/// Sets the interrupt flag and runs all registered interrupt callbacks.
/// This can be used to programmatically interrupt long-running Nix operations.
///
/// On Windows, this only sets the interrupt flag; callbacks are not supported.
pub fn trigger_interrupt() {
    unsafe { raw::trigger_interrupt() }
}
