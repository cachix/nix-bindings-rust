//! Lazy attribute cursor for efficient traversal of Nix attribute sets.
//!
//! AttrCursor provides a way to traverse attribute sets lazily, only forcing
//! evaluation when values are actually accessed. This is more efficient than
//! eager evaluation when you don't need all values.
//!
//! Cursors are obtained from [`EvalCache::root()`](crate::EvalCache::root) or
//! [`AttrCursor::get_attr()`].

use anyhow::{bail, Result};
use nix_bindings_bindgen_raw as raw;
use std::ffi::CString;
use std::ptr::NonNull;

/// Cursor for lazy traversal of attribute sets.
///
/// AttrCursor allows navigating through nested attribute sets without forcing
/// evaluation of values until they are actually needed.
///
/// Cursors are created via [`EvalCache::root()`](crate::EvalCache::root) or by
/// navigating with [`get_attr()`](Self::get_attr).
pub struct AttrCursor {
    ptr: NonNull<raw::attr_cursor>,
}

impl Drop for AttrCursor {
    fn drop(&mut self) {
        unsafe {
            raw::attr_cursor_free(self.ptr.as_ptr());
        }
    }
}

impl AttrCursor {
    /// Create a cursor from a raw pointer.
    ///
    /// This is used internally by `EvalCache::root()` and `get_attr()`.
    pub(crate) fn from_raw(ptr: NonNull<raw::attr_cursor>) -> Self {
        Self { ptr }
    }

    /// Get raw pointer for FFI.
    pub(crate) fn as_ptr(&self) -> *mut raw::attr_cursor {
        self.ptr.as_ptr()
    }

    /// Get a child attribute by name, returning None if not found.
    pub fn get_attr(&self, name: &str) -> Result<Option<AttrCursor>> {
        let name_cstr = CString::new(name)?;

        let ptr = unsafe {
            raw::attr_cursor_get_attr(std::ptr::null_mut(), self.ptr.as_ptr(), name_cstr.as_ptr())
        };

        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(AttrCursor::from_raw(NonNull::new(ptr).unwrap())))
        }
    }

    /// Get the number of attributes in the current attrset.
    pub fn attrs_count(&self) -> Result<usize> {
        let count = unsafe {
            raw::attr_cursor_get_attrs_count(std::ptr::null_mut(), self.ptr.as_ptr())
        };
        if count < 0 {
            bail!("Failed to get attrs count");
        }
        Ok(count as usize)
    }

    /// Get all attribute names.
    pub fn attr_names(&self) -> Result<Vec<String>> {
        let count = self.attrs_count()?;
        let mut names = Vec::with_capacity(count);

        for i in 0..count {
            let name = self.attr_name_at(i)?;
            names.push(name);
        }

        Ok(names)
    }

    /// Get attribute name at index.
    pub fn attr_name_at(&self, index: usize) -> Result<String> {
        let mut result = String::new();

        extern "C" fn callback(
            data: *const std::ffi::c_char,
            len: std::ffi::c_uint,
            user_data: *mut std::ffi::c_void,
        ) {
            unsafe {
                let result = &mut *(user_data as *mut String);
                let slice = std::slice::from_raw_parts(data as *const u8, len as usize);
                *result = String::from_utf8_lossy(slice).into_owned();
            }
        }

        let err = unsafe {
            raw::attr_cursor_get_attr_name(
                std::ptr::null_mut(),
                self.ptr.as_ptr(),
                index as std::ffi::c_uint,
                Some(callback),
                &mut result as *mut String as *mut std::ffi::c_void,
            )
        };

        if err != 0 {
            bail!("Failed to get attr name at index {}", index);
        }

        Ok(result)
    }

    /// Check if the current value is a derivation.
    pub fn is_derivation(&self) -> Result<bool> {
        let mut is_drv = false;
        let err = unsafe {
            raw::attr_cursor_is_derivation(std::ptr::null_mut(), self.ptr.as_ptr(), &mut is_drv)
        };
        if err != 0 {
            bail!("Failed to check is_derivation");
        }
        Ok(is_drv)
    }

    /// Get the string value at cursor.
    pub fn get_string(&self) -> Result<String> {
        let mut result = String::new();

        extern "C" fn callback(
            data: *const std::ffi::c_char,
            len: std::ffi::c_uint,
            user_data: *mut std::ffi::c_void,
        ) {
            unsafe {
                let result = &mut *(user_data as *mut String);
                let slice = std::slice::from_raw_parts(data as *const u8, len as usize);
                *result = String::from_utf8_lossy(slice).into_owned();
            }
        }

        let err = unsafe {
            raw::attr_cursor_get_string(
                std::ptr::null_mut(),
                self.ptr.as_ptr(),
                Some(callback),
                &mut result as *mut String as *mut std::ffi::c_void,
            )
        };

        if err != 0 {
            bail!("Failed to get string value");
        }

        Ok(result)
    }

    /// Get the boolean value at cursor.
    pub fn get_bool(&self) -> Result<bool> {
        let mut value = false;
        let err = unsafe {
            raw::attr_cursor_get_bool(std::ptr::null_mut(), self.ptr.as_ptr(), &mut value)
        };
        if err != 0 {
            bail!("Failed to get bool value");
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_attr_cursor_basic() {
        // Basic compilation test
    }
}
