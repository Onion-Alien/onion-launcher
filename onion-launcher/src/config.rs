//! The saved app list (%APPDATA%\OnionLauncher\apps.json) and command-line bits.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct App {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub args: String,
    /// where the picture comes from when the path itself has none (game links)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// the category section it sits in; empty = the top section
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
}

impl App {
    pub fn from_path(p: &str) -> App {
        let name = Path::new(p).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string());
        App { name, path: p.to_string(), args: String::new(), icon: String::new(), category: String::new() }
    }
    /// Tells two list entries apart even when they open the same program.
    pub fn key(&self) -> String {
        format!("{}\u{0}{}\u{0}{}", self.path, self.args, self.name)
    }
    /// What to ask Windows for this app's icon.
    pub fn icon_source(&self) -> &str {
        if self.icon.is_empty() { &self.path } else { &self.icon }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub apps: Vec<App>,
    /// category order, top to bottom
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let mut c: Config = std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        c.tidy();
        c
    }
    /// Writes a temp file and swaps it in, so a crash mid-write never loses the list.
    pub fn save(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(s) = serde_json::to_string_pretty(self) else { return };
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
    /// Every category an app uses is listed (once), and no listed category is empty.
    pub fn tidy(&mut self) {
        for a in &self.apps {
            if !a.category.is_empty() && !self.categories.contains(&a.category) {
                self.categories.push(a.category.clone());
            }
        }
        let used: std::collections::HashSet<&str> = self.apps.iter().map(|a| a.category.as_str()).collect();
        let mut seen = std::collections::HashSet::new();
        self.categories.retain(|c| used.contains(c.as_str()) && seen.insert(c.clone()));
    }
    pub fn has(&self, path: &str) -> bool {
        self.apps.iter().any(|a| a.path.eq_ignore_ascii_case(path))
    }
}

/// Value after `name` on the command line, e.g. `--config x.json`.
pub fn arg_value(args: &[String], name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).cloned()
}

/// `--config <file>`, else %APPDATA%\OnionLauncher\apps.json.
pub fn config_path(args: &[String]) -> PathBuf {
    if let Some(p) = arg_value(args, "--config") {
        return p.into();
    }
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| ".".into());
    let path = base.join("OnionLauncher").join("apps.json");
    // The app was called QuickLaunch before: on the first start, take that list over (a
    // copy; the old file stays where it is).
    let old = base.join("QuickLaunch").join("apps.json");
    if !path.exists() && old.is_file() {
        let _ = std::fs::create_dir_all(base.join("OnionLauncher"));
        let _ = std::fs::copy(&old, &path);
    }
    path
}
