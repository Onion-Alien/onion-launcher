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
    /// The saved file exists but couldn't be read or kept aside (another program had it
    /// open, say): never write over it this run.
    #[serde(skip)]
    pub no_save: bool,
}

impl Config {
    /// The saved list. A file that can't be parsed is moved aside first
    /// (apps.broken.json), so the next save can't wipe it; its new path comes back too.
    /// A file that can't be read at all, or can't be moved aside, is left alone and
    /// `no_save` is set.
    pub fn load(path: &Path) -> (Self, Option<PathBuf>) {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(_) => return (Config { no_save: true, ..Config::default() }, None),
        };
        // Notepad and Windows PowerShell can save UTF-8 with a byte-order mark
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let (mut c, backup) = if text.trim().is_empty() {
            (Config::default(), None)
        } else {
            match serde_json::from_str(text) {
                Ok(c) => (c, None),
                Err(_) => match keep_aside(path) {
                    Some(b) => (Config::default(), Some(b)),
                    None => (Config { no_save: true, ..Config::default() }, None),
                },
            }
        };
        c.tidy();
        (c, backup)
    }
    /// Writes a temp file, flushes it to disk and swaps it in, so a crash or power cut
    /// mid-write never loses the list.
    pub fn save(&self, path: &Path) {
        if self.no_save {
            return;
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(s) = serde_json::to_string_pretty(self) else { return };
        let tmp = path.with_extension("json.tmp");
        let written = std::fs::File::create(&tmp).and_then(|mut f| {
            use std::io::Write;
            f.write_all(s.as_bytes())?;
            f.sync_all()
        });
        if written.is_ok() {
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

/// Moves `apps.json` to the first free `apps.broken.json`, `apps.broken-2.json`, ...
/// (copies it if it can't be moved). None if neither worked.
fn keep_aside(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let to = (1..1000)
        .map(|n| path.with_file_name(if n == 1 { format!("{stem}.broken.json") } else { format!("{stem}.broken-{n}.json") }))
        .find(|p| !p.exists())?;
    (std::fs::rename(path, &to).is_ok() || std::fs::copy(path, &to).is_ok()).then_some(to)
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
    base.join("OnionLauncher").join("apps.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("onion-launcher-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_file_is_an_empty_list() {
        let d = temp_dir("missing");
        let (c, b) = Config::load(&d.join("apps.json"));
        assert!(c.apps.is_empty() && b.is_none() && !c.no_save);
    }

    #[test]
    fn saves_and_loads_back() {
        let d = temp_dir("roundtrip");
        let p = d.join("apps.json");
        let mut c = Config::default();
        c.apps.push(App::from_path(r"C:\Windows\notepad.exe"));
        c.save(&p);
        let (back, _) = Config::load(&p);
        assert_eq!(back.apps.len(), 1);
        assert_eq!(back.apps[0].name, "notepad");
        assert!(!p.with_extension("json.tmp").exists());
    }

    #[test]
    fn reads_a_file_with_a_byte_order_mark() {
        let d = temp_dir("bom");
        let p = d.join("apps.json");
        std::fs::write(&p, "\u{feff}{\"apps\":[{\"name\":\"A\",\"path\":\"a.exe\"}]}").unwrap();
        let (c, b) = Config::load(&p);
        assert_eq!(c.apps.len(), 1);
        assert!(b.is_none());
    }

    #[test]
    fn broken_file_is_kept_aside() {
        let d = temp_dir("broken");
        let p = d.join("apps.json");
        std::fs::write(&p, "{ not json").unwrap();
        let (c, b) = Config::load(&p);
        assert!(c.apps.is_empty() && !c.no_save);
        let b = b.expect("kept aside");
        assert_eq!(std::fs::read_to_string(b).unwrap(), "{ not json");
        assert!(!p.exists());
    }

    #[test]
    fn locked_file_is_never_overwritten() {
        use std::os::windows::fs::OpenOptionsExt;
        let d = temp_dir("locked");
        let p = d.join("apps.json");
        let original = "{\"apps\":[{\"name\":\"Keep me\",\"path\":\"k.exe\"}]}";
        std::fs::write(&p, original).unwrap();
        // another program holding the file with no sharing, like a backup tool might
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&p).unwrap();
        let (mut c, b) = Config::load(&p);
        assert!(c.no_save && b.is_none());
        drop(lock);
        c.apps.push(App::from_path("new.exe"));
        c.save(&p);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), original);
    }
}
