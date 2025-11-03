# Nix Bindings Rust - FFI Functions and Exposed APIs

This document provides a comprehensive inventory of C FFI functions and capabilities currently bound in the nix-bindings-rust project.

## Project Structure

The project consists of 6 main crates that wrap the Nix C API:

1. **nix-bindings-bindgen-raw** - Automatically generated bindings from Nix C headers
2. **nix-bindings-util** - Error handling and utility functions
3. **nix-bindings-store** - Store operations and derivation management
4. **nix-bindings-expr** - Expression evaluation and value manipulation
5. **nix-bindings-flake** - Flake locking and reference parsing
6. **nix-bindings-fetchers** - Fetcher settings management
7. **nix-cmd** - Command utilities and REPL support

---

## 1. nix-bindings-store

### 1.1 Store Management

**Exposed Rust API in `Store` struct:**

#### Store Lifecycle
- `Store::open(url: Option<&str>, params)` → `Result<Store>`
  - Wraps: `nix_store_open()`
  - Caches store instances to prevent duplicate opens

- `Store::get_uri()` → `Result<String>`
  - Wraps: `nix_store_get_uri()`

- `Store::get_storedir()` → `Result<String>` (Nix 2.26+)
  - Wraps: `nix_store_get_storedir()`

- `Store::is_trusted_client()` → `TrustedFlag`
  - Wraps: `nix_store_is_trusted_client()`
  - Checks if the client connection is trusted by the store
  - Returns `TrustedFlag::Trusted` if client is trusted
  - Returns `TrustedFlag::NotTrusted` if client is not trusted
  - Returns `TrustedFlag::Unknown` if trust status is not applicable (e.g., HTTP binary caches)

#### Store Path Operations
- `Store::parse_store_path(path: &str)` → `Result<StorePath>`
  - Wraps: `nix_store_parse_path()`

- `Store::real_path(path: &StorePath)` → `Result<String>`
  - Wraps: `nix_store_real_path()`

#### Derivation Operations (Nix 2.33+)
- `Store::derivation_from_json(json: &str)` → `Result<Derivation>`
  - Wraps: `nix_derivation_from_json()`
  - Parses JSON string following Nix derivation schema

- `Store::add_derivation(drv: &Derivation)` → `Result<StorePath>`
  - Wraps: `nix_add_derivation()`
  - Registers derivation in store, returns .drv file path

- `Store::realise(path: &StorePath)` → `Result<BTreeMap<String, StorePath>>`
  - Wraps: `nix_store_realise()`
  - Builds a derivation and returns output paths
  - Returns ordered map of output names to store paths

#### Closure Operations (Nix 2.33+)
- `Store::get_fs_closure(store_path, flip_direction, include_outputs, include_derivers)` → `Result<Vec<StorePath>>`
  - Wraps: `nix_store_get_fs_closure()`
  - Computes filesystem closure (dependency graph) with fine-grained control

#### Garbage Collection
- `Store::collect_garbage(action, paths_to_delete, ignore_liveness, max_freed)` → `Result<(Vec<StorePath>, u64)>`
  - Wraps: `nix_bindings_store_collect_garbage()`
  - Supports 4 modes: ReturnLive, ReturnDead, DeleteDead, DeleteSpecific
  - Returns freed paths and bytes freed

#### GC Roots Management
- `Store::add_perm_root(path: &StorePath, gc_root: &Path)` → `Result<()>`
  - Wraps: `nix_bindings_store_add_perm_root()`
  - Creates persistent GC root symlink

- `Store::add_indirect_root(symlink_path: &Path)` → `Result<()>`
  - Wraps: `nix_bindings_store_add_indirect_root()`
  - Adds weak reference GC root

#### Substituter Management
- `Store::add_substituter(uri: &str)` → `Result<()>`
  - Wraps: `nix_bindings_store_add_substituter()`
  - Adds substituter at runtime

- `Store::remove_substituter(uri: &str)` → `Result<()>`
  - Wraps: `nix_bindings_store_remove_substituter()`

- `Store::list_substituters()` → `Result<Vec<(String, i32)>>`
  - Wraps: `nix_bindings_store_list_substituters()`
  - Returns (URI, priority) tuples

- `Store::clear_substituters()` → `Result<()>`
  - Wraps: `nix_bindings_store_clear_substituters()`

### 1.2 Types and Enums

**Exposed Rust enum `TrustedFlag`:**
- `TrustedFlag::Trusted` - Client is trusted by the store
- `TrustedFlag::NotTrusted` - Client is not trusted by the store
- `TrustedFlag::Unknown` - Trust status is not applicable for this store type (e.g., HTTP binary caches)

Used by `Store::is_trusted_client()` to indicate the trust status of the client connection.

### 1.4 StorePath

**Public API in `StorePath` struct:**
- `StorePath::name()` → `Result<String>`
  - Extracts basename from store path (e.g., "foo-1.2" from "/nix/store/abc...-foo-1.2")

### 1.5 Derivation (Nix 2.33+)

**Minimal wrapper struct:**
- Holds parsed derivation from JSON
- Used with `Store::add_derivation()` and `BuildEnvironment::from_derivation()`

### 1.6 BuildEnvironment

**Exposed Rust API in `BuildEnvironment` struct:**

- `BuildEnvironment::new()` → `Result<BuildEnvironment>`
  - Wraps: `nix_build_env_new()`
  - Creates empty build environment

- `BuildEnvironment::parse_json(json: &str)` → `Result<BuildEnvironment>`
  - Wraps: `nix_build_env_parse_json()`
  - Parses from `nix print-dev-env --json` output

- `BuildEnvironment::from_derivation(store: &Store, drv_path: &StorePath)` → `Result<BuildEnvironment>`
  - Wraps: `nix_build_env_from_derivation()`
  - Extracts build environment from derivation store path

---

## 2. nix-bindings-expr

### 2.1 Expression Evaluation Core

**Exposed Rust API in `EvalState` struct:**

#### Core Evaluation
- `EvalState::new(store: Store, lookup_path: impl IntoIterator<Item = &str>)` → `Result<EvalState>`
  - Creates evaluation state with optional lookup paths

- `EvalState::eval_from_string(expr: &str, path: &str)` → `Result<Value>`
  - Wraps: `nix_eval_from_string()`
  - Evaluates Nix expression from string

- `EvalState::force(v: &Value)` → `Result<()>`
  - Wraps: `nix_force_value()`
  - Forces evaluation of a thunk to WHNF

#### Type Inspection
- `EvalState::value_type_unforced(value: &Value)` → `Option<ValueType>`
  - Wraps: `nix_get_type()`
  - Non-evaluating type check; returns None if thunk

- `EvalState::value_type(value: &Value)` → `Result<ValueType>`
  - Wraps: `nix_get_type()` (with prior force)
  - Type checking after evaluation to WHNF

#### Value Extraction - Scalar Types
- `EvalState::require_int(v: &Value)` → `Result<i64>`
  - Wraps: `nix_get_int()`

- `EvalState::require_bool(v: &Value)` → `Result<bool>`
  - Wraps: `nix_get_bool()`

- `EvalState::require_string(value: &Value)` → `Result<String>`
  - Wraps: `nix_get_string()`

- `EvalState::realise_string(value: &Value)` → `Result<RealisedString>`
  - Wraps: `nix_realise_string()`
  - Returns string with associated store path references

#### Value Extraction - Collections
- `EvalState::require_list_size(v: &Value)` → `Result<u32>`
  - Wraps: `nix_get_list_size()`
  - Lazy: doesn't evaluate list elements

- `EvalState::require_list_select_idx_strict(v: &Value, idx: u32)` → `Result<Option<Value>>`
  - Wraps: `nix_list_elem()`
  - Evaluates only accessed element

- `EvalState::require_list_strict<C: FromIterator<Value>>(value: &Value)` → `Result<C>`
  - Collects all list elements into container (strict evaluation)

- `EvalState::require_attrs_names(v: &Value)` → `Result<Vec<String>>`
  - Wraps: `nix_get_attr_names()`
  - Returns sorted attribute names

- `EvalState::require_attrs_names_unsorted(v: &Value)` → `Result<Vec<String>>`
  - Wraps: `nix_get_attr_names_unsorted()`

- `EvalState::require_attrs_select(v: &Value, attr_name: &str)` → `Result<Value>`
  - Wraps: `nix_get_attr()`
  - Selects single attribute (lazy)

- `EvalState::require_attrs_select_opt(v: &Value, attr_name: &str)` → `Result<Option<Value>>`
  - Optional attribute selection

#### Value Construction
- `EvalState::new_value_str(s: &str)` → `Result<Value>`
  - Wraps: `nix_new_value_string()`

- `EvalState::new_value_int(i: i64)` → `Result<Value>`
  - Wraps: `nix_new_value_int()`

- `EvalState::new_value_attrs<I: IntoIterator>(attrs: I)` → `Result<Value>`
  - Creates attribute set from iterator of (key, Value) pairs
  - Supports Vec, HashMap, BTreeMap, slice formats

- `EvalState::new_value_thunk(builder: ThunkBuilder, f: F)` → `Result<Value>`
  - Wraps: `nix_new_value_thunk()`
  - Creates lazy thunk with custom computation

#### Function Operations
- `EvalState::call(f: Value, a: Value)` → `Result<Value>`
  - Wraps: `nix_call()`
  - Function application (single argument)

- `EvalState::call_multi(f: &Value, args: &[Value])` → `Result<Value>`
  - Multi-argument function application

- `EvalState::new_value_apply(f: &Value, a: &Value)` → `Result<Value>`
  - Creates unevaluated application thunk

- `EvalState::new_value_primop(primop: PrimOp)` → `Result<Value>`
  - Wraps: `nix_new_value_primop()`
  - Creates custom primitive operation

#### Debugging and Control
- `EvalState::enable_debugger()` → `Result<()>`
  - Wraps: `nix_enable_debugger()`

### 2.2 Value Type System

**Exposed Rust enum `ValueType`:**
```
- AttrSet (attribute set)
- Bool
- External (opaque plugin value)
- Float
- Function
- Int
- List
- Null
- Path (path value)
- String
- Unknown (new unsupported type)
```

### 2.3 Memory Management

**Exposed Rust API:**
- `gc_register_my_thread()` → `Result<ThreadRegistrationGuard>`
  - Wraps: GC thread registration
  - Must call before using EvalState in a thread

- `gc_now()`
  - Wraps: `nix_gc_now()`
  - Forces garbage collection

- `init()` → `Result<()>`
  - Wraps: `nix_libexpr_init()`
  - Library initialization

### 2.4 Logger

**Exposed Rust API in `Logger` struct:**
- `Logger::new()` → `Result<Logger>`
  - Wraps: `nix_logger_new()`

- `Logger::set_verbose(verbose: bool)` → `Result<()>`
  - Wraps: `nix_logger_set_verbose()`

---

## 3. nix-bindings-flake

### 3.1 Flake Settings and References

**Exposed Rust API:**

#### FlakeSettings
- `FlakeSettings::new()` → `Result<FlakeSettings>`
  - Wraps: `nix_flake_settings_new()`
  - Enables features like `builtins.getFlake`

- `EvalStateBuilder::flakes(settings: &FlakeSettings)` → Trait extension
  - Wraps: `nix_flake_settings_add_to_eval_state_builder()`

#### FlakeReferenceParseFlags
- `FlakeReferenceParseFlags::new(settings: &FlakeSettings)` → `Result<Self>`
  - Wraps: `nix_flake_reference_parse_flags_new()`

- `FlakeReferenceParseFlags::set_base_directory(path: &str)` → `Result<()>`
  - Wraps: `nix_flake_reference_parse_flags_set_base_directory()`

#### FlakeReference
- `FlakeReference::parse_with_fragment(fetch_settings, flake_settings, flags, ref_string)` → `Result<(FlakeReference, String)>`
  - Wraps: `nix_flake_reference_and_fragment_from_string()`
  - Parses flake reference and fragment (e.g., "github:owner/repo#output")

### 3.2 Flake Locking

**Exposed Rust API:**

#### LockedFlake
- `LockedFlake::lock(fetch_settings, flake_settings, eval_state, flags, flake_ref)` → `Result<LockedFlake>`
  - Wraps: `nix_flake_lock()`
  - Locks a flake and evaluates it

- `LockedFlake::outputs(flake_settings, eval_state)` → `Result<Value>`
  - Wraps: `nix_locked_flake_get_output_attrs()`
  - Gets flake outputs as Nix value

#### FlakeLockFlags
- `FlakeLockFlags::new(settings: &FlakeSettings)` → `Result<Self>`
  - Wraps: `nix_flake_lock_flags_new()`

- `FlakeLockFlags::set_mode_write_as_needed()` → `Result<()>`
  - Wraps: `nix_flake_lock_flags_set_mode_write_as_needed()`

- `FlakeLockFlags::set_mode_check()` → `Result<()>`
  - Wraps: `nix_flake_lock_flags_set_mode_check()`

- `FlakeLockFlags::set_mode_virtual()` → `Result<()>`
  - Wraps: `nix_flake_lock_flags_set_mode_virtual()`

- `FlakeLockFlags::add_input_override(path: &str, ref: &FlakeReference)` → `Result<()>`
  - Wraps: `nix_flake_lock_flags_add_input_override()`
  - Override specific inputs during locking

- `FlakeLockFlags::add_input_update(path: &str)` → `Result<()>`
  - Wraps: `nix_flake_lock_flags_add_input_update()`
  - Mark input for update (ignore existing lock)

### 3.3 Flake Inputs and Lock Files

**Exposed Rust API:**

#### FlakeInput
- `FlakeInput::new(flake_ref: &FlakeReference, is_flake: bool)` → `Result<Self>`
  - Wraps: `nix_flake_input_new()`

#### FlakeInputs
- `FlakeInputs::new()` → `Result<Self>`
  - Wraps: `nix_flake_inputs_new()`

- `FlakeInputs::add(&mut self, name: &str, input: FlakeInput)` → `Result<()>`
  - Wraps: `nix_flake_inputs_add()`

#### LockFile
- `LockFile::new()` → `Result<Self>`
  - Wraps: `nix_lock_file_new()`

- `LockFile::parse(fetch_settings, json_content, source_path)` → `Result<Self>`
  - Wraps: `nix_lock_file_parse()`
  - Parses lock file from JSON string

- `LockFile::to_string()` → `Result<String>`
  - Wraps: `nix_lock_file_to_string()`
  - Serializes lock file to JSON

- `LockFile::equals(other: &LockFile)` → `Result<bool>`
  - Wraps: `nix_lock_file_equals()`

- `LockFile::diff(other: &LockFile)` → `Result<String>`
  - Wraps: `nix_lock_file_diff()`
  - Generates human-readable diff with ANSI colors

- `LockFile::has_changes(other: &LockFile)` → `Result<bool>`
  - Convenience wrapper for `!equals()`

- `LockFile::inputs_iterator()` → `Result<LockFileInputsIterator>`
  - Wraps: `nix_lock_file_inputs_iterator_new()`

#### LockFileInputsIterator
- `LockFileInputsIterator::next()` → `bool`
  - Wraps: `nix_lock_file_inputs_iterator_next()`
  - Advances to next input

- `LockFileInputsIterator::attr_path()` → `Result<String>`
  - Wraps: `nix_lock_file_inputs_iterator_get_attr_path()`

- `LockFileInputsIterator::locked_ref()` → `Result<String>`
  - Wraps: `nix_lock_file_inputs_iterator_get_locked_ref()`

- `LockFileInputsIterator::original_ref()` → `Result<String>`
  - Wraps: `nix_lock_file_inputs_iterator_get_original_ref()`

### 3.4 High-Level Flake Locking Builder

**Exposed Rust API in `InputsLocker` struct:**

Fluent builder for convenient flake input locking with batch operations:

```rust
InputsLocker::new(flake_settings)
    .with_inputs(inputs)
    .source_path("/path/to/flake")
    .old_lock_file(&existing_lock)
    .update_inputs(&["nixpkgs", "rust-overlay"])
    .override_input("custom", &custom_ref)
    .mode(LockMode::WriteAsNeeded)
    .lock(&fetch_settings, &eval_state)?
```

---

## 4. nix-bindings-fetchers

### 4.1 Fetchers Settings

**Exposed Rust API in `FetchersSettings` struct:**

- `FetchersSettings::new()` → `Result<Self>`
  - Wraps: `nix_fetchers_settings_new()`
  - Required for flake reference parsing and locking

---

## 5. nix-cmd

### 5.1 REPL Support

**Exposed Rust API:**

- `nix_cmd::init()` → `Result<()>`
  - Wraps: `nix_libcmd_init()`
  - Initializes command utilities

#### ValMap
- `ValMap::new()` → `Result<Self>`
  - Creates variable map for REPL environment

- `ValMap::insert(key: &str, value: &Value)` → `Result<()>`
  - Wraps: `nix_valmap_insert()`
  - Injects variables into REPL scope

#### REPL Execution
- `run_repl_simple(eval_state: &mut EvalState, extra_env: Option<&mut ValMap>)` → `Result<ReplExitStatus>`
  - Wraps: `nix_repl_run_simple()`
  - Runs interactive Nix REPL

**ReplExitStatus enum:**
- `QuitAll` - User exited with `:quit` (program should exit)
- `Continue` - User exited with `:continue` (program continues)

---

## 6. nix-bindings-util

### 6.1 Error Context Management

**Exposed Rust API:**

- `Context::new()` → `Context`
  - Creates error context for C API calls
  - Used internally by all bindings

### 6.2 Settings Management

**Exposed Rust API:**

- `nix_bindings_util::settings::set(key: &str, value: &str)` → `Result<()>`
  - Wraps: Sets Nix configuration settings

- `nix_bindings_util::settings::get(key: &str)` → `Result<String>`
  - Wraps: Gets Nix configuration settings

Examples:
- `set("experimental-features", "flakes")?`
- `set("substituters", "")?`
- `get("system")?`

---

## Summary Table: C Functions Wrapped

| Category | Function | Rust API | Crate |
|----------|----------|----------|-------|
| **Store Init** | `nix_libstore_init` | `Store::open()` | store |
| **Store Lifecycle** | `nix_store_open` | `Store::open()` | store |
| **Store Lifecycle** | `nix_store_free` | Drop impl | store |
| **Store Info** | `nix_store_get_uri` | `Store::get_uri()` | store |
| **Store Info** | `nix_store_get_storedir` | `Store::get_storedir()` | store |
| **Store Paths** | `nix_store_parse_path` | `Store::parse_store_path()` | store |
| **Store Paths** | `nix_store_real_path` | `Store::real_path()` | store |
| **Store Paths** | `nix_store_path_name` | `StorePath::name()` | store |
| **Store Paths** | `nix_store_path_free` | Drop impl | store |
| **Derivations** | `nix_derivation_from_json` | `Store::derivation_from_json()` | store |
| **Derivations** | `nix_add_derivation` | `Store::add_derivation()` | store |
| **Derivations** | `nix_derivation_free` | Drop impl | store |
| **Building** | `nix_store_realise` | `Store::realise()` | store |
| **Closures** | `nix_store_get_fs_closure` | `Store::get_fs_closure()` | store |
| **GC** | `nix_bindings_store_collect_garbage` | `Store::collect_garbage()` | store |
| **GC Roots** | `nix_bindings_store_add_perm_root` | `Store::add_perm_root()` | store |
| **GC Roots** | `nix_bindings_store_add_indirect_root` | `Store::add_indirect_root()` | store |
| **Substituters** | `nix_bindings_store_add_substituter` | `Store::add_substituter()` | store |
| **Substituters** | `nix_bindings_store_remove_substituter` | `Store::remove_substituter()` | store |
| **Substituters** | `nix_bindings_store_list_substituters` | `Store::list_substituters()` | store |
| **Substituters** | `nix_bindings_store_clear_substituters` | `Store::clear_substituters()` | store |
| **Build Env** | `nix_build_env_new` | `BuildEnvironment::new()` | store |
| **Build Env** | `nix_build_env_parse_json` | `BuildEnvironment::parse_json()` | store |
| **Build Env** | `nix_build_env_from_derivation` | `BuildEnvironment::from_derivation()` | store |
| **Expr Init** | `nix_libexpr_init` | `init()` | expr |
| **Expr Eval** | `nix_eval_from_string` | `EvalState::eval_from_string()` | expr |
| **Expr Force** | `nix_force_value` | `EvalState::force()` | expr |
| **Type Info** | `nix_get_type` | `EvalState::value_type()` | expr |
| **Type Check** | `nix_get_int` | `EvalState::require_int()` | expr |
| **Type Check** | `nix_get_bool` | `EvalState::require_bool()` | expr |
| **Type Check** | `nix_get_string` | `EvalState::require_string()` | expr |
| **Type Check** | `nix_realise_string` | `EvalState::realise_string()` | expr |
| **Lists** | `nix_get_list_size` | `EvalState::require_list_size()` | expr |
| **Lists** | `nix_list_elem` | `EvalState::require_list_select_idx_strict()` | expr |
| **Attrs** | `nix_get_attr_names` | `EvalState::require_attrs_names()` | expr |
| **Attrs** | `nix_get_attr_names_unsorted` | `EvalState::require_attrs_names_unsorted()` | expr |
| **Attrs** | `nix_get_attr` | `EvalState::require_attrs_select()` | expr |
| **Value Constr** | `nix_new_value_string` | `EvalState::new_value_str()` | expr |
| **Value Constr** | `nix_new_value_int` | `EvalState::new_value_int()` | expr |
| **Value Constr** | `nix_new_value_attrs` | `EvalState::new_value_attrs()` | expr |
| **Value Constr** | `nix_new_value_thunk` | `EvalState::new_value_thunk()` | expr |
| **Value Constr** | `nix_new_value_primop` | `EvalState::new_value_primop()` | expr |
| **Functions** | `nix_call` | `EvalState::call()` | expr |
| **Functions** | `nix_new_value_apply` | `EvalState::new_value_apply()` | expr |
| **Debugging** | `nix_enable_debugger` | `EvalState::enable_debugger()` | expr |
| **Value Mgmt** | `nix_value_incref` | Value::clone() | expr |
| **Value Mgmt** | `nix_value_decref` | Value::drop() | expr |
| **GC Thread** | GC registration | `gc_register_my_thread()` | expr |
| **GC** | `nix_gc_now` | `gc_now()` | expr |
| **Logger** | `nix_logger_new` | `Logger::new()` | expr |
| **Logger** | `nix_logger_set_verbose` | `Logger::set_verbose()` | expr |
| **Flake Settings** | `nix_flake_settings_new` | `FlakeSettings::new()` | flake |
| **Flake Settings** | `nix_flake_settings_add_to_eval_state_builder` | EvalStateBuilder ext | flake |
| **Flake Settings** | `nix_flake_settings_free` | Drop impl | flake |
| **Flake Ref Flags** | `nix_flake_reference_parse_flags_new` | `FlakeReferenceParseFlags::new()` | flake |
| **Flake Ref Flags** | `nix_flake_reference_parse_flags_set_base_directory` | `FlakeReferenceParseFlags::set_base_directory()` | flake |
| **Flake Ref Flags** | `nix_flake_reference_parse_flags_free` | Drop impl | flake |
| **Flake Ref** | `nix_flake_reference_and_fragment_from_string` | `FlakeReference::parse_with_fragment()` | flake |
| **Flake Ref** | `nix_flake_reference_free` | Drop impl | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_new` | `FlakeLockFlags::new()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_set_mode_write_as_needed` | `FlakeLockFlags::set_mode_write_as_needed()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_set_mode_check` | `FlakeLockFlags::set_mode_check()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_set_mode_virtual` | `FlakeLockFlags::set_mode_virtual()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_add_input_override` | `FlakeLockFlags::add_input_override()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_add_input_update` | `FlakeLockFlags::add_input_update()` | flake |
| **Flake Lock Flags** | `nix_flake_lock_flags_free` | Drop impl | flake |
| **Locked Flake** | `nix_flake_lock` | `LockedFlake::lock()` | flake |
| **Locked Flake** | `nix_locked_flake_get_output_attrs` | `LockedFlake::outputs()` | flake |
| **Locked Flake** | `nix_locked_flake_free` | Drop impl | flake |
| **Flake Input** | `nix_flake_input_new` | `FlakeInput::new()` | flake |
| **Flake Input** | `nix_flake_input_free` | Drop impl | flake |
| **Flake Inputs** | `nix_flake_inputs_new` | `FlakeInputs::new()` | flake |
| **Flake Inputs** | `nix_flake_inputs_add` | `FlakeInputs::add()` | flake |
| **Flake Inputs** | `nix_flake_inputs_free` | Drop impl | flake |
| **Lock File** | `nix_lock_file_new` | `LockFile::new()` | flake |
| **Lock File** | `nix_lock_file_parse` | `LockFile::parse()` | flake |
| **Lock File** | `nix_lock_file_to_string` | `LockFile::to_string()` | flake |
| **Lock File** | `nix_lock_file_equals` | `LockFile::equals()` | flake |
| **Lock File** | `nix_lock_file_diff` | `LockFile::diff()` | flake |
| **Lock File** | `nix_lock_file_free` | Drop impl | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_new` | `LockFile::inputs_iterator()` | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_next` | `LockFileInputsIterator::next()` | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_get_attr_path` | `LockFileInputsIterator::attr_path()` | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_get_locked_ref` | `LockFileInputsIterator::locked_ref()` | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_get_original_ref` | `LockFileInputsIterator::original_ref()` | flake |
| **Lock File Iter** | `nix_lock_file_inputs_iterator_free` | Drop impl | flake |
| **Flake Lock Inputs** | `nix_flake_lock_inputs` | `lock_inputs()` function | flake |
| **Fetchers** | `nix_fetchers_settings_new` | `FetchersSettings::new()` | fetchers |
| **Fetchers** | `nix_fetchers_settings_free` | Drop impl | fetchers |
| **REPL** | `nix_libcmd_init` | `nix_cmd::init()` | nix-cmd |
| **REPL** | `nix_valmap_new` | `ValMap::new()` | nix-cmd |
| **REPL** | `nix_valmap_insert` | `ValMap::insert()` | nix-cmd |
| **REPL** | `nix_valmap_free` | Drop impl | nix-cmd |
| **REPL** | `nix_repl_run_simple` | `run_repl_simple()` | nix-cmd |

---

## Key Capabilities Summary

### Direct FFI Access
- No subprocess spawning or CLI parsing overhead
- Direct in-memory operations on Nix data structures
- Proper error propagation through Rust Result types

### Expression Evaluation
- Full Nix language evaluation support
- Type-safe value extraction with automatic type checking
- Support for lazy and strict evaluation modes
- Custom primitive operation creation

### Store Operations
- Derivation parsing from JSON (Nix 2.33+)
- Direct building and store path management
- Closure computation with fine-grained control
- Garbage collection with multiple modes

### Flake Support
- Complete flake reference parsing
- Flake locking with virtual/write/check modes
- Lock file parsing, diffing, and comparison
- Batch input updates and overrides
- High-level builder API for convenience

### Development Environment
- Build environment extraction from derivations
- Environment variable and bash function handling
- Interactive REPL support with variable injection

---

## Configuration Examples

Setting experimental features:
```rust
nix_bindings_util::settings::set("experimental-features", "flakes")?;
```

Disabling substituters for offline mode:
```rust
store.clear_substituters()?;
```

Getting system information:
```rust
let system = nix_bindings_util::settings::get("system")?;
```

