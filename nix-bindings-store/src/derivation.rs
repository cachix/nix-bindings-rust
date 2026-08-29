#![cfg(nix_at_least = "2.33")]

use anyhow::Result;
use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::{
    check_call,
    context::Context,
    result_string_init,
    string_return::{callback_get_result_string, callback_get_result_string_data},
};
use std::ptr::NonNull;

/// A Nix derivation
///
/// **Requires Nix 2.33 or later.**
pub struct Derivation {
    pub(crate) inner: NonNull<raw::derivation>,
}

impl Derivation {
    pub(crate) fn new_raw(inner: NonNull<raw::derivation>) -> Self {
        Derivation { inner }
    }

    /// Return the derivation in Nix's JSON format.
    pub fn to_json(&self) -> Result<String> {
        let mut context = Context::new();
        let mut result = result_string_init!();
        unsafe {
            check_call!(raw::derivation_to_json(
                &mut context,
                self.inner.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut result)
            ))?;
        }
        result
    }
}

impl Clone for Derivation {
    fn clone(&self) -> Self {
        let inner = unsafe { raw::derivation_clone(self.inner.as_ptr()) };
        let inner = NonNull::new(inner).expect("nix_derivation_clone returned null");
        Derivation::new_raw(inner)
    }
}

impl Drop for Derivation {
    fn drop(&mut self) {
        unsafe {
            raw::derivation_free(self.inner.as_ptr());
        }
    }
}
