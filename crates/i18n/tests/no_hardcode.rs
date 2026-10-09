//! No-hardcoded-strings lint: user-facing prose must live in the locales.
//!
//! Rule: every `"..."` literal in `crates/*/src` (excluding the `i18n` crate
//! itself, `tests/` and `examples/` dirs) that contains whitespace and is at
//! least 12 chars long must either be absent or match an entry in
//! `allowlist-technical.txt` (developer diagnostics with a documented
//! reason). Lines invoking `assert`/`expect`/`panic`/`unreachable`/`todo`/
//! `unimplemented` are bug paths, not UX, and are skipped. Spec ref:
//! `.specify/spec-06-i18n.md` section 1.

use std::fs;
use std::path::{Path, PathBuf};

const MIN_LEN: usize = 12;
const MACRO_TOKENS: [&str; 7] = [
    "assert",
    "expect",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "debug_assert",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/i18n two levels below root")
        .to_path_buf()
}

fn allowlist(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join("crates/i18n/tests/allowlist-technical.txt"))
        .expect("read allowlist");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Strip `//` comments (string-aware so `https://` inside literals survives).
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
        } else if c == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '/' {
            return &line[..i];
        }
        i += 1;
    }
    line
}

/// Extract double-quoted literals (escape-aware) from comment-free code.
fn literals(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        match &mut current {
            None => {
                if c == '"' {
                    current = Some(String::new());
                }
            }
            Some(buf) => {
                if c == '\\' {
                    if let Some(e) = chars.next() {
                        buf.push('\\');
                        buf.push(e);
                    }
                } else if c == '"' {
                    out.push(current.take().expect("open literal"));
                } else {
                    buf.push(c);
                }
            }
        }
    }
    out
}

fn is_test_or_example(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        s == "tests" || s == "examples"
    })
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("read dir");
    for entry in entries {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect_sources(&path, out);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
}

#[test]
fn no_hardcoded_user_strings() {
    let root = workspace_root();
    let allowed = allowlist(&root);
    let crates = ["core", "capture", "encode", "audio", "kms-server", "plugin"];
    let mut violations = Vec::new();
    for name in crates {
        let mut sources = Vec::new();
        collect_sources(&root.join("crates").join(name).join("src"), &mut sources);
        for path in sources {
            if is_test_or_example(&path) {
                continue;
            }
            let text = fs::read_to_string(&path).expect("read source");
            for (n, line) in text.lines().enumerate() {
                if MACRO_TOKENS.iter().any(|t| line.contains(t)) {
                    continue;
                }
                let code = strip_comment(line);
                for lit in literals(code) {
                    if lit.len() >= MIN_LEN && lit.chars().any(char::is_whitespace) {
                        // Doc comments (`///`, `//!`) carry no string literals
                        // after stripping... they may: check the code slice.
                        let trimmed = code.trim_start();
                        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                            continue;
                        }
                        if !allowed.iter().any(|a| lit.contains(a.as_str())) {
                            violations.push(format!(
                                "{}:{}: hardcoded prose: {lit:?}",
                                path.strip_prefix(&root).unwrap().display(),
                                n + 1
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "user-facing strings must live in crates/i18n/locales (or allowlist):\n{}",
        violations.join("\n")
    );
}
