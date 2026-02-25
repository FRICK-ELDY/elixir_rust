//! Path: native/xtask/src/main.rs
//! Summary: workspace-layout サブコマンドで WorkspaceLayout.md を生成する xtask バイナリ

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Extensions to scan (Rust, Elixir)
const RUST_EXT: &[&str] = &["rs"];
const ELIXIR_EXT: &[&str] = &["ex", "exs"];
const ALL_EXT: &[&[&str]] = &[RUST_EXT, ELIXIR_EXT];

/// GitHub URL for Path links (owner/repo and branch)
const GITHUB_BASE: &str = "https://github.com/FRICK-ELDY/elixir_rust/blob/main";

fn main() {
    let args: Vec<String> = env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("");

    if sub == "workspace-layout" || sub == "layout" {
        workspace_layout();
    } else {
        eprintln!("Usage: cargo run -p xtask -- workspace-layout");
        eprintln!("   (cargo xtask workspace-layout if cargo-xtask is installed)");
        eprintln!("  Generate WorkspaceLayout.md with Path, Lines, Status, Summary for each file.");
        std::process::exit(1);
    }
}

fn workspace_layout() {
    let root = find_project_root();
    let root = root.as_path();

    let mut entries: Vec<FileEntry> = Vec::new();

    for dir in &["native", "lib"] {
        let dir_path = root.join(dir);
        if dir_path.is_dir() {
            scan_dir(&dir_path, root, &mut entries);
        }
    }

    // Sort by classification, then path
    entries.sort_by(|a, b| {
        let order_a = CLASSIFICATION_ORDER.iter().position(|&x| x == a.classification).unwrap_or(999);
        let order_b = CLASSIFICATION_ORDER.iter().position(|&x| x == b.classification).unwrap_or(999);
        order_a.cmp(&order_b).then_with(|| a.path.cmp(&b.path))
    });

    let md = format_output(&entries);
    let out_path = root.join("WorkspaceLayout.md");
    fs::write(&out_path, md).expect("Failed to write WorkspaceLayout.md");
    println!("Generated {}", out_path.display());
}

struct FileEntry {
    path: String,
    lines: u32,
    summary: String,
    classification: String,
}

/// Classification の表示順
const CLASSIFICATION_ORDER: &[&str] = &[
    "xtask:elixir:app",
    "xtask:elixir:engine",
    "xtask:elixir:games:mini_shooter",
    "xtask:elixir:games:vampire_survivor",
    "xtask:rust:native",
    "xtask:rust:game",
    "xtask:rust:core",
    "xtask:rust:xtask",
];

fn find_project_root() -> PathBuf {
    let cwd = env::current_dir().expect("current_dir");
    let mut p = cwd.as_path();
    loop {
        if p.join("native").is_dir() && (p.join("native").join("game_native").is_dir() || p.join("native").join("Cargo.toml").exists()) {
            return p.to_path_buf();
        }
        if let Some(parent) = p.parent() {
            p = parent;
        } else {
            return cwd;
        }
    }
}

fn scan_dir(dir: &Path, root: &Path, entries: &mut Vec<FileEntry>) {
    let read_dir = match fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return,
    };

    for e in read_dir.flatten() {
        let path = e.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == "_build" || name == ".git" || name == "node_modules" {
                continue;
            }
            scan_dir(&path, root, entries);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !ALL_EXT.iter().any(|exts| exts.contains(&ext)) {
                continue;
            }
            if let Some(rel) = path.strip_prefix(root).ok() {
                let path_str = rel.to_string_lossy().replace('\\', "/");
                let (lines, summary, classification) = analyze_file(&path, ext, &path_str);
                entries.push(FileEntry {
                    path: path_str,
                    lines,
                    summary,
                    classification,
                });
            }
        }
    }
}

fn analyze_file(path: &Path, ext: &str, path_str: &str) -> (u32, String, String) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (0, "(読込失敗)".to_string(), derive_classification_from_path(path_str)),
    };

    let raw_lines = count_effective_lines(&content, ext);
    let summary = extract_summary(&content, ext);
    let classification = derive_classification_from_path(path_str);

    // 識別用コメント（Path, Summary）が揃っている場合は Lines から 3 を引く
    let has_header = content.contains("Path:") && content.contains("Summary:");
    let lines = if has_header && raw_lines >= 3 {
        raw_lines - 3
    } else {
        raw_lines
    };

    (lines, summary, classification)
}

fn count_effective_lines(content: &str, ext: &str) -> u32 {
    let mut n = 0u32;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if ext == "rs" {
            if t.starts_with("//") || t.starts_with("/*") || t.starts_with("*/") {
                continue;
            }
        } else if ext == "ex" || ext == "exs" {
            if t.starts_with("#") {
                continue;
            }
        }
        n += 1;
    }
    n
}

fn derive_classification_from_path(path_str: &str) -> String {
    let classification = match path_str {
        s if s.starts_with("lib/app/") => "xtask:elixir:app",
        s if s.starts_with("lib/engine") => "xtask:elixir:engine",
        s if s.starts_with("lib/games/mini_shooter/") => "xtask:elixir:games:mini_shooter",
        s if s.starts_with("lib/games/vampire_survivor/") => "xtask:elixir:games:vampire_survivor",
        s if s.starts_with("native/game_native/src/core/") => "xtask:rust:core",
        s if s.starts_with("native/game_native/src/main.rs") => "xtask:rust:game",
        s if s.starts_with("native/xtask/") => "xtask:rust:xtask",
        s if s.starts_with("native/") => "xtask:rust:native",
        s if s.starts_with("lib/") => "xtask:elixir:app",
        _ => "xtask:other",
    };
    classification.to_string()
}

fn extract_summary(content: &str, ext: &str) -> String {
    for line in content.lines() {
        let t = line.trim();
        if ext == "rs" {
            if let Some(rest) = t.strip_prefix("//!") {
                let rest = rest.trim();
                if let Some(s) = rest.strip_prefix("Summary:") {
                    return s.trim().to_string();
                }
            }
        } else if ext == "ex" || ext == "exs" {
            if let Some(s) = t.strip_prefix("# Summary:") {
                return s.trim().to_string();
            }
        }
    }
    "(未設定)".to_string()
}

fn status_for_lines(lines: u32) -> &'static str {
    match lines {
        0..=4 => "⚪",
        5..=50 => "🟢",
        51..=100 => "🟡",
        101..=200 => "🟠",
        _ => "🔴",
    }
}

fn format_output(entries: &[FileEntry]) -> String {
    let mut md = String::from("# Workspace Layout（自動生成）\n\n");

    let mut current_class: &str = "";
    for e in entries {
        if e.classification != current_class {
            current_class = &e.classification;
            let display = e.classification.strip_prefix("xtask:").unwrap_or(&e.classification);
            md.push_str(&format!("## {}\n\n", display));
            md.push_str("| Path | Lines | Status | Summary |\n");
            md.push_str("|------|-------|--------|--------|\n");
        }
        let status_icon = status_for_lines(e.lines);
        let summary_escaped = e.summary.replace('|', "\\|").replace('\n', " ");
        let path_link = format!("[{}]({}/{})", e.path, GITHUB_BASE, e.path);
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            path_link, e.lines, status_icon, summary_escaped
        ));
    }

    md
}
