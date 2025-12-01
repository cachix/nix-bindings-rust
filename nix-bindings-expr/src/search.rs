//! Package search functionality similar to `nix search`.
//!
//! This module provides Rust bindings for the nix search C API, allowing
//! traversal of attribute sets to find derivations matching specified patterns.

use crate::attr_cursor::AttrCursor;
use anyhow::{bail, Result};
use nix_bindings_bindgen_raw as raw;
use std::ffi::{CStr, CString};
use std::ptr::NonNull;

/// Search result containing package information.
///
/// All fields are owned strings extracted during the search callback.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Full attribute path, e.g. "legacyPackages.x86_64-linux.hello"
    pub attr_path: String,
    /// Package name, e.g. "hello"
    pub name: String,
    /// Package version, e.g. "2.10" (may be empty)
    pub version: String,
    /// Package description (may be empty)
    pub description: String,
}

/// Search parameters configuration.
///
/// Use this to configure include and exclude regex patterns for searching.
///
/// # Example
///
/// ```ignore
/// let mut params = SearchParams::new()?;
/// params.add_regex("hello")?;           // Must match "hello"
/// params.add_regex("world")?;           // AND must match "world"
/// params.add_exclude("unfree")?;        // Exclude if matches "unfree"
/// ```
pub struct SearchParams {
    ptr: NonNull<raw::search_params>,
}

impl Drop for SearchParams {
    fn drop(&mut self) {
        unsafe {
            raw::search_params_free(self.ptr.as_ptr());
        }
    }
}

impl SearchParams {
    /// Create new search parameters with default settings.
    ///
    /// Default settings:
    /// - No include patterns (matches all)
    /// - No exclude patterns
    pub fn new() -> Result<Self> {
        let ptr = unsafe {
            let mut ctx: raw::c_context = std::mem::zeroed();
            let ptr = raw::search_params_new(&mut ctx);
            if ptr.is_null() {
                bail!("Failed to create search params");
            }
            ptr
        };

        Ok(Self {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Add an include regex pattern.
    ///
    /// Results must match ALL include patterns (AND logic).
    /// Patterns are matched case-insensitively against:
    /// - attribute path
    /// - package name
    /// - description
    ///
    /// Uses extended POSIX regex syntax.
    pub fn add_regex(&mut self, pattern: &str) -> Result<()> {
        let pattern_cstr = CString::new(pattern)?;

        let err = unsafe {
            let mut ctx: raw::c_context = std::mem::zeroed();
            raw::search_params_add_regex(&mut ctx, self.ptr.as_ptr(), pattern_cstr.as_ptr())
        };

        if err != 0 {
            bail!("Failed to add regex pattern: {}", pattern);
        }

        Ok(())
    }

    /// Add an exclude regex pattern.
    ///
    /// Results matching ANY exclude pattern are filtered out.
    /// Patterns are matched case-insensitively against:
    /// - attribute path
    /// - package name
    /// - description
    ///
    /// Uses extended POSIX regex syntax.
    pub fn add_exclude(&mut self, pattern: &str) -> Result<()> {
        let pattern_cstr = CString::new(pattern)?;

        let err = unsafe {
            let mut ctx: raw::c_context = std::mem::zeroed();
            raw::search_params_add_exclude(&mut ctx, self.ptr.as_ptr(), pattern_cstr.as_ptr())
        };

        if err != 0 {
            bail!("Failed to add exclude pattern: {}", pattern);
        }

        Ok(())
    }

    /// Get raw pointer for FFI.
    pub(crate) fn as_ptr(&self) -> *mut raw::search_params {
        self.ptr.as_ptr()
    }
}

/// Callback context passed through FFI.
struct SearchCallbackContext<'a, F>
where
    F: FnMut(SearchResult) -> bool,
{
    callback: &'a mut F,
}

/// C callback adapter that converts raw result to SearchResult and calls Rust closure.
unsafe extern "C" fn search_callback_adapter<F>(
    result: *const raw::search_result,
    user_data: *mut std::ffi::c_void,
) -> bool
where
    F: FnMut(SearchResult) -> bool,
{
    let ctx = &mut *(user_data as *mut SearchCallbackContext<F>);
    let result = &*result;

    let search_result = SearchResult {
        attr_path: if result.attr_path.is_null() {
            String::new()
        } else {
            CStr::from_ptr(result.attr_path)
                .to_string_lossy()
                .into_owned()
        },
        name: if result.name.is_null() {
            String::new()
        } else {
            CStr::from_ptr(result.name).to_string_lossy().into_owned()
        },
        version: if result.version.is_null() {
            String::new()
        } else {
            CStr::from_ptr(result.version)
                .to_string_lossy()
                .into_owned()
        },
        description: if result.description.is_null() {
            String::new()
        } else {
            CStr::from_ptr(result.description)
                .to_string_lossy()
                .into_owned()
        },
    };

    (ctx.callback)(search_result)
}

/// Search for packages matching the given patterns.
///
/// Traverses the attrset starting from cursor, finding derivations that
/// match the search criteria and invoking the callback for each match.
///
/// The traversal follows `nix search` semantics:
/// - Always recurses into the root level
/// - Always recurses 2 levels deep into `packages.*` and `legacyPackages.*`
/// - For deeper `legacyPackages` levels, only recurses if `recurseForDerivations = true`
/// - Evaluation errors in `legacyPackages` are silently ignored
///
/// # Arguments
///
/// * `cursor` - Root cursor to search from (typically from `EvalCache::root()`)
/// * `params` - Optional search parameters (regexes). Pass `None` for defaults (match all).
/// * `callback` - Function called for each result. Return `true` to continue, `false` to stop.
///
/// # Example
///
/// ```ignore
/// use nix_bindings_expr::{EvalCache, search, SearchParams};
///
/// let cache = EvalCache::new(&mut state, &nixpkgs, None)?;
/// let cursor = cache.root()?;
///
/// let mut params = SearchParams::new()?;
/// params.add_regex("hello")?;
///
/// search(&cursor, Some(&params), |result| {
///     println!("{}: {}", result.attr_path, result.description);
///     true // continue searching
/// })?;
/// ```
pub fn search<F>(cursor: &AttrCursor, params: Option<&SearchParams>, mut callback: F) -> Result<()>
where
    F: FnMut(SearchResult) -> bool,
{
    let mut ctx = SearchCallbackContext {
        callback: &mut callback,
    };

    let params_ptr = params
        .map(|p| p.as_ptr())
        .unwrap_or(std::ptr::null_mut());

    let err = unsafe {
        let mut nix_ctx: raw::c_context = std::mem::zeroed();
        raw::search(
            &mut nix_ctx,
            cursor.as_ptr(),
            params_ptr,
            Some(search_callback_adapter::<F>),
            &mut ctx as *mut SearchCallbackContext<F> as *mut std::ffi::c_void,
        )
    };

    if err != 0 {
        bail!("Search failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_search_params_basic() {
        // Basic compilation test
    }
}
