//! Runtime proof for the devenv `tarball-ttl` fix.
//!
//! Locking/evaluating a flake input whose ref is a git branch resolves the
//! branch to a revision. Within `tarball-ttl` seconds that resolution is served
//! from the fetcher cache, so a fresh eval returns the stale revision. Setting
//! `tarball-ttl=0` on the eval state's fetcher settings (via
//! `EvalStateBuilder::fetch_setting`) must bypass that cache and re-resolve to
//! the new branch HEAD.
//!
//! A local `file://` git input does not persist its rev in `flake.lock`, so the
//! resolved rev is observed by evaluating the flake output `dep.rev`. The
//! on-disk fetcher cache is shared between evals, so the only variable between
//! the "stale" and "refreshed" results is the eval state's `tarball-ttl`.

use std::path::Path;
use std::process::Command;

use nix_bindings_expr::eval_state::{gc_register_my_thread, EvalState, EvalStateBuilder};
use nix_bindings_fetchers::FetchersSettings;
use nix_bindings_flake::{
    EvalStateBuilderExt, FlakeLockFlags, FlakeReference, FlakeReferenceParseFlags, FlakeSettings,
    LockedFlake,
};
use nix_bindings_store::store::Store;

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn commit(repo: &Path, content: &str) -> String {
    std::fs::write(repo.join("file"), content).unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", content]);
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Lock the consumer flake from scratch and return the rev `dep` resolves to,
/// read from the evaluated flake output so it reflects the actual fetch.
fn resolved_dep_rev(
    eval_state: &mut EvalState,
    fetch_settings: &FetchersSettings,
    flake_settings: &FlakeSettings,
    consumer: &Path,
) -> String {
    // Remove any existing lock so every call re-resolves the input.
    let _ = std::fs::remove_file(consumer.join("flake.lock"));

    let parse_flags = FlakeReferenceParseFlags::new(flake_settings).unwrap();
    let (flake_ref, _) = FlakeReference::parse_with_fragment(
        fetch_settings,
        flake_settings,
        &parse_flags,
        &format!("path:{}", consumer.display()),
    )
    .unwrap();

    let mut flags = FlakeLockFlags::new(flake_settings).unwrap();
    flags.set_mode_write_as_needed().unwrap();

    let locked =
        LockedFlake::lock(fetch_settings, flake_settings, eval_state, &flags, &flake_ref).unwrap();

    let outputs = locked.outputs(flake_settings, eval_state).unwrap();
    let rev = eval_state.require_attrs_select(&outputs, &"rev").unwrap();
    eval_state.require_string(&rev).unwrap()
}

#[test]
fn tarball_ttl_zero_forces_branch_refresh() {
    // Isolate the fetcher cache so the TTL behavior is deterministic.
    let cache = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CACHE_HOME", cache.path());

    nix_bindings_expr::eval_state::init().unwrap();
    nix_bindings_util::settings::set("experimental-features", "flakes nix-command").unwrap();
    let _gc = gc_register_my_thread();

    // Upstream as a bare repo (a local working tree is treated as a mutable
    // source). Commit in a work clone and push to the bare remote.
    let work = tempfile::tempdir().unwrap();
    git(work.path(), &["init", "-q", "-b", "trunk"]);
    let rev_a = commit(work.path(), "a");

    let bare_dir = tempfile::tempdir().unwrap();
    let bare = bare_dir.path().join("repo.git");
    git(work.path(), &["clone", "--bare", "-q", ".", bare.to_str().unwrap()]);

    // Consumer flake that pulls the branch as a (non-flake) input and exposes
    // the resolved rev as an output.
    let consumer = tempfile::tempdir().unwrap();
    std::fs::write(
        consumer.path().join("flake.nix"),
        format!(
            "{{\n  inputs.dep.url = \"git+file://{}?ref=trunk\";\n  inputs.dep.flake = false;\n  outputs = {{ dep, ... }}: {{ rev = dep.rev; }};\n}}\n",
            bare.display()
        ),
    )
    .unwrap();

    let store = Store::open(None, []).unwrap();
    let flake_settings = FlakeSettings::new().unwrap();
    let fetch_settings = FetchersSettings::new().unwrap();

    let build_state = |refresh: bool| -> EvalState {
        let mut b = EvalStateBuilder::new(store.clone())
            .unwrap()
            .flakes(&flake_settings)
            .unwrap();
        if refresh {
            b = b.fetch_setting("tarball-ttl", "0").unwrap();
        }
        b.build().unwrap()
    };

    // Eval #1: resolves trunk to rev A and warms the fetcher cache.
    let mut s1 = build_state(false);
    let got_a = resolved_dep_rev(&mut s1, &fetch_settings, &flake_settings, consumer.path());
    assert_eq!(got_a, rev_a, "first eval should resolve to rev A");

    // Move the branch forward on the remote.
    let rev_b = commit(work.path(), "b");
    git(work.path(), &["push", "-q", bare.to_str().unwrap(), "trunk"]);
    assert_ne!(rev_a, rev_b);

    // Control: a fresh default eval state still serves the cached rev (the bug).
    let mut s2 = build_state(false);
    let got_cached = resolved_dep_rev(&mut s2, &fetch_settings, &flake_settings, consumer.path());
    assert_eq!(
        got_cached, rev_a,
        "with default tarball-ttl the stale cached rev is returned"
    );

    // Fix: tarball-ttl=0 on the eval state forces re-resolution to rev B.
    let mut s3 = build_state(true);
    let got_refreshed = resolved_dep_rev(&mut s3, &fetch_settings, &flake_settings, consumer.path());
    assert_eq!(
        got_refreshed, rev_b,
        "with tarball-ttl=0 the branch is re-resolved to rev B"
    );
}
