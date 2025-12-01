//! Evaluation cache for lazy attribute evaluation with optional SQLite backing.
//!
//! EvalCache wraps the C++ `eval_cache::EvalCache` class, providing lazy
//! attribute traversal with optional persistent caching to SQLite.

use crate::attr_cursor::AttrCursor;
use crate::eval_state::EvalState;
use crate::value::Value;
use anyhow::{bail, Result};
use nix_bindings_bindgen_raw as raw;
use std::ffi::CString;
use std::ptr::NonNull;

/// Evaluation cache for lazy attribute evaluation.
///
/// EvalCache provides lazy evaluation of Nix attribute sets with optional
/// SQLite caching for faster subsequent access.
///
/// # Example
///
/// ```ignore
/// let cache = EvalCache::new(&mut state, &value, None)?;
/// let cursor = cache.root()?;
///
/// // Navigate lazily
/// if let Some(hello) = cursor.get_attr("hello")? {
///     println!("Found hello: {}", hello.is_derivation()?);
/// }
/// ```
pub struct EvalCache {
    ptr: NonNull<raw::eval_cache>,
}

impl Drop for EvalCache {
    fn drop(&mut self) {
        unsafe {
            raw::eval_cache_free(self.ptr.as_ptr());
        }
    }
}

impl EvalCache {
    /// Create an evaluation cache from a root value.
    ///
    /// # Arguments
    ///
    /// * `eval_state` - The evaluation state
    /// * `value` - The root value to cache (usually an attrset)
    /// * `cache_key` - Optional SHA256 hash string for cache fingerprint.
    ///                 If `None`, SQLite caching is disabled. If provided,
    ///                 results are cached to `~/.cache/nix/eval-cache-v6/<hash>.sqlite`
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Without caching
    /// let cache = EvalCache::new(&mut state, &nixpkgs, None)?;
    ///
    /// // With caching (e.g., using flake lock hash)
    /// let cache = EvalCache::new(&mut state, &nixpkgs, Some("abc123..."))?;
    /// ```
    pub fn new(
        eval_state: &mut EvalState,
        value: &Value,
        cache_key: Option<&str>,
    ) -> Result<Self> {
        let cache_key_cstr = cache_key.map(|s| CString::new(s)).transpose()?;
        let cache_key_ptr = cache_key_cstr
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null());

        let ptr = unsafe {
            let mut ctx: raw::c_context = std::mem::zeroed();
            let ptr = raw::eval_cache_create(
                &mut ctx,
                eval_state.raw_ptr(),
                value.raw_ptr(),
                cache_key_ptr,
            );
            if ptr.is_null() {
                bail!("Failed to create eval cache");
            }
            ptr
        };

        Ok(Self {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Get the root cursor from this cache.
    ///
    /// The returned cursor points to the root of the cached value and can be
    /// used to lazily traverse the attribute set.
    pub fn root(&self) -> Result<AttrCursor> {
        let ptr = unsafe {
            let mut ctx: raw::c_context = std::mem::zeroed();
            let ptr = raw::eval_cache_get_root(&mut ctx, self.ptr.as_ptr());
            if ptr.is_null() {
                bail!("Failed to get root cursor from eval cache");
            }
            ptr
        };

        Ok(AttrCursor::from_raw(NonNull::new(ptr).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_eval_cache_basic() {
        // Basic compilation test
    }
}
