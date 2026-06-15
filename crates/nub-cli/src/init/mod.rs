//! `nub init` — scaffold a minimal modern-TS Node project.
//!
//! Spec: `wiki/commands/init.md`. Writes up to five files (`package.json`,
//! `tsconfig.json`, `index.ts`, `.gitignore`, `README.md`), runs `git init`, and
//! never installs by default (lock-file agnostic). Existing target files are
//! refused with a per-file list unless `--force`.
//!
//! This module also owns the SHARED `@nubjs/types` project-integration wiring
//! ([`types_wiring`], [`tsconfig`], [`package_json`]) that `nub agent init` reuses
//! — the tsconfig `types`/`lib` merge, the `@nubjs/types` devDependency, and the
//! `nub-env.d.ts` fallback. `nub init` scaffolds the tsconfig and then runs that
//! wiring so a fresh project lands fully type-aware; `nub agent init` offers the
//! same wiring on an existing TS-shaped project.
//!
//! Like `node`/`pm`/`agent`, this is a manual-dispatch verb (not a clap variant)
//! so its flag-parsing and messages stay self-contained.

pub mod package_json;
mod scaffold;
pub mod tsconfig;
pub mod types_wiring;

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::agent::prompt::{Confirm, Mode};

/// Whether the entry file / language is TypeScript or JavaScript. JS skips the
/// tsconfig (and the `@nubjs/types` wiring) and writes `index.js`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    TypeScript,
    JavaScript,
}

impl Language {
    fn entry(self) -> &'static str {
        match self {
            Language::TypeScript => "index.ts",
            Language::JavaScript => "index.js",
        }
    }
}

/// Parsed `nub init` flags. Hand-parsed (manual-dispatch verb), matching the
/// spec's flag table. `Option<bool>` fields distinguish "user said yes/no" from
/// "unset, so prompt / use default".
#[derive(Debug)]
struct InitFlags {
    yes: bool,
    name: Option<String>,
    language: Option<Language>,
    git: Option<bool>,
    tsconfig: bool,
    gitignore: bool,
    readme: bool,
    package_json: bool,
    force: bool,
    install: Option<bool>,
    pm: Option<String>,
    dir: Option<PathBuf>,
}

impl Default for InitFlags {
    fn default() -> Self {
        Self {
            yes: false,
            name: None,
            language: None,
            git: None,
            tsconfig: true,
            gitignore: true,
            readme: true,
            package_json: true,
            force: false,
            install: None,
            pm: None,
            dir: None,
        }
    }
}

/// Entry point for `nub init …`, dispatched from `dispatch_subcommand`.
pub fn run(args: &[String]) -> Result<i32> {
    if matches!(
        args.first().map(String::as_str),
        Some("help") | Some("--help") | Some("-h")
    ) {
        print_usage();
        return Ok(0);
    }
    let flags = parse_flags(args)?;
    run_init(flags)
}

fn print_usage() {
    println!(
        "nub init — scaffold a minimal modern-TS Node project\n\n\
         Usage: nub init [options]\n\n\
         Options:\n\
         \x20 -y, --yes            non-interactive; accept all defaults\n\
         \x20     --name <name>    project name (default: directory name)\n\
         \x20     --js             JavaScript variant (index.js, no tsconfig)\n\
         \x20     --no-git         skip `git init`\n\
         \x20     --no-tsconfig    don't write tsconfig.json\n\
         \x20     --no-gitignore   don't write .gitignore\n\
         \x20     --no-readme      don't write README.md\n\
         \x20     --no-package-json  don't write package.json\n\
         \x20     --force          overwrite existing files\n\
         \x20     --install        run install at the end\n\
         \x20     --no-install     skip install (default)\n\
         \x20     --pm <name>      package manager for install (pnpm/npm/bun/yarn)\n\
         \x20 -C, --dir <DIR>      operate on <DIR> instead of the cwd"
    );
}

fn parse_flags(args: &[String]) -> Result<InitFlags> {
    let mut f = InitFlags::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "-y" | "--yes" => f.yes = true,
            "--js" => f.language = Some(Language::JavaScript),
            "--ts" => f.language = Some(Language::TypeScript),
            "--name" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    bail!("`nub init --name` requires a value");
                };
                f.name = Some(v.clone());
            }
            s if s.starts_with("--name=") => f.name = Some(s["--name=".len()..].to_string()),
            "--git" => f.git = Some(true),
            "--no-git" => f.git = Some(false),
            "--no-tsconfig" => f.tsconfig = false,
            "--no-gitignore" => f.gitignore = false,
            "--no-readme" => f.readme = false,
            "--no-package-json" => f.package_json = false,
            "--force" => f.force = true,
            "--install" => f.install = Some(true),
            "--no-install" => f.install = Some(false),
            "--pm" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    bail!("`nub init --pm` requires a value (pnpm/npm/bun/yarn)");
                };
                f.pm = Some(v.clone());
            }
            s if s.starts_with("--pm=") => f.pm = Some(s["--pm=".len()..].to_string()),
            "-C" | "--dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    bail!("`nub init {arg}` requires a directory");
                };
                f.dir = Some(PathBuf::from(v));
            }
            s if s.starts_with("--dir=") => f.dir = Some(PathBuf::from(&s["--dir=".len()..])),
            other => bail!("unknown flag '{other}' for `nub init`"),
        }
        i += 1;
    }
    if f.pm.is_some() && f.install == Some(false) {
        bail!("`nub init --pm` requires `--install`");
    }
    Ok(f)
}

fn run_init(flags: InitFlags) -> Result<i32> {
    let cwd = match &flags.dir {
        Some(d) => d.clone(),
        None => std::env::current_dir()?,
    };
    std::fs::create_dir_all(&cwd)?;

    // Non-TTY (CI / piped stdin) falls back to non-interactive defaults even
    // without -y, so init never hangs waiting on stdin.
    let interactive = !flags.yes && io::stdin().is_terminal() && io::stdout().is_terminal();

    // ── Resolve the four interactive decisions (name, language, git, install).
    let default_name = default_project_name(&cwd);
    let name = match &flags.name {
        Some(n) => scaffold::sanitize_name(n),
        None => {
            let chosen = if interactive {
                prompt_line("Project name?", &default_name)
            } else {
                default_name.clone()
            };
            scaffold::sanitize_name(&chosen)
        }
    };

    // `-y`/`--yes` means "skip the prompts, take each question's stated DEFAULT"
    // (git=Yes, install=No) — NOT "say yes to everything". So a non-interactive
    // run resolves via `Mode::Defaults`, which honors each question's default;
    // only a true interactive run prompts. (This is the difference from `nub
    // agent init`, where --yes is an accept-every-offer affirmation.)
    let confirm = Confirm::new(if interactive {
        Mode::Interactive
    } else {
        Mode::Defaults
    });

    let language = match flags.language {
        Some(l) => l,
        None => {
            if interactive && !confirm.ask("Use TypeScript? (No = JavaScript)", true) {
                Language::JavaScript
            } else {
                Language::TypeScript
            }
        }
    };

    let do_git = match flags.git {
        Some(g) => g,
        None => confirm.ask("Initialize a git repository?", true),
    };

    let do_install = match flags.install {
        Some(v) => v,
        None => confirm.ask("Install dependencies now?", false),
    };

    println!("nub init — {}", cwd.display());

    // ── Plan the file set. JS skips tsconfig regardless of --no-tsconfig.
    let entry = language.entry();
    let want_tsconfig = flags.tsconfig && language == Language::TypeScript;
    let mut targets: Vec<(&str, FileBody)> = Vec::new();
    if flags.package_json {
        targets.push((
            "package.json",
            FileBody::Owned(scaffold::package_json(&name, entry)),
        ));
    }
    if want_tsconfig {
        targets.push(("tsconfig.json", FileBody::Static(scaffold::TSCONFIG_JSON)));
    }
    targets.push((entry, FileBody::Static(scaffold::ENTRY_BODY)));
    if flags.gitignore {
        targets.push((".gitignore", FileBody::Static(scaffold::GITIGNORE)));
    }
    if flags.readme {
        targets.push(("README.md", FileBody::Owned(scaffold::readme(&name))));
    }

    // ── Refuse on any existing target unless --force.
    if !flags.force {
        let conflicts: Vec<&str> = targets
            .iter()
            .filter(|(rel, _)| cwd.join(rel).exists())
            .map(|(rel, _)| *rel)
            .collect();
        if !conflicts.is_empty() {
            let list = conflicts
                .iter()
                .map(|c| format!("  {c}"))
                .collect::<Vec<_>>()
                .join("\n");
            bail!(
                "nub init: refusing to overwrite existing files:\n{list}\n  \
                 (re-run with --force to overwrite)"
            );
        }
    }

    // ── Write the files.
    let mut written: Vec<String> = Vec::new();
    for (rel, body) in &targets {
        let path = cwd.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body.as_str())?;
        written.push(rel.to_string());
    }

    // ── Wire the shared @nubjs/types integration into the fresh project. Only
    //    for TS projects that got a tsconfig + package.json. We pass a
    //    non-interactive confirm here (the user already chose TypeScript; the
    //    wiring is the whole point of a TS scaffold) so it lands without an extra
    //    prompt, but we DON'T auto-write the offline `.d.ts` fallback (that's the
    //    no-install escape hatch, not wanted by default on a fresh project).
    if want_tsconfig && flags.package_json {
        let wiring_confirm = TypesWiringConfirm;
        wiring_confirm.run(&cwd, &mut written)?;
    }

    // ── git init (unless --no-git / non-TTY default-no, and never in an existing repo).
    if do_git {
        if cwd.join(".git").exists() {
            println!("  .git already present — skipping git init");
        } else {
            match run_git_init(&cwd) {
                Ok(true) => written.push(".git/".to_string()),
                Ok(false) => println!("  git init failed — skipped (is git installed?)"),
                Err(e) => println!("  git init error: {e} — skipped"),
            }
        }
    }

    print_summary(&written, entry);

    // ── Optional install.
    if do_install {
        let pm = flags.pm.as_deref().unwrap_or("pnpm");
        run_install(&cwd, pm)?;
    }

    Ok(0)
}

/// A non-interactive `Confirm`-shaped wrapper that accepts the tsconfig+devDep
/// wiring but declines the offline `.d.ts` fallback, used for the fresh-project
/// scaffold path. (We can't use a plain `Confirm` because it would also accept
/// the fallback under `AssumeYes`.)
struct TypesWiringConfirm;

impl TypesWiringConfirm {
    fn run(&self, cwd: &Path, written: &mut Vec<String>) -> Result<()> {
        // The merge + devDep are scaffold-defaults (yes); the fallback is no.
        // `types_wiring::wire` drives both off one `Confirm`, so for the scaffold
        // path we call the lower-level pieces directly to get yes/yes/no.
        let ts_path = cwd.join("tsconfig.json");
        let text = std::fs::read_to_string(&ts_path)?;
        let plan = types_wiring::tsconfig_for(&text)?;
        if let Some(new_text) = plan {
            std::fs::write(&ts_path, new_text)?;
            written.push("tsconfig.json (types wired)".to_string());
        }
        types_wiring::add_dev_dep(cwd, true, written)?;
        Ok(())
    }
}

/// What to write for a target file — a static `&str` or an owned `String`.
enum FileBody {
    Static(&'static str),
    Owned(String),
}

impl FileBody {
    fn as_str(&self) -> &str {
        match self {
            FileBody::Static(s) => s,
            FileBody::Owned(s) => s.as_str(),
        }
    }
}

/// The default project name: the directory basename, sanitized.
fn default_project_name(cwd: &Path) -> String {
    let raw = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_string();
    scaffold::sanitize_name(&raw)
}

/// Prompt for a single line of text on a TTY; returns `default` on empty/EOF.
fn prompt_line(question: &str, default: &str) -> String {
    print!("{question} ({default}) ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return default.to_string();
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Run `git init` in `cwd`. Returns `Ok(true)` on success, `Ok(false)` when git
/// ran but returned non-zero, `Err` when git couldn't be spawned at all.
fn run_git_init(cwd: &Path) -> Result<bool> {
    let status = std::process::Command::new("git")
        .arg("init")
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) => Ok(s.success()),
        Err(e) => Err(e.into()),
    }
}

/// Run the chosen package manager's install in `cwd`. Best-effort: a missing PM
/// is reported, not fatal.
fn run_install(cwd: &Path, pm: &str) -> Result<()> {
    if !matches!(pm, "pnpm" | "npm" | "bun" | "yarn") {
        bail!("unknown package manager '{pm}' (expected pnpm/npm/bun/yarn)");
    }
    println!("\nRunning {pm} install …");
    let status = std::process::Command::new(pm)
        .arg("install")
        .current_dir(cwd)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(_) => println!("  {pm} install exited non-zero"),
        Err(_) => println!("  could not run '{pm}' — is it installed?"),
    }
    Ok(())
}

fn print_summary(written: &[String], entry: &str) {
    if written.is_empty() {
        println!("\nNothing written.");
        return;
    }
    println!("\nCreated:");
    for w in written {
        println!("  + {w}");
    }
    println!("\nNext: nub {entry}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn td() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn yes_init_in(dir: &Path, extra: &[&str]) -> i32 {
        let mut args: Vec<String> = vec![
            "--yes".into(),
            "--no-git".into(),
            "--dir".into(),
            dir.display().to_string(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));
        run(&args).unwrap()
    }

    #[test]
    fn yes_in_empty_dir_writes_the_five_ts_files() {
        let d = td();
        assert_eq!(yes_init_in(d.path(), &[]), 0);
        for f in [
            "package.json",
            "tsconfig.json",
            "index.ts",
            ".gitignore",
            "README.md",
        ] {
            assert!(d.path().join(f).is_file(), "{f} must be written");
        }
        // No index.js for a TS scaffold.
        assert!(!d.path().join("index.js").exists());
    }

    #[test]
    fn package_json_has_start_script_and_module_type() {
        let d = td();
        yes_init_in(d.path(), &[]);
        let pkg = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let v: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(v["type"], "module");
        assert_eq!(v["scripts"]["start"], "nub index.ts");
        assert_eq!(v["version"], "0.0.1");
    }

    #[test]
    fn name_defaults_to_sanitized_directory_basename() {
        let d = td();
        let sub = d.path().join("My App");
        std::fs::create_dir(&sub).unwrap();
        yes_init_in(&sub, &[]);
        let pkg = std::fs::read_to_string(sub.join("package.json")).unwrap();
        let v: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(v["name"], "my-app", "basename sanitized to npm-valid name");
    }

    #[test]
    fn explicit_name_flag_is_sanitized_and_wins() {
        let d = td();
        yes_init_in(d.path(), &["--name", "Cool Thing!!"]);
        let pkg = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let v: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(v["name"], "cool-thing");
    }

    #[test]
    fn types_wiring_lands_for_a_fresh_ts_project() {
        let d = td();
        yes_init_in(d.path(), &[]);
        // tsconfig has @nubjs/types in types and es2024 in lib.
        let ts = std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap();
        let v: Value = serde_json::from_str(&ts).unwrap();
        let types: Vec<&str> = v["compilerOptions"]["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap())
            .collect();
        assert!(types.contains(&"node"));
        assert!(types.contains(&"@nubjs/types"));
        // devDependency added to package.json at the binary version.
        let pkg = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let pv: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(
            pv["devDependencies"]["@nubjs/types"],
            Value::String(env!("CARGO_PKG_VERSION").to_string())
        );
        // The offline fallback .d.ts is NOT written on a fresh scaffold.
        assert!(!d.path().join("nub-env.d.ts").exists());
    }

    #[test]
    fn js_variant_writes_index_js_and_skips_tsconfig() {
        let d = td();
        yes_init_in(d.path(), &["--js"]);
        assert!(d.path().join("index.js").is_file());
        assert!(!d.path().join("tsconfig.json").exists());
        assert!(!d.path().join("index.ts").exists());
        let pkg = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let v: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(v["scripts"]["start"], "nub index.js");
        // No types devDep for a JS project.
        assert!(v["devDependencies"].is_null());
    }

    #[test]
    fn refuses_existing_package_json_without_force() {
        let d = td();
        std::fs::write(d.path().join("package.json"), "{}").unwrap();
        let err = run(&[
            "--yes".into(),
            "--no-git".into(),
            "--dir".into(),
            d.path().display().to_string(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("refusing to overwrite"));
        assert!(err.to_string().contains("package.json"));
        // The untouched file is preserved (still "{}").
        assert_eq!(
            std::fs::read_to_string(d.path().join("package.json")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn force_overwrites_existing_files() {
        let d = td();
        std::fs::write(d.path().join("package.json"), "{\"old\":true}").unwrap();
        yes_init_in(d.path(), &["--force"]);
        let pkg = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let v: Value = serde_json::from_str(&pkg).unwrap();
        assert_eq!(v["type"], "module", "package.json was overwritten");
        assert!(v.get("old").is_none());
    }

    #[test]
    fn no_flags_suppress_individual_files() {
        let d = td();
        yes_init_in(
            d.path(),
            &["--no-tsconfig", "--no-gitignore", "--no-readme"],
        );
        assert!(d.path().join("package.json").is_file());
        assert!(d.path().join("index.ts").is_file());
        assert!(!d.path().join("tsconfig.json").exists());
        assert!(!d.path().join(".gitignore").exists());
        assert!(!d.path().join("README.md").exists());
    }

    #[test]
    fn no_package_json_skips_only_package_json() {
        let d = td();
        yes_init_in(d.path(), &["--no-package-json"]);
        assert!(!d.path().join("package.json").exists());
        assert!(d.path().join("index.ts").is_file());
        assert!(d.path().join("tsconfig.json").is_file());
        // No devDep target → no types devDep write attempted (no package.json).
        assert!(!d.path().join("nub-env.d.ts").exists());
    }

    #[test]
    fn rerun_is_idempotent_under_force() {
        let d = td();
        yes_init_in(d.path(), &[]);
        let pkg1 = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let ts1 = std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap();
        yes_init_in(d.path(), &["--force"]);
        let pkg2 = std::fs::read_to_string(d.path().join("package.json")).unwrap();
        let ts2 = std::fs::read_to_string(d.path().join("tsconfig.json")).unwrap();
        assert_eq!(pkg1, pkg2, "package.json stable across re-run");
        assert_eq!(ts1, ts2, "tsconfig.json stable across re-run");
    }

    #[test]
    fn yes_does_not_install_by_default() {
        // -y means "take each question's default"; install's default is No, so a
        // bare `nub init -y` must NOT run a package manager (no node_modules /
        // lockfile). (Regression guard: AssumeYes-everything would have installed.)
        let d = td();
        yes_init_in(d.path(), &[]);
        assert!(
            !d.path().join("node_modules").exists(),
            "init -y must not install"
        );
        assert!(!d.path().join("pnpm-lock.yaml").exists());
    }

    #[test]
    fn pm_without_install_is_an_error() {
        let err = parse_flags(&["--pm".into(), "pnpm".into(), "--no-install".into()]).unwrap_err();
        assert!(err.to_string().contains("requires `--install`"));
    }

    #[test]
    fn unknown_flag_errors() {
        assert!(parse_flags(&["--frobnicate".into()]).is_err());
    }

    #[test]
    fn help_prints_usage_ok() {
        assert_eq!(run(&["--help".into()]).unwrap(), 0);
        assert_eq!(run(&["help".into()]).unwrap(), 0);
    }
}
