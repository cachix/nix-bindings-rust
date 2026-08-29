use std::ptr::NonNull;

use anyhow::{Context as _, Result};
use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::{
    check_call,
    context::Context,
    result_string_init,
    string_return::{callback_get_result_string, callback_get_result_string_data},
};

pub struct StorePath {
    raw: NonNull<raw::StorePath>,
}
impl StorePath {
    /// Get the name of the store path.
    ///
    /// For a store path like `/nix/store/abc1234...-foo-1.2`, this function will return `foo-1.2`.
    pub fn name(&self) -> Result<String> {
        unsafe {
            let mut r = result_string_init!();
            raw::store_path_name(
                self.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r),
            );
            r
        }
    }

    /// Return the 20-byte hash part of this store path.
    #[cfg(nix_at_least = "2.33")]
    pub fn hash(&self) -> Result<[u8; 20]> {
        let mut context = Context::new();
        let mut hash = raw::store_path_hash_part { bytes: [0; 20] };
        unsafe {
            check_call!(raw::store_path_hash(&mut context, self.as_ptr(), &mut hash))?;
        }
        Ok(hash.bytes)
    }

    /// Construct a store path from its 20-byte hash and name.
    #[cfg(nix_at_least = "2.33")]
    pub fn from_parts(hash: [u8; 20], name: &str) -> Result<Self> {
        let mut context = Context::new();
        let hash = raw::store_path_hash_part { bytes: hash };
        let path = unsafe {
            check_call!(raw::store_create_from_parts(
                &mut context,
                &hash,
                name.as_ptr().cast(),
                name.len()
            ))?
        };
        let path = NonNull::new(path).context("nix_store_create_from_parts returned null")?;
        Ok(unsafe { StorePath::new_raw(path) })
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Construct a new `StorePath` by first cloning the C store path.
    ///
    /// # Safety
    ///
    /// This does not take ownership of the C store path, so it should be a borrowed pointer, or you should free it.
    pub unsafe fn new_raw_clone(raw: NonNull<raw::StorePath>) -> Self {
        Self::new_raw(
            NonNull::new(raw::store_path_clone(raw.as_ptr()))
                .or_else(|| panic!("nix_store_path_clone returned a null pointer"))
                .unwrap(),
        )
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Takes ownership of a C `nix_store_path`. It will be freed when the `StorePath` is dropped.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the provided `NonNull<raw::StorePath>` is valid and that the ownership
    /// semantics are correctly followed. The `raw` pointer must not be used after being passed to this function.
    pub unsafe fn new_raw(raw: NonNull<raw::StorePath>) -> Self {
        StorePath { raw }
    }

    /// This is a low level function that you shouldn't have to call unless you are developing the Nix bindings.
    ///
    /// Get a pointer to the underlying Nix C API store path.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it returns a raw pointer. The caller must ensure that the pointer is not used beyond the lifetime of this `StorePath`.
    pub unsafe fn as_ptr(&self) -> *mut raw::StorePath {
        self.raw.as_ptr()
    }
}
impl Drop for StorePath {
    fn drop(&mut self) {
        unsafe {
            raw::store_path_free(self.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(nix_at_least = "2.33")]
    use super::StorePath;

    #[test]
    #[cfg(nix_at_least = "2.26" /* get_storedir */)]
    fn store_path_name() {
        let mut store = crate::store::Store::open(Some("dummy://"), []).unwrap();
        let store_dir = store.get_storedir().unwrap();
        let store_path_string =
            format!("{store_dir}/rdd4pnr4x9rqc9wgbibhngv217w2xvxl-bash-interactive-5.2p26");
        let store_path = store.parse_store_path(store_path_string.as_str()).unwrap();
        assert_eq!(store_path.name().unwrap(), "bash-interactive-5.2p26");
    }

    #[test]
    #[cfg(nix_at_least = "2.33")]
    fn store_path_parts_roundtrip() {
        let mut store = crate::store::Store::open(Some("dummy://"), []).unwrap();
        let original = store
            .parse_store_path("/nix/store/rdd4pnr4x9rqc9wgbibhngv217w2xvxl-bash-interactive-5.2p26")
            .unwrap();
        let rebuilt =
            StorePath::from_parts(original.hash().unwrap(), &original.name().unwrap()).unwrap();

        assert_eq!(rebuilt.hash().unwrap(), original.hash().unwrap());
        assert_eq!(rebuilt.name().unwrap(), original.name().unwrap());
    }
}
