use anyhow::{Context as _, Result};
use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::check_call;
use nix_bindings_util::context::{self, Context};
use std::ptr::NonNull;

/// Fetcher settings for Nix fetcher operations.
///
/// Settings are automatically loaded from nix.conf files when created,
/// including access-tokens for authenticated fetchers like GitHub.
pub struct FetchersSettings {
    pub(crate) ptr: NonNull<raw::fetchers_settings>,
}
impl Drop for FetchersSettings {
    fn drop(&mut self) {
        unsafe {
            raw::fetchers_settings_free(self.ptr.as_ptr());
        }
    }
}
impl FetchersSettings {
    /// Create new fetcher settings, pre-populated from nix.conf files.
    ///
    /// This automatically loads settings from:
    /// - System config: `/etc/nix/nix.conf`
    /// - User config: `~/.config/nix/nix.conf`
    /// - Environment: `NIX_CONFIG`
    ///
    /// This includes settings like `access-tokens` for authenticated GitHub/GitLab access.
    pub fn new() -> Result<Self> {
        // Ensure libstore is initialized first (thread-safe via lazy_static).
        // This is required because the C API calls loadConfFile which depends
        // on global settings being initialized.
        nix_bindings_store::store::init()?;

        let mut ctx = Context::new();
        let ptr = unsafe { context::check_call!(raw::fetchers_settings_new(&mut ctx))? };
        Ok(FetchersSettings {
            ptr: NonNull::new(ptr).context("fetchers_settings_new unexpectedly returned null")?,
        })
    }

    pub fn raw_ptr(&self) -> *mut raw::fetchers_settings {
        self.ptr.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetchers_settings_new() {
        // Settings are now automatically loaded from nix.conf
        let _ = FetchersSettings::new().unwrap();
    }
}
