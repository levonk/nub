//! Shared `@nubjs/types` project-integration wiring, used by both `nub init`
//! (scaffolds it into a fresh tsconfig + package.json) and `nub agent init`
//! (offers it on an existing TS-shaped project).
//!
//! The wiring has three parts, all additive and idempotent:
//!   1. tsconfig `compilerOptions.types += ["node", "@nubjs/types"]` and
//!      `lib += es2024` (dropping a `"dom"` placeholder) — see [`tsconfig`].
//!   2. `package.json` `devDependencies += "@nubjs/types"` at the binary's
//!      version — see [`package_json`].
//!   3. An optional in-repo `nub-env.d.ts` fallback (offline / no-install pickup).
//!
//! This module owns the *file IO + prompting* around those pure merges so the two
//! callers don't duplicate it. `nub agent init` only offers the wiring when a
//! tsconfig already exists; `nub init` scaffolds the tsconfig itself and then
//! invokes the same wiring so a fresh project lands fully type-aware.

use std::path::Path;

use anyhow::Result;

use crate::agent::prompt::Confirm;
use crate::init::{package_json, tsconfig};

/// The in-repo ambient-declarations fallback, bundled from `assets/nub-env.d.ts`
/// (kept byte-identical to the `@nubjs/types` package content). Written into the
/// project as the offline / no-install-step pickup path.
pub const NUB_ENV_DTS: &str = include_str!("../../assets/nub-env.d.ts");

/// Wire nub's types into an existing project: merge the tsconfig, add the
/// `@nubjs/types` devDependency, and (default-NO) optionally drop the
/// `nub-env.d.ts` fallback.
///
/// `has_package_json` gates the devDep write; the fallback `.d.ts` is offered
/// regardless. Each accepted write appends a human description to `written`.
/// Returns `Ok` even when individual merges are skipped (malformed tsconfig,
/// no package.json) — those are reported, not fatal.
///
/// The caller is responsible for deciding *whether* a tsconfig exists to wire;
/// `nub init` passes a freshly-written one, `nub agent init` gates on detection.
pub fn wire(
    cwd: &Path,
    has_package_json: bool,
    confirm: &Confirm,
    written: &mut Vec<String>,
) -> Result<()> {
    let tsconfig_path = cwd.join("tsconfig.json");
    let text = std::fs::read_to_string(&tsconfig_path)?;
    let ts_plan = match tsconfig::plan(&text) {
        Ok(p) => p,
        Err(e) => {
            println!("  skipping tsconfig: {e}");
            return Ok(());
        }
    };

    if ts_plan.changed {
        let mut q = format!(
            "Wire nub's types into tsconfig.json (types += {}, lib += es2024{})?",
            tsconfig::TYPES_PACKAGE,
            if ts_plan.dropped_dom {
                ", drop dom"
            } else {
                ""
            }
        );
        if ts_plan.had_comments {
            q.push_str(" [note: comments in tsconfig.json will be removed]");
        }
        if confirm.ask(&q, true) {
            std::fs::write(&tsconfig_path, &ts_plan.new_text)?;
            written.push("tsconfig.json (types wired)".to_string());
            // Also add @nubjs/types to devDependencies when package.json is present.
            add_dev_dep(cwd, has_package_json, written)?;
        }
    } else {
        println!("  tsconfig.json already wired for nub's types — no change");
        // Even if the tsconfig was already wired, the devDep may still be missing.
        add_dev_dep(cwd, has_package_json, written)?;
    }

    // The in-repo fallback `.d.ts` — works without installing `@nubjs/types`
    // (offline / no-install). Default NO: it duplicates the package's content, so
    // it's only wanted when the package can't be added.
    if confirm.ask(
        "Also drop an in-repo nub-env.d.ts fallback (for offline / no-install)?",
        false,
    ) {
        let path = cwd.join("nub-env.d.ts");
        std::fs::write(&path, NUB_ENV_DTS)?;
        written.push("nub-env.d.ts (types fallback)".to_string());
    }
    Ok(())
}

/// Pure tsconfig merge for the scaffold path: given the current tsconfig text,
/// return `Some(new_text)` when nub's `types`/`lib` need adding, or `None` when
/// it's already wired (idempotent no-op). Used by `nub init`, which always
/// accepts the wiring on its own freshly-written tsconfig (no prompt).
pub fn tsconfig_for(text: &str) -> Result<Option<String>> {
    let plan = tsconfig::plan(text)?;
    Ok(if plan.changed {
        Some(plan.new_text)
    } else {
        None
    })
}

/// If a `package.json` exists, add `@nubjs/types` to `devDependencies`.
/// Silently skips when no package.json is present. Idempotent.
pub fn add_dev_dep(cwd: &Path, has_package_json: bool, written: &mut Vec<String>) -> Result<()> {
    if !has_package_json {
        return Ok(());
    }
    let pkg_path = cwd.join("package.json");
    let text = std::fs::read_to_string(&pkg_path)?;
    let plan = match package_json::plan(&text) {
        Ok(p) => p,
        Err(e) => {
            println!("  skipping package.json devDep: {e}");
            return Ok(());
        }
    };
    if plan.changed {
        std::fs::write(&pkg_path, &plan.new_text)?;
        written.push(format!(
            "package.json (devDependencies += {}@{})",
            tsconfig::TYPES_PACKAGE,
            package_json::TYPES_VERSION,
        ));
    }
    Ok(())
}
