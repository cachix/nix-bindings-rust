use std::{ffi::CString, os::raw::c_char, ptr::NonNull};

use anyhow::{Context as _, Result};
use nix_bindings_bindgen_raw as raw;
use nix_bindings_expr::eval_state::EvalState;
use nix_bindings_fetchers::FetchersSettings;
use nix_bindings_store::store::Store;
use nix_bindings_util::{
    context::{self, Context},
    result_string_init,
    string_return::{callback_get_result_string, callback_get_result_string_data},
};

/// Store settings for the flakes feature.
pub struct FlakeSettings {
    pub(crate) ptr: *mut raw::flake_settings,
}
impl Drop for FlakeSettings {
    fn drop(&mut self) {
        unsafe {
            raw::flake_settings_free(self.ptr);
        }
    }
}
impl FlakeSettings {
    pub fn new() -> Result<Self> {
        let mut ctx = Context::new();
        let s = unsafe { context::check_call!(raw::flake_settings_new(&mut ctx)) }?;
        Ok(FlakeSettings { ptr: s })
    }
    fn add_to_eval_state_builder(
        &self,
        builder: &mut nix_bindings_expr::eval_state::EvalStateBuilder,
    ) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_settings_add_to_eval_state_builder(
                &mut ctx,
                self.ptr,
                builder.raw_ptr()
            ))
        }?;
        Ok(())
    }
}

pub trait EvalStateBuilderExt {
    /// Configures the eval state to provide flakes features such as `builtins.getFlake`.
    fn flakes(self, settings: &FlakeSettings) -> Result<nix_bindings_expr::eval_state::EvalStateBuilder>;
}
impl EvalStateBuilderExt for nix_bindings_expr::eval_state::EvalStateBuilder {
    /// Configures the eval state to provide flakes features such as `builtins.getFlake`.
    fn flakes(
        mut self,
        settings: &FlakeSettings,
    ) -> Result<nix_bindings_expr::eval_state::EvalStateBuilder> {
        settings.add_to_eval_state_builder(&mut self)?;
        Ok(self)
    }
}

/// Parameters for parsing a flake reference.
pub struct FlakeReferenceParseFlags {
    pub(crate) ptr: NonNull<raw::flake_reference_parse_flags>,
}
impl Drop for FlakeReferenceParseFlags {
    fn drop(&mut self) {
        unsafe {
            raw::flake_reference_parse_flags_free(self.ptr.as_ptr());
        }
    }
}
impl FlakeReferenceParseFlags {
    pub fn new(settings: &FlakeSettings) -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            context::check_call!(raw::flake_reference_parse_flags_new(&mut ctx, settings.ptr))
        }?;
        let ptr = NonNull::new(ptr)
            .context("flake_reference_parse_flags_new unexpectedly returned null")?;
        Ok(FlakeReferenceParseFlags { ptr })
    }
    /// Sets the [base directory](https://nix.dev/manual/nix/latest/glossary#gloss-base-directory)
    /// for resolving local flake references.
    pub fn set_base_directory(&mut self, base_directory: &str) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_reference_parse_flags_set_base_directory(
                &mut ctx,
                self.ptr.as_ptr(),
                base_directory.as_ptr() as *const c_char,
                base_directory.len()
            ))
        }?;
        Ok(())
    }

    /// Enables preserving relative paths in flake references.
    ///
    /// When enabled, relative paths like `./` or `../` are preserved as-is
    /// in the parsed flake reference instead of being resolved to absolute paths.
    /// This allows the locking mechanism to resolve them later using the source path context.
    pub fn set_preserve_relative_paths(&mut self, preserve: bool) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_reference_parse_flags_set_preserve_relative_paths(
                &mut ctx,
                self.ptr.as_ptr(),
                preserve
            ))
        }?;
        Ok(())
    }
}

pub struct FlakeReference {
    pub(crate) ptr: NonNull<raw::flake_reference>,
}
impl Drop for FlakeReference {
    fn drop(&mut self) {
        unsafe {
            raw::flake_reference_free(self.ptr.as_ptr());
        }
    }
}
impl FlakeReference {
    /// Parse a flake reference from a string.
    /// The string must be a valid flake reference, such as `github:owner/repo`.
    /// It may also be suffixed with a `#` and a fragment, such as `github:owner/repo#something`,
    /// in which case, the returned string will contain the fragment.
    pub fn parse_with_fragment(
        fetch_settings: &FetchersSettings,
        flake_settings: &FlakeSettings,
        flags: &FlakeReferenceParseFlags,
        reference: &str,
    ) -> Result<(FlakeReference, String)> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        let mut ptr: *mut raw::flake_reference = std::ptr::null_mut();
        unsafe {
            context::check_call!(raw::flake_reference_and_fragment_from_string(
                &mut ctx,
                fetch_settings.raw_ptr(),
                flake_settings.ptr,
                flags.ptr.as_ptr(),
                reference.as_ptr() as *const c_char,
                reference.len(),
                // pointer to ptr
                &mut ptr,
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        let ptr = NonNull::new(ptr)
            .context("flake_reference_and_fragment_from_string unexpectedly returned null")?;
        Ok((FlakeReference { ptr: ptr }, r?))
    }
}

/// Parameters that affect the locking of a flake.
pub struct FlakeLockFlags {
    pub(crate) ptr: *mut raw::flake_lock_flags,
}
impl Drop for FlakeLockFlags {
    fn drop(&mut self) {
        unsafe {
            raw::flake_lock_flags_free(self.ptr);
        }
    }
}
impl FlakeLockFlags {
    pub fn new(settings: &FlakeSettings) -> Result<Self> {
        let mut ctx = Context::new();
        let s = unsafe { context::check_call!(raw::flake_lock_flags_new(&mut ctx, settings.ptr)) }?;
        Ok(FlakeLockFlags { ptr: s })
    }
    /// Configures [LockedFlake::lock] to make incremental changes to the lock file as needed. Changes are written to file.
    pub fn set_mode_write_as_needed(&mut self) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_set_mode_write_as_needed(
                &mut ctx, self.ptr
            ))
        }?;
        Ok(())
    }
    /// Make [LockedFlake::lock] check if the lock file is up to date. If not, an error is returned.
    pub fn set_mode_check(&mut self) -> Result<()> {
        let mut ctx = Context::new();
        unsafe { context::check_call!(raw::flake_lock_flags_set_mode_check(&mut ctx, self.ptr)) }?;
        Ok(())
    }
    /// Like `set_mode_write_as_needed`, but does not write to the lock file.
    pub fn set_mode_virtual(&mut self) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_set_mode_virtual(&mut ctx, self.ptr))
        }?;
        Ok(())
    }
    /// Adds an input override to the lock file that will be produced. The [LockedFlake::lock] operation will not write to the lock file.
    pub fn add_input_override(
        &mut self,
        override_path: &str,
        override_ref: &FlakeReference,
    ) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_add_input_override(
                &mut ctx,
                self.ptr,
                CString::new(override_path)
                    .context("Failed to create CString for override_path")?
                    .as_ptr(),
                override_ref.ptr.as_ptr()
            ))
        }?;
        Ok(())
    }
    /// Marks an input for update, ignoring its existing lock entry
    pub fn add_input_update(&mut self, input_path: &str) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_add_input_update(
                &mut ctx,
                self.ptr,
                CString::new(input_path)
                    .context("Failed to create CString for input_path")?
                    .as_ptr(),
                input_path.len()
            ))
        }?;
        Ok(())
    }

    /// Enable or disable registry lookups for flake input resolution.
    ///
    /// When enabled, indirect flake references like `nixpkgs` or `flake:nixpkgs`
    /// can be resolved through the flake registry. When disabled (the default),
    /// such references will cause an error.
    pub fn set_use_registries(&mut self, use_registries: bool) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_set_use_registries(
                &mut ctx,
                self.ptr,
                use_registries
            ))
        }?;
        Ok(())
    }

    /// Set recreateLockFile flag to re-resolve all inputs from scratch.
    ///
    /// When enabled, the lock file is recreated from scratch, ignoring all
    /// existing locks. This is the equivalent of `nix flake update` with no
    /// specific input arguments.
    pub fn set_recreate_lock_file(&mut self, recreate: bool) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_lock_flags_set_recreate_lock_file(
                &mut ctx,
                self.ptr,
                recreate
            ))
        }?;
        Ok(())
    }
}

pub struct LockedFlake {
    pub(crate) ptr: NonNull<raw::locked_flake>,
}
impl Drop for LockedFlake {
    fn drop(&mut self) {
        unsafe {
            raw::locked_flake_free(self.ptr.as_ptr());
        }
    }
}
impl LockedFlake {
    pub fn lock(
        fetch_settings: &FetchersSettings,
        flake_settings: &FlakeSettings,
        eval_state: &EvalState,
        flags: &FlakeLockFlags,
        flake_ref: &FlakeReference,
    ) -> Result<LockedFlake> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            context::check_call!(raw::flake_lock(
                &mut ctx,
                fetch_settings.raw_ptr(),
                flake_settings.ptr,
                eval_state.raw_ptr(),
                flags.ptr,
                flake_ref.ptr.as_ptr()
            ))
        }?;
        let ptr = NonNull::new(ptr).context("flake_lock unexpectedly returned null")?;
        Ok(LockedFlake { ptr })
    }

    /// Returns the outputs of the flake - the result of calling the `outputs` attribute.
    pub fn outputs(
        &self,
        flake_settings: &FlakeSettings,
        eval_state: &mut EvalState,
    ) -> Result<nix_bindings_expr::value::Value> {
        let mut ctx = Context::new();
        unsafe {
            let r = context::check_call!(raw::locked_flake_get_output_attrs(
                &mut ctx,
                flake_settings.ptr,
                eval_state.raw_ptr(),
                self.ptr.as_ptr()
            ))?;
            Ok(nix_bindings_expr::value::__private::raw_value_new(r))
        }
    }
}

/// A single flake input specification
pub struct FlakeInput {
    pub(crate) ptr: NonNull<raw::flake_input>,
}
impl Drop for FlakeInput {
    fn drop(&mut self) {
        unsafe {
            raw::flake_input_free(self.ptr.as_ptr());
        }
    }
}
impl FlakeInput {
    /// Create a new flake input from a flake reference
    pub fn new(flake_ref: &FlakeReference, is_flake: bool) -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            context::check_call!(raw::flake_input_new(
                &mut ctx,
                flake_ref.ptr.as_ptr(),
                is_flake
            ))
        }?;
        let ptr = NonNull::new(ptr).context("flake_input_new unexpectedly returned null")?;
        Ok(FlakeInput { ptr })
    }

    /// Set this input to follow another input's version
    pub fn set_follows(&mut self, follows_path: &str) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_input_set_follows(
                &mut ctx,
                self.ptr.as_ptr(),
                follows_path.as_ptr() as *const c_char,
                follows_path.len()
            ))
        }?;
        Ok(())
    }

    /// Set nested input overrides for this input
    ///
    /// This allows configuring how this input's own inputs should be resolved.
    /// For example, to make git-hooks.inputs.nixpkgs follow the root nixpkgs:
    /// ```ignore
    /// let mut overrides = FlakeInputs::new()?;
    /// // Use follows target name as placeholder reference (gets cleared by set_follows)
    /// let (nixpkgs_ref, _) = FlakeReference::parse(..., "nixpkgs")?;
    /// let mut nixpkgs_override = FlakeInput::new(&nixpkgs_ref, true)?;
    /// nixpkgs_override.set_follows("nixpkgs")?;
    /// overrides.add("nixpkgs", nixpkgs_override)?;
    /// git_hooks_input.set_overrides(overrides)?;
    /// ```
    pub fn set_overrides(&mut self, overrides: FlakeInputs) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_input_set_overrides(
                &mut ctx,
                self.ptr.as_ptr(),
                overrides.ptr.as_ptr()
            ))
        }?;
        // Ownership transferred to input, prevent double-free
        std::mem::forget(overrides);
        Ok(())
    }
}

/// A collection of flake inputs
pub struct FlakeInputs {
    pub(crate) ptr: NonNull<raw::flake_inputs>,
}
impl Drop for FlakeInputs {
    fn drop(&mut self) {
        unsafe {
            raw::flake_inputs_free(self.ptr.as_ptr());
        }
    }
}
impl FlakeInputs {
    /// Create a new empty collection of flake inputs
    pub fn new() -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe { context::check_call!(raw::flake_inputs_new(&mut ctx)) }?;
        let ptr = NonNull::new(ptr).context("flake_inputs_new unexpectedly returned null")?;
        Ok(FlakeInputs { ptr })
    }

    /// Add an input to the collection
    pub fn add(&mut self, name: &str, input: FlakeInput) -> Result<()> {
        let mut ctx = Context::new();
        unsafe {
            context::check_call!(raw::flake_inputs_add(
                &mut ctx,
                self.ptr.as_ptr(),
                name.as_ptr() as *const c_char,
                name.len(),
                input.ptr.as_ptr()
            ))
        }?;
        // Ownership transferred to inputs collection, prevent double-free
        std::mem::forget(input);
        Ok(())
    }
}

/// A flake lock file
pub struct LockFile {
    pub(crate) ptr: NonNull<raw::lock_file>,
}
impl Drop for LockFile {
    fn drop(&mut self) {
        unsafe {
            raw::lock_file_free(self.ptr.as_ptr());
        }
    }
}
impl LockFile {
    /// Create a new empty lock file
    pub fn new() -> Result<Self> {
        let mut ctx = Context::new();
        let ptr = unsafe { context::check_call!(raw::lock_file_new(&mut ctx)) }?;
        let ptr = NonNull::new(ptr).context("lock_file_new unexpectedly returned null")?;
        Ok(LockFile { ptr })
    }

    /// Parse a lock file from JSON string
    pub fn parse(
        fetch_settings: &FetchersSettings,
        content: &str,
        source_path: Option<&str>,
    ) -> Result<Self> {
        let mut ctx = Context::new();
        let (src_ptr, src_len) = match source_path {
            Some(s) => (s.as_ptr() as *const c_char, s.len()),
            None => (std::ptr::null(), 0),
        };
        let ptr = unsafe {
            context::check_call!(raw::lock_file_parse(
                &mut ctx,
                fetch_settings.raw_ptr(),
                content.as_ptr() as *const c_char,
                content.len(),
                src_ptr,
                src_len
            ))
        }?;
        let ptr = NonNull::new(ptr).context("lock_file_parse unexpectedly returned null")?;
        Ok(LockFile { ptr })
    }

    /// Convert lock file to JSON string
    pub fn to_string(&self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_to_string(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        r
    }

    /// Compare two lock files for equality
    pub fn equals(&self, other: &LockFile) -> Result<bool> {
        let mut ctx = Context::new();
        let mut are_equal = false;
        unsafe {
            context::check_call!(raw::lock_file_equals(
                &mut ctx,
                self.ptr.as_ptr(),
                other.ptr.as_ptr(),
                &mut are_equal
            ))
        }?;
        Ok(are_equal)
    }

    /// Generate a human-readable diff showing what changed between two lock files
    ///
    /// Returns a string with ANSI color codes showing added inputs (green),
    /// removed inputs (red), and updated inputs (bold).
    pub fn diff(&self, other: &LockFile) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_diff(
                &mut ctx,
                self.ptr.as_ptr(),
                other.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        r
    }

    /// Check if this lock file has any changes compared to another
    pub fn has_changes(&self, other: &LockFile) -> Result<bool> {
        Ok(!self.equals(other)?)
    }

    /// Create an iterator over all inputs in this lock file
    pub fn inputs_iterator(&self) -> Result<LockFileInputsIterator> {
        let mut ctx = Context::new();
        let ptr = unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_new(
                &mut ctx,
                self.ptr.as_ptr()
            ))
        }?;
        let ptr = NonNull::new(ptr)
            .context("lock_file_inputs_iterator_new unexpectedly returned null")?;
        Ok(LockFileInputsIterator { ptr })
    }

    /// Get the first unlocked input in this lock file
    ///
    /// Returns the flake reference of the first input that is not fully locked,
    /// or `None` if all inputs are locked.
    ///
    /// An input is considered locked based on its type:
    /// - git/mercurial: has a revision
    /// - github/gitlab/sourcehut: has a revision (and narHash if `allow-dirty-locks` is disabled)
    /// - path/tarball/file: has a narHash
    ///
    /// The `allow-dirty-locks` setting in `fetch_settings` affects this check:
    /// when enabled, inputs with a narHash are considered locked even without a revision.
    pub fn get_unlocked_input(&self, fetch_settings: &FetchersSettings) -> Result<Option<String>> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_get_unlocked_input(
                &mut ctx,
                fetch_settings.raw_ptr(),
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        let s = r?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }
}

/// Iterator over inputs in a lock file
pub struct LockFileInputsIterator {
    ptr: NonNull<raw::lock_file_inputs_iterator>,
}

impl Drop for LockFileInputsIterator {
    fn drop(&mut self) {
        unsafe {
            raw::lock_file_inputs_iterator_free(self.ptr.as_ptr());
        }
    }
}

impl LockFileInputsIterator {
    /// Advance to the next input and return true if valid, false if at end
    pub fn next(&mut self) -> bool {
        unsafe { raw::lock_file_inputs_iterator_next(self.ptr.as_ptr()) }
    }

    /// Get the attribute path of the current input (e.g., "nixpkgs" or "nix/nixpkgs")
    pub fn attr_path(&self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_get_attr_path(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        r
    }

    /// Get the locked flake reference of the current input as a string
    /// For example: "github:NixOS/nixpkgs/6a08e6bb4e46ff7fcbb53d409b253f6bad8a28ce"
    pub fn locked_ref(&self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_get_locked_ref(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        r
    }

    /// Get the original flake reference of the current input as a string
    pub fn original_ref(&self) -> Result<String> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_get_original_ref(
                &mut ctx,
                self.ptr.as_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        r
    }

    /// Check if the current input is locked
    ///
    /// An input is considered "locked" based on its type:
    /// - git/mercurial: has a revision
    /// - github/gitlab/sourcehut: has a revision and narHash
    /// - path/tarball/file: has a narHash
    ///
    /// Note: This does not consider the `allow-dirty-locks` setting.
    /// For that behavior, use [`LockFile::get_unlocked_input`] instead.
    ///
    /// For "follows" inputs, this returns true since they inherit locking from their target.
    pub fn is_locked(&self, fetch_settings: &FetchersSettings) -> Result<bool> {
        let mut ctx = Context::new();
        let mut result = false;
        unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_is_locked(
                &mut ctx,
                fetch_settings.raw_ptr(),
                self.ptr.as_ptr(),
                &mut result
            ))
        }?;
        Ok(result)
    }

    /// Get the fingerprint of the current input
    ///
    /// Returns a content identifier for the locked input that can be used to detect changes.
    /// The fingerprint format varies by input type:
    /// - git/github/mercurial: the revision hash (e.g., "abc123...")
    /// - tarball/path: the narHash in SRI format (e.g., "sha256-abc...")
    ///
    /// For "follows" inputs (InputAttrPath), returns `None` since they inherit
    /// their fingerprint from the target input.
    pub fn fingerprint(&self, fetch_settings: &FetchersSettings, store: &Store) -> Result<Option<String>> {
        let mut ctx = Context::new();
        let mut r = result_string_init!();
        unsafe {
            context::check_call!(raw::lock_file_inputs_iterator_get_fingerprint(
                &mut ctx,
                self.ptr.as_ptr(),
                fetch_settings.raw_ptr(),
                store.raw_ptr(),
                Some(callback_get_result_string),
                callback_get_result_string_data(&mut r)
            ))
        }?;
        let s = r?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }
}

/// Lock inputs without reading a top-level flake.nix
///
/// This function takes manually-constructed flake inputs and computes
/// a lock file. EvalState is still required because transitive flake
/// inputs need to be fetched and evaluated.
pub fn lock_inputs(
    fetch_settings: &FetchersSettings,
    flake_settings: &FlakeSettings,
    eval_state: &EvalState,
    inputs: FlakeInputs,
    source_path: &str,
    old_lock_file: Option<&LockFile>,
    flags: &FlakeLockFlags,
) -> Result<LockFile> {
    let mut ctx = Context::new();
    let old_lock_ptr = match old_lock_file {
        Some(lf) => lf.ptr.as_ptr(),
        None => std::ptr::null_mut(),
    };
    let ptr = unsafe {
        context::check_call!(raw::flake_lock_inputs(
            &mut ctx,
            fetch_settings.raw_ptr(),
            flake_settings.ptr,
            eval_state.raw_ptr(),
            inputs.ptr.as_ptr(),
            source_path.as_ptr() as *const c_char,
            source_path.len(),
            old_lock_ptr,
            flags.ptr
        ))
    }?;
    let ptr = NonNull::new(ptr).context("flake_lock_inputs unexpectedly returned null")?;
    Ok(LockFile { ptr })
}

/// Locking mode for flake inputs
#[derive(Debug, Clone, Copy)]
pub enum LockMode {
    /// Compute locks and write to disk if changes are needed
    WriteAsNeeded,
    /// Compute locks in memory only, don't write to disk
    Virtual,
    /// Check if locks are up-to-date, fail if updates are needed
    Check,
}

/// High-level builder for locking flake inputs with fluent API
///
/// Provides a convenient way to lock flake inputs with support for:
/// - Batch input updates (ignore existing locks)
/// - Batch input overrides (replace with specific references)
/// - Multiple locking modes
///
/// # Example
/// ```ignore
/// let lock_file = InputsLocker::new(&flake_settings)
///     .with_inputs(inputs)
///     .source_path("/path/to/flake")
///     .old_lock_file(&existing_lock)
///     .update_inputs(&["nixpkgs", "rust-overlay"])
///     .override_input("custom", &custom_ref)
///     .mode(LockMode::WriteAsNeeded)
///     .lock(&fetch_settings, &eval_state)?;
/// ```
///
/// # Note
/// Input follows must be configured on individual [FlakeInput] objects before adding them
/// to the collection, as the C API does not support modifying inputs after they're added.
pub struct InputsLocker<'a> {
    flake_settings: &'a FlakeSettings,
    inputs: Option<FlakeInputs>,
    source_path: Option<String>,
    old_lock: Option<&'a LockFile>,
    updates: Vec<String>,
    update_all: bool,
    overrides: Vec<(String, &'a FlakeReference)>,
    mode: LockMode,
    use_registries: bool,
}

impl<'a> InputsLocker<'a> {
    /// Create a new InputsLocker with the given flake settings
    pub fn new(flake_settings: &'a FlakeSettings) -> Self {
        Self {
            flake_settings,
            inputs: None,
            source_path: None,
            old_lock: None,
            updates: Vec::new(),
            update_all: false,
            overrides: Vec::new(),
            mode: LockMode::WriteAsNeeded,
            use_registries: false,
        }
    }

    /// Set the inputs to lock
    pub fn with_inputs(mut self, inputs: FlakeInputs) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Set the source path for resolving relative references
    pub fn source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Set the old lock file to use as basis for incremental updates
    pub fn old_lock_file(mut self, lock: &'a LockFile) -> Self {
        self.old_lock = Some(lock);
        self
    }

    /// Mark a single input for update (ignore its existing lock)
    pub fn update_input(mut self, input: impl Into<String>) -> Self {
        self.updates.push(input.into());
        self
    }

    /// Mark multiple inputs for update (batch operation)
    pub fn update_inputs<I, S>(mut self, inputs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.updates.extend(inputs.into_iter().map(|s| s.into()));
        self
    }

    /// Recreate the lock file from scratch, re-resolving all inputs.
    ///
    /// This is the equivalent of `nix flake update` with no specific input arguments.
    pub fn update_all(mut self) -> Self {
        self.update_all = true;
        self
    }

    /// Override a single input with a specific flake reference
    pub fn override_input(mut self, path: impl Into<String>, ref_: &'a FlakeReference) -> Self {
        self.overrides.push((path.into(), ref_));
        self
    }

    /// Set multiple input overrides (batch operation)
    pub fn overrides<I>(mut self, overrides: I) -> Self
    where
        I: IntoIterator<Item = (String, &'a FlakeReference)>,
    {
        self.overrides.extend(overrides);
        self
    }

    /// Set the locking mode
    pub fn mode(mut self, mode: LockMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable or disable registry lookups for flake input resolution.
    ///
    /// When enabled, indirect flake references like `nixpkgs` or `flake:nixpkgs`
    /// can be resolved through the flake registry. When disabled (the default),
    /// such references will cause an error.
    pub fn use_registries(mut self, use_registries: bool) -> Self {
        self.use_registries = use_registries;
        self
    }

    /// Execute the locking operation with all accumulated settings
    pub fn lock(
        self,
        fetch_settings: &FetchersSettings,
        eval_state: &EvalState,
    ) -> Result<LockFile> {
        // Create flags with the specified mode
        let mut flags = FlakeLockFlags::new(self.flake_settings)?;

        match self.mode {
            LockMode::WriteAsNeeded => flags.set_mode_write_as_needed()?,
            LockMode::Virtual => flags.set_mode_virtual()?,
            LockMode::Check => flags.set_mode_check()?,
        }

        // Set registry usage
        flags.set_use_registries(self.use_registries)?;

        // Set recreate lock file if updating all inputs
        if self.update_all {
            flags.set_recreate_lock_file(true)?;
        }

        // Add all input updates
        for input_path in self.updates {
            flags.add_input_update(&input_path)?;
        }

        // Add all input overrides
        for (input_path, flake_ref) in self.overrides {
            flags.add_input_override(&input_path, flake_ref)?;
        }

        // Lock the inputs
        lock_inputs(
            fetch_settings,
            self.flake_settings,
            eval_state,
            self.inputs.context("inputs must be set")?,
            &self.source_path.context("source_path must be set")?,
            self.old_lock,
            &flags,
        )
    }
}

#[cfg(test)]
mod tests {
    use nix_bindings_expr::eval_state::{gc_register_my_thread, EvalStateBuilder};
    use nix_bindings_store::store::Store;

    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init() {
        // Only set experimental-features once to minimize the window where
        // concurrent Nix operations might read the setting while it's being modified
        INIT.call_once(|| {
            nix_bindings_expr::eval_state::init().unwrap();
            nix_bindings_util::settings::set("experimental-features", "flakes").unwrap();
        });
    }

    #[test]
    fn flake_settings_getflake_exists() {
        init();
        let gc_registration = gc_register_my_thread();
        let store = Store::open(None, []).unwrap();
        let mut eval_state = EvalStateBuilder::new(store)
            .unwrap()
            .flakes(&FlakeSettings::new().unwrap())
            .unwrap()
            .build()
            .unwrap();

        let v = eval_state
            .eval_from_string("builtins?getFlake", "<test>")
            .unwrap();

        let b = eval_state.require_bool(&v).unwrap();

        assert_eq!(b, true);

        drop(gc_registration);
    }

    #[test]
    fn flake_lock_load_flake() {
        init();
        let gc_registration = gc_register_my_thread();
        let store = Store::open(None, []).unwrap();
        let fetchers_settings = FetchersSettings::new().unwrap();
        let flake_settings = FlakeSettings::new().unwrap();
        let mut eval_state = EvalStateBuilder::new(store)
            .unwrap()
            .flakes(&flake_settings)
            .unwrap()
            .build()
            .unwrap();

        let tmp_dir = tempfile::tempdir().unwrap();

        // Create flake.nix
        let flake_nix = tmp_dir.path().join("flake.nix");
        std::fs::write(
            &flake_nix,
            r#"
{
    outputs = { ... }: {
        hello = "potato";
    };
}
        "#,
        )
        .unwrap();

        let flake_lock_flags = FlakeLockFlags::new(&flake_settings).unwrap();

        let (flake_ref, fragment) = FlakeReference::parse_with_fragment(
            &fetchers_settings,
            &flake_settings,
            &FlakeReferenceParseFlags::new(&flake_settings).unwrap(),
            &format!("path:{}#subthing", tmp_dir.path().display()),
        )
        .unwrap();

        assert_eq!(fragment, "subthing");

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();

        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();

        assert_eq!(hello, "potato");

        drop(fetchers_settings);
        drop(tmp_dir);
        drop(gc_registration);
    }

    #[test]
    fn flake_lock_load_flake_with_flags() {
        init();
        let gc_registration = gc_register_my_thread();
        let store = Store::open(None, []).unwrap();
        let fetchers_settings = FetchersSettings::new().unwrap();
        let flake_settings = FlakeSettings::new().unwrap();
        let mut eval_state = EvalStateBuilder::new(store)
            .unwrap()
            .flakes(&flake_settings)
            .unwrap()
            .build()
            .unwrap();

        let tmp_dir = tempfile::tempdir().unwrap();

        let flake_dir_a = tmp_dir.path().join("a");
        let flake_dir_b = tmp_dir.path().join("b");
        let flake_dir_c = tmp_dir.path().join("c");

        std::fs::create_dir_all(&flake_dir_a).unwrap();
        std::fs::create_dir_all(&flake_dir_b).unwrap();
        std::fs::create_dir_all(&flake_dir_c).unwrap();

        let flake_dir_a_str = flake_dir_a.to_str().unwrap();
        let flake_dir_c_str = flake_dir_c.to_str().unwrap();
        assert!(!flake_dir_a_str.is_empty());
        assert!(!flake_dir_c_str.is_empty());

        // a
        std::fs::write(
            &tmp_dir.path().join("a/flake.nix"),
            r#"
            {
                inputs.b.url = "@flake_dir_b@";
                outputs = { b, ... }: {
                    hello = b.hello;
                };
            }
            "#
            .replace("@flake_dir_b@", flake_dir_b.to_str().unwrap()),
        )
        .unwrap();

        // b
        std::fs::write(
            &tmp_dir.path().join("b/flake.nix"),
            r#"
            {
                outputs = { ... }: {
                    hello = "BOB";
                };
            }
            "#,
        )
        .unwrap();

        // c
        std::fs::write(
            &tmp_dir.path().join("c/flake.nix"),
            r#"
            {
                outputs = { ... }: {
                    hello = "Claire";
                };
            }
            "#,
        )
        .unwrap();

        let mut flake_lock_flags = FlakeLockFlags::new(&flake_settings).unwrap();

        let mut flake_reference_parse_flags =
            FlakeReferenceParseFlags::new(&flake_settings).unwrap();

        flake_reference_parse_flags
            .set_base_directory(tmp_dir.path().to_str().unwrap())
            .unwrap();

        let (flake_ref_a, fragment) = FlakeReference::parse_with_fragment(
            &fetchers_settings,
            &flake_settings,
            &flake_reference_parse_flags,
            &format!("path:{}", &flake_dir_a_str),
        )
        .unwrap();

        assert_eq!(fragment, "");

        // Step 1: Do not update (check), fails

        flake_lock_flags.set_mode_check().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        );
        // Has not been locked and would need to write a lock file.
        assert!(locked_flake.is_err());
        let saved_err = match locked_flake {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => e,
        };

        // Step 2: Update but do not write, succeeds
        flake_lock_flags.set_mode_virtual().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();

        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();

        assert_eq!(hello, "BOB");

        // Step 3: The lock was not written, so Step 1 would fail again

        flake_lock_flags.set_mode_check().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        );
        // Has not been locked and would need to write a lock file.
        assert!(locked_flake.is_err());
        match locked_flake {
            Ok(_) => panic!("Expected error, but got Ok"),
            Err(e) => {
                assert_eq!(e.to_string(), saved_err.to_string());
            }
        };

        // Step 4: Update and write, succeeds

        flake_lock_flags.set_mode_write_as_needed().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();
        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();
        assert_eq!(hello, "BOB");

        // Step 5: Lock was written, so Step 1 succeeds

        flake_lock_flags.set_mode_check().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();
        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();
        assert_eq!(hello, "BOB");

        // Step 6: Lock with override, do not write

        // This shouldn't matter; write_as_needed will be overridden
        flake_lock_flags.set_mode_write_as_needed().unwrap();

        let (flake_ref_c, fragment) = FlakeReference::parse_with_fragment(
            &fetchers_settings,
            &flake_settings,
            &flake_reference_parse_flags,
            &format!("path:{}", &flake_dir_c_str),
        )
        .unwrap();
        assert_eq!(fragment, "");

        flake_lock_flags
            .add_input_override("b", &flake_ref_c)
            .unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();
        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();
        assert_eq!(hello, "Claire");

        // Can't delete overrides, so trash it
        let mut flake_lock_flags = FlakeLockFlags::new(&flake_settings).unwrap();

        // Step 7: Override was not written; lock still points to b

        flake_lock_flags.set_mode_check().unwrap();

        let locked_flake = LockedFlake::lock(
            &fetchers_settings,
            &flake_settings,
            &eval_state,
            &flake_lock_flags,
            &flake_ref_a,
        )
        .unwrap();

        let outputs = locked_flake
            .outputs(&flake_settings, &mut eval_state)
            .unwrap();
        let hello = eval_state.require_attrs_select(&outputs, &"hello").unwrap();
        let hello = eval_state.require_string(&hello).unwrap();
        assert_eq!(hello, "BOB");

        drop(fetchers_settings);
        drop(tmp_dir);
        drop(gc_registration);
    }
}
