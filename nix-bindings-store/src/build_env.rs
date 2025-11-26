//! Build environment extraction and manipulation.
//!
//! A `BuildEnvironment` represents the environment variables, bash functions,
//! and structured attributes extracted from a Nix derivation.

use anyhow::Result;
use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::context::Context;
use nix_bindings_util::string_return::{
    callback_get_result_string, callback_get_result_string_data,
};
use nix_bindings_util::{check_call, result_string_init};
use std::ffi::CString;
use std::ptr::NonNull;

use crate::path::StorePath;
use crate::store::Store;

/// A build environment extracted from a derivation.
///
/// A build environment contains:
/// - Environment variables (in various forms and export statuses)
/// - Bash function definitions
/// - Optionally, structured attributes (.attrs.json and .attrs.sh)
///
/// It can be parsed from JSON (e.g., from `nix print-dev-env --json`) and
/// serialized back to JSON or bash script format.
pub struct BuildEnvironment {
    ptr: NonNull<raw::build_env>,
}

impl BuildEnvironment {
    /// Create a new, empty BuildEnvironment.
    ///
    /// # Errors
    ///
    /// Returns an error if the allocation fails.
    pub fn new() -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe { raw::build_env_new(ctx.ptr()) };

        if ptr.is_null() {
            ctx.check_err()?;
            return Err(anyhow::anyhow!("Failed to allocate BuildEnvironment"));
        }

        Ok(BuildEnvironment {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Parse a BuildEnvironment from a JSON string.
    ///
    /// The JSON must have the structure produced by `nix print-dev-env --json`,
    /// with "variables", "bashFunctions", and optionally "structuredAttrs" keys.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON parsing fails or is invalid.
    pub fn parse_json(json: &str) -> Result<Self> {
        let json_cstr = CString::new(json)?;
        let mut ctx = Context::new();
        let ptr = unsafe { raw::build_env_parse_json(ctx.ptr(), json_cstr.as_ptr()) };

        if ptr.is_null() {
            ctx.check_err()?;
            return Err(anyhow::anyhow!(
                "Failed to parse BuildEnvironment from JSON"
            ));
        }

        Ok(BuildEnvironment {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Create a BuildEnvironment from a derivation store path.
    ///
    /// Extracts the environment variables and structured attributes from a
    /// derivation and creates a BuildEnvironment that represents the build
    /// environment for that derivation.
    ///
    /// Note: This only extracts the raw environment variables from the derivation.
    /// It does NOT run stdenv setup hooks, so variables like PKG_CONFIG_PATH that
    /// are computed by setup hooks will not be set. For the fully-expanded
    /// environment, use [`get_dev_environment`](Self::get_dev_environment) instead.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the derivation or extracting the environment fails.
    pub fn from_derivation(store: &Store, drv_path: &StorePath) -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            raw::build_env_from_derivation(ctx.ptr(), store.raw_ptr(), drv_path.as_ptr())
        };

        if ptr.is_null() {
            ctx.check_err()?;
            return Err(anyhow::anyhow!(
                "Failed to extract BuildEnvironment from derivation"
            ));
        }

        Ok(BuildEnvironment {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Get the fully-expanded development environment from a derivation.
    ///
    /// Unlike [`from_derivation`](Self::from_derivation) which only reads the raw `.drv` file,
    /// this function actually builds a modified derivation that runs the stdenv
    /// setup hooks and captures the resulting environment. This is equivalent to
    /// what `nix print-dev-env` does.
    ///
    /// The function:
    /// 1. Creates a modified derivation that runs a special script instead of the builder
    /// 2. Builds that derivation to capture the environment after setup hooks run
    /// 3. Parses the output JSON and returns the BuildEnvironment
    ///
    /// This is the recommended way to get the environment for interactive shell use,
    /// as it includes computed variables like PKG_CONFIG_PATH, PATH additions, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The derivation doesn't use bash as its builder
    /// - Building the environment derivation fails
    /// - Parsing the output fails
    pub fn get_dev_environment(store: &Store, drv_path: &StorePath) -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            raw::nix_get_dev_environment(ctx.ptr(), store.raw_ptr(), drv_path.as_ptr())
        };

        if ptr.is_null() {
            ctx.check_err()?;
            return Err(anyhow::anyhow!(
                "Failed to get dev environment from derivation"
            ));
        }

        Ok(BuildEnvironment {
            ptr: NonNull::new(ptr).unwrap(),
        })
    }

    /// Serialize this BuildEnvironment to JSON.
    ///
    /// The output JSON will have the same structure as accepted by
    /// [parse_json](Self::parse_json).
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    pub fn to_json(&mut self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();

        unsafe {
            check_call!(raw::build_env_to_json(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))?;
        }

        r
    }

    /// Serialize this BuildEnvironment to bash script format.
    ///
    /// Generates bash code that can be sourced to apply the build environment.
    /// This includes variable assignments, export statements, and function definitions.
    ///
    /// # Errors
    ///
    /// Returns an error if the serialization fails.
    pub fn to_bash(&mut self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();

        unsafe {
            check_call!(raw::build_env_to_bash(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))?;
        }

        r
    }

    /// Check if this environment provides structured attributes.
    ///
    /// Structured attributes are optional additions that provide alternative
    /// representations of the environment data.
    pub fn has_structured_attrs(&self) -> bool {
        unsafe { raw::build_env_has_structured_attrs(self.ptr.as_ptr()) }
    }

    /// Get the structured attributes JSON content.
    ///
    /// Only valid if [has_structured_attrs](Self::has_structured_attrs) returns true.
    ///
    /// # Errors
    ///
    /// Returns an error if the retrieval fails or structured attributes are not available.
    pub fn get_attrs_json(&mut self) -> Result<String> {
        if !self.has_structured_attrs() {
            return Err(anyhow::anyhow!(
                "BuildEnvironment does not have structured attributes"
            ));
        }

        let mut ctx = Context::new();
        let mut r = result_string_init!();

        unsafe {
            check_call!(raw::build_env_get_attrs_json(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))?;
        }

        r
    }

    /// Get the structured attributes shell script content.
    ///
    /// Only valid if [has_structured_attrs](Self::has_structured_attrs) returns true.
    ///
    /// # Errors
    ///
    /// Returns an error if the retrieval fails or structured attributes are not available.
    pub fn get_attrs_sh(&mut self) -> Result<String> {
        if !self.has_structured_attrs() {
            return Err(anyhow::anyhow!(
                "BuildEnvironment does not have structured attributes"
            ));
        }

        let mut ctx = Context::new();
        let mut r = result_string_init!();

        unsafe {
            check_call!(raw::build_env_get_attrs_sh(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))?;
        }

        r
    }
}

impl Drop for BuildEnvironment {
    fn drop(&mut self) {
        unsafe {
            raw::build_env_free(self.ptr.as_ptr());
        }
    }
}

impl Default for BuildEnvironment {
    fn default() -> Self {
        Self::new().expect("Failed to create default BuildEnvironment")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_env_new() {
        let result = BuildEnvironment::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_env_default() {
        let _env = BuildEnvironment::default();
        // Just verify it doesn't panic on creation and drop
    }

    #[test]
    fn test_build_env_has_structured_attrs_empty() {
        let env = BuildEnvironment::new().unwrap();
        // New empty environment should not have structured attrs
        assert!(!env.has_structured_attrs());
    }
}
