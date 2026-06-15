//! The files `nub init` writes, plus project-name sanitization.
//!
//! Pure content + naming helpers — no IO. The shapes here track the spec in
//! `wiki/commands/init.md` ("Files written"): a `package.json` with only a
//! `start` script, a strict modern-TS `tsconfig.json`, a one-line entry file, a
//! minimal `.gitignore`, and a one-line `README.md`.

/// Sanitize a raw project name (usually the directory basename) into an
/// npm-valid package name. npm's rules: lowercase, ≤214 chars, no leading dot or
/// underscore, URL-safe — the allowed set is `a-z 0-9 - _ . ~` (plus a single
/// leading `@scope/` which we don't synthesize here).
///
/// The transform: lowercase, spaces → `-`, drop every other disallowed char,
/// collapse runs of `-`, trim leading/trailing `-._~`, cap at 214. An input that
/// sanitizes to nothing falls back to `"app"` so we never write an invalid name.
pub fn sanitize_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        let c = ch.to_ascii_lowercase();
        match c {
            'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            ' ' | '\t' => out.push('-'),
            _ => { /* drop everything else */ }
        }
    }
    // Collapse consecutive '-' (e.g. "my  thing" → "my-thing", not "my--thing").
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_dash = false;
    for c in out.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push(c);
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }
    // npm forbids a leading dot or underscore; also trim stray separators.
    let trimmed = collapsed.trim_matches(|c| matches!(c, '-' | '.' | '_' | '~'));
    let mut name: String = trimmed.chars().take(214).collect();
    if name.is_empty() {
        name = "app".to_string();
    }
    name
}

/// The `package.json` body. `scripts.start` points at the chosen entry file
/// (`index.ts` or `index.js`); `"type": "module"` is non-negotiable.
pub fn package_json(name: &str, entry: &str) -> String {
    format!(
        "{{\n  \"name\": \"{name}\",\n  \"version\": \"0.0.1\",\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"start\": \"nub {entry}\"\n  }}\n}}\n"
    )
}

/// The strict modern-TS `tsconfig.json`. Mirrors `wiki/commands/init.md` exactly,
/// EXCEPT the `lib`/`types` pair, which the shared `@nubjs/types` wiring fills in
/// (so a fresh project lands type-aware): `lib: ["es2024"]`, `types: ["node",
/// "@nubjs/types"]`. We write it without `"dom"` and let the wiring add `types`.
pub const TSCONFIG_JSON: &str = r#"{
  "compilerOptions": {
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "es2024",
    "lib": ["es2024"],
    "moduleDetection": "force",
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "isolatedModules": true,
    "noUncheckedSideEffectImports": true,
    "resolveJsonModule": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noEmit": true
  }
}
"#;

/// The one-line entry file body (no imports → runs with zero install).
pub const ENTRY_BODY: &str = "console.log(\"Hello from Nub\");\n";

/// Minimal modern `.gitignore`.
pub const GITIGNORE: &str = "node_modules\n.env*\n*.log\n.DS_Store\n";

/// One-line README: just the project name as an H1.
pub fn readme(name: &str) -> String {
    format!("# {name}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_lowercases_and_replaces_spaces() {
        assert_eq!(sanitize_name("My Cool App"), "my-cool-app");
    }

    #[test]
    fn sanitize_drops_disallowed_and_collapses_dashes() {
        assert_eq!(sanitize_name("foo!!!bar"), "foobar");
        assert_eq!(sanitize_name("a  b"), "a-b");
        assert_eq!(sanitize_name("a/b\\c"), "abc");
    }

    #[test]
    fn sanitize_trims_leading_dot_and_separators() {
        // npm forbids a leading dot/underscore; we strip them.
        assert_eq!(sanitize_name(".hidden"), "hidden");
        assert_eq!(sanitize_name("-edge-"), "edge");
        assert_eq!(sanitize_name("_under_"), "under");
    }

    #[test]
    fn sanitize_keeps_valid_punctuation() {
        assert_eq!(sanitize_name("a-b_c.d~e"), "a-b_c.d~e");
    }

    #[test]
    fn sanitize_empty_falls_back_to_app() {
        assert_eq!(sanitize_name(""), "app");
        assert_eq!(sanitize_name("!!!"), "app");
        assert_eq!(sanitize_name("   "), "app");
    }

    #[test]
    fn sanitize_caps_at_214_chars() {
        let long = "a".repeat(300);
        assert_eq!(sanitize_name(&long).len(), 214);
    }

    #[test]
    fn package_json_is_valid_and_has_only_start_script() {
        let body = package_json("my-thing", "index.ts");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["name"], "my-thing");
        assert_eq!(v["version"], "0.0.1");
        assert_eq!(v["type"], "module");
        assert_eq!(v["scripts"]["start"], "nub index.ts");
        // No test/dev/build scripts (spec: start only).
        let scripts = v["scripts"].as_object().unwrap();
        assert_eq!(scripts.len(), 1, "only `start` belongs in scripts");
        // No lib/publish-shaped fields.
        for vestigial in ["main", "engines", "license", "description", "private"] {
            assert!(v.get(vestigial).is_none(), "{vestigial} must be absent");
        }
    }

    #[test]
    fn tsconfig_parses_and_carries_the_strict_settings() {
        let v: serde_json::Value = serde_json::from_str(TSCONFIG_JSON).unwrap();
        let co = &v["compilerOptions"];
        assert_eq!(co["module"], "nodenext");
        assert_eq!(co["strict"], true);
        assert_eq!(co["noEmit"], true);
        assert_eq!(co["target"], "es2024");
        // jsx / erasableSyntaxOnly intentionally absent.
        assert!(co.get("jsx").is_none());
        assert!(co.get("erasableSyntaxOnly").is_none());
    }
}
