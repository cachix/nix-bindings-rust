//! Generic Logger API for observing Nix activities
//!
//! This module provides a type-safe wrapper around Nix's logging system,
//! allowing you to hook into build events, progress updates, and other activities.
//!
//! # Design
//!
//! The logger API is intentionally generic - you define closures that handle
//! different event types. The C API passes raw field data, so you interpret it
//! based on `result_type`.
//!
//! # Example
//!
//! ```ignore
//! use nix_bindings_expr::logger::ActivityLogger;
//!
//! let logger = ActivityLogger::new()
//!     .on_start(|id, desc, ty| {
//!         println!("[{}] {}", ty, desc);
//!     })
//!     .on_result(|id, result_type, field_count, field_types, int_values, string_values| {
//!         if result_type == "progress" && field_count >= 4 {
//!             let done = int_values[0];
//!             let expected = int_values[1];
//!             println!("Progress: {}/{}", done, expected);
//!         }
//!     })
//!     .register()?;
//! ```

use std::ffi::CStr;
use std::os::raw::c_void;
use std::sync::Arc;

use nix_bindings_bindgen_raw as raw;
use nix_bindings_util::context::Context;

/// A closure that handles activity start events
pub type OnActivityStart = Arc<dyn Fn(u64, &str, &str) + Send + Sync>;

/// A closure that handles activity stop events
pub type OnActivityStop = Arc<dyn Fn(u64) + Send + Sync>;

/// A closure that handles activity result/progress events
///
/// Arguments:
/// - `activity_id`: Unique identifier for the activity
/// - `result_type`: String describing the result type (e.g., "progress", "build-log-line")
/// - `field_count`: Number of fields
/// - `field_types`: Array of field types (0 = int64, 1 = string)
/// - `int_values`: Array of int64 values
/// - `string_values`: Array of C string pointers (NULL for non-string fields)
pub type OnActivityResult = Arc<dyn Fn(u64, &str, &[i32], &[i64], &[Option<&str>]) + Send + Sync>;

/// Builder for setting up activity callbacks
pub struct ActivityLoggerBuilder {
    on_start: Option<OnActivityStart>,
    on_stop: Option<OnActivityStop>,
    on_result: Option<OnActivityResult>,
}

impl ActivityLoggerBuilder {
    /// Create a new logger builder
    pub fn new() -> Self {
        ActivityLoggerBuilder {
            on_start: None,
            on_stop: None,
            on_result: None,
        }
    }

    /// Set the callback for activity start events
    pub fn on_start<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64, &str, &str) + Send + Sync + 'static,
    {
        self.on_start = Some(Arc::new(callback));
        self
    }

    /// Set the callback for activity stop events
    pub fn on_stop<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        self.on_stop = Some(Arc::new(callback));
        self
    }

    /// Set the callback for activity result/progress events
    pub fn on_result<F>(mut self, callback: F) -> Self
    where
        F: Fn(u64, &str, &[i32], &[i64], &[Option<&str>]) + Send + Sync + 'static,
    {
        self.on_result = Some(Arc::new(callback));
        self
    }

    /// Register the callbacks with Nix
    ///
    /// This must be called before any Nix operations that generate activities.
    pub fn register(self, context: &mut Context) -> anyhow::Result<ActivityLogger> {
        let data = Arc::new(LoggerCallbackData {
            on_start: self.on_start,
            on_stop: self.on_stop,
            on_result: self.on_result,
        });

        // Create raw pointers for C callbacks
        let data_ptr = Arc::into_raw(data.clone()) as *mut c_void;

        unsafe {
            // C callback that forwards to Rust closure for activity start
            extern "C" fn on_start_callback(
                activity_id: u64,
                description: *const std::os::raw::c_char,
                activity_type: *const std::os::raw::c_char,
                user_data: *mut c_void,
            ) {
                if let Some(data) = unsafe { (user_data as *mut LoggerCallbackData).as_ref() } {
                    if let Some(ref callback) = data.on_start {
                        if !description.is_null() && !activity_type.is_null() {
                            if let (Ok(desc), Ok(ty)) = unsafe {
                                (
                                    CStr::from_ptr(description).to_str(),
                                    CStr::from_ptr(activity_type).to_str(),
                                )
                            } {
                                callback(activity_id, desc, ty);
                            }
                        }
                    }
                }
            }

            // C callback for activity stop
            extern "C" fn on_stop_callback(activity_id: u64, user_data: *mut c_void) {
                if let Some(data) = unsafe { (user_data as *mut LoggerCallbackData).as_ref() } {
                    if let Some(ref callback) = data.on_stop {
                        callback(activity_id);
                    }
                }
            }

            // C callback for activity results
            extern "C" fn on_result_callback(
                activity_id: u64,
                result_type: *const std::os::raw::c_char,
                field_count: usize,
                field_types: *const i32,
                int_values: *const i64,
                string_values: *const *const std::os::raw::c_char,
                user_data: *mut c_void,
            ) {
                if let Some(data) = unsafe { (user_data as *mut LoggerCallbackData).as_ref() } {
                    if let Some(ref callback) = data.on_result {
                        if !result_type.is_null()
                            && !field_types.is_null()
                            && !int_values.is_null()
                            && !string_values.is_null()
                        {
                            unsafe {
                                if let Ok(result_type_str) = CStr::from_ptr(result_type).to_str() {
                                    // Convert C arrays to Rust slices
                                    let field_types_slice =
                                        std::slice::from_raw_parts(field_types, field_count);
                                    let int_values_slice =
                                        std::slice::from_raw_parts(int_values, field_count);

                                    // Convert C string pointers to Rust Options
                                    let mut string_values_vec = Vec::new();
                                    for i in 0..field_count {
                                        let ptr = *string_values.add(i);
                                        if ptr.is_null() {
                                            string_values_vec.push(None);
                                        } else if let Ok(s) = CStr::from_ptr(ptr).to_str() {
                                            string_values_vec.push(Some(s));
                                        } else {
                                            string_values_vec.push(None);
                                        }
                                    }

                                    callback(
                                        activity_id,
                                        result_type_str,
                                        field_types_slice,
                                        int_values_slice,
                                        &string_values_vec,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Call the C API
            let err = raw::set_logger_callbacks(
                context.ptr(),
                Some(on_start_callback),
                Some(on_stop_callback),
                Some(on_result_callback),
                data_ptr,
            );

            if err != raw::err_NIX_OK {
                // Clean up on error
                let _ = Arc::from_raw(data_ptr as *mut LoggerCallbackData);
                anyhow::bail!("set_logger_callbacks failed with error code {}", err);
            }
        }

        Ok(ActivityLogger { data })
    }
}

impl Default for ActivityLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal data passed through C callbacks
struct LoggerCallbackData {
    on_start: Option<OnActivityStart>,
    on_stop: Option<OnActivityStop>,
    on_result: Option<OnActivityResult>,
}

/// Active logger that holds callback data alive
///
/// # Lifetime Requirements
///
/// **IMPORTANT**: The returned `ActivityLogger` MUST be kept alive for the entire
/// duration that Nix may fire activity callbacks. Dropping this logger will free
/// the callback data, causing crashes if Nix fires events afterward.
///
/// # Example
///
/// ```ignore
/// fn main() -> Result<()> {
///     let mut context = Context::new();
///
///     // Keep logger alive for the entire program
///     let _logger = ActivityLoggerBuilder::new()
///         .on_start(|id, desc, ty| { println!("[{}] {}", ty, desc); })
///         .register(&mut context)?;
///
///     // Do Nix operations
///     // _logger stays alive until end of main()
///     Ok(())
/// }
/// ```
pub struct ActivityLogger {
    data: Arc<LoggerCallbackData>,
}

impl ActivityLogger {
    /// Create a new logger builder
    pub fn builder() -> ActivityLoggerBuilder {
        ActivityLoggerBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = ActivityLoggerBuilder::new()
            .on_start(|_id, _desc, _ty| {})
            .on_stop(|_id| {})
            .on_result(|_id, _ty, _ftypes, _ints, _strs| {});

        assert!(builder.on_start.is_some());
        assert!(builder.on_stop.is_some());
        assert!(builder.on_result.is_some());
    }
}
