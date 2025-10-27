//! # Nix Command Utilities
//!
//! This module provides Rust bindings for Nix command utilities, including
//! interactive REPL support.

use nix_bindings_bindgen_raw as raw;
use nix_bindings_expr::value::Value;
use nix_bindings_util::context::Context;
use std::ptr::NonNull;

/// Initialize the Nix command library (REPL support).
///
/// This function must be called at least once before using any other functions
/// in this module. It is idempotent and can be called multiple times.
///
/// # Errors
///
/// Returns an error if the initialization fails.
pub fn init() -> anyhow::Result<()> {
    let mut ctx = Context::new();
    unsafe {
        nix_bindings_util::check_call!(raw::libcmd_init(&mut ctx))?;
    }
    Ok(())
}

/// Exit status from the REPL.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReplExitStatus {
    /// The REPL exited with `:quit`. The program should exit.
    QuitAll,
    /// The REPL exited with `:continue`. The program should continue running.
    Continue,
}

impl ReplExitStatus {
    fn from_raw(raw: raw::repl_exit_status) -> Self {
        match raw {
            raw::repl_exit_status_NIX_REPL_EXIT_QUIT_ALL => ReplExitStatus::QuitAll,
            raw::repl_exit_status_NIX_REPL_EXIT_CONTINUE => ReplExitStatus::Continue,
            _ => ReplExitStatus::Continue, // default fallback
        }
    }
}

/// A map of string keys to Nix values for REPL environment.
///
/// This is used to inject pre-populated variables into the REPL scope.
pub struct ValMap {
    ptr: NonNull<raw::valmap>,
}

impl ValMap {
    /// Create a new, empty ValMap.
    ///
    /// # Errors
    ///
    /// Returns an error if the ValMap allocation fails.
    pub fn new() -> anyhow::Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe { raw::valmap_new(ctx.ptr()) };

        if ptr.is_null() {
            ctx.check_err()?;
            return Err(anyhow::anyhow!("Failed to allocate ValMap"));
        }

        Ok(ValMap {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Insert a key-value pair into this ValMap.
    ///
    /// The value is copied/referenced but not owned by the map; you remain
    /// responsible for its memory management.
    ///
    /// # Errors
    ///
    /// Returns an error if the insertion fails.
    pub fn insert(&mut self, key: &str, value: &Value) -> anyhow::Result<()> {
        let key_cstr = std::ffi::CString::new(key)?;
        let mut ctx = Context::new();

        unsafe {
            nix_bindings_util::check_call!(raw::valmap_insert(
                &mut ctx,
                self.ptr.as_ptr(),
                key_cstr.as_ptr(),
                value.raw_ptr()
            ))?;
        }

        Ok(())
    }

    /// Get a raw pointer to the underlying ValMap.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is not used after this ValMap is dropped.
    unsafe fn raw_ptr(&self) -> *mut raw::valmap {
        self.ptr.as_ptr()
    }
}

impl Drop for ValMap {
    fn drop(&mut self) {
        unsafe {
            raw::valmap_free(self.ptr.as_ptr());
        }
    }
}

impl Default for ValMap {
    fn default() -> Self {
        Self::new().expect("Failed to create default ValMap")
    }
}

/// Run a simple REPL with an EvalState and optional extra variables.
///
/// This function launches an interactive Nix REPL with the given evaluation state
/// and optionally pre-populated variables from a ValMap. The REPL runs until the
/// user exits with `:quit` or `:continue`.
///
/// # Arguments
///
/// * `eval_state` - The evaluation state for the REPL (mutable to allow internal state updates)
/// * `extra_env` - Optional ValMap of additional variables to inject into scope
///
/// # Errors
///
/// Returns an error if the REPL fails to start or encounters an error during execution.
///
/// # Example
///
/// ```rust,no_run
/// use nix_cmd::{init, ValMap, run_repl_simple};
/// use nix_bindings_expr::eval_state::EvalState;
/// use nix_bindings_store::store::Store;
/// use std::collections::HashMap;
///
/// # fn example() -> anyhow::Result<()> {
/// init()?;
/// let store = Store::open(None, HashMap::new())?;
/// let mut eval_state = EvalState::new(store, [])?;
/// let mut extra_env = ValMap::new()?;
/// // Populate extra_env with values...
/// let status = run_repl_simple(&mut eval_state, Some(&mut extra_env))?;
/// # Ok(())
/// # }
/// ```
pub fn run_repl_simple(
    eval_state: &mut nix_bindings_expr::eval_state::EvalState,
    mut extra_env: Option<&mut ValMap>,
) -> anyhow::Result<ReplExitStatus> {
    let mut ctx = Context::new();
    let mut exit_status: raw::repl_exit_status = 0;

    let extra_env_ptr = extra_env
        .as_mut()
        .map(|env| unsafe { env.raw_ptr() })
        .unwrap_or(std::ptr::null_mut());

    unsafe {
        nix_bindings_util::check_call!(raw::repl_run_simple(
            &mut ctx,
            eval_state.raw_ptr(),
            extra_env_ptr,
            &mut exit_status as *mut _
        ))?;
    }

    Ok(ReplExitStatus::from_raw(exit_status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_init() {
        assert!(init().is_ok());
    }

    #[test]
    fn test_valmap_new() {
        let result = ValMap::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_valmap_default() {
        let _valmap = ValMap::default();
        // Just verify it doesn't panic on creation and drop
    }
}
