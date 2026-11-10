use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static STORE: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();

pub fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("KAIRO_DATA_DIR") {
        return PathBuf::from(d);
    }
    Path::new(&std::env::var("LOCALAPPDATA").unwrap_or_default()).join("dev.ethone.ethra")
}

fn file() -> PathBuf {
    data_dir().join("store.json")
}

fn map() -> &'static Mutex<BTreeMap<String, String>> {
    STORE.get_or_init(|| {
        let loaded = std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok());
        Mutex::new(loaded.unwrap_or_else(|| {
            let src = std::env::var("KAIRO_MIGRATE_FROM").map(PathBuf::from).unwrap_or_else(|_| Path::new(&std::env::var("LOCALAPPDATA").unwrap_or_default()).join("dev.ethone.ethra").join("EBWebView").join("Default").join("Local Storage").join("leveldb"));
            let m = migrate(&src).unwrap_or_default();
            write(&m);
            m
        }))
    })
}

fn write(m: &BTreeMap<String, String>) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("store.json.tmp");
    if let Ok(text) = serde_json::to_string(m) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, file());
        }
    }
}

pub fn get(key: &str) -> Option<String> {
    map().lock().unwrap().get(key).cloned()
}

pub fn set(key: &str, value: String) {
    let mut m = map().lock().unwrap();
    m.insert(key.to_string(), value);
    write(&m);
}

pub fn remove(keys: &[String]) {
    let mut m = map().lock().unwrap();
    for k in keys {
        m.remove(k);
    }
    write(&m);
}

pub fn keys_with(prefix: &str) -> Vec<String> {
    map().lock().unwrap().keys().filter(|k| k.starts_with(prefix)).cloned().collect()
}

pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_str(&get(key)?).ok()
}

pub fn save<T: Serialize>(key: &str, value: &T) {
    if let Ok(t) = serde_json::to_string(value) {
        set(key, t);
    }
}

pub fn migrate(src: &Path) -> Option<BTreeMap<String, String>> {
    if !src.is_dir() {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!("kairo-ls-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).ok()?;
    for e in std::fs::read_dir(src).ok()?.flatten() {
        if e.file_name() != "LOCK" {
            let _ = std::fs::copy(e.path(), tmp.join(e.file_name()));
        }
    }
    let out = read_leveldb(&tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    out
}

fn decode(v: &[u8]) -> Option<String> {
    match v.first()? {
        0 => {
            let u: Vec<u16> = v[1..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            Some(String::from_utf16_lossy(&u))
        }
        1 => Some(v[1..].iter().map(|b| *b as char).collect()),
        _ => None,
    }
}

fn read_leveldb(dir: &Path) -> Option<BTreeMap<String, String>> {
    use rusty_leveldb::LdbIterator;
    let opts = rusty_leveldb::Options { create_if_missing: false, ..Default::default() };
    let mut db = rusty_leveldb::DB::open(dir, opts).ok()?;
    let mut it = db.new_iter().ok()?;
    let mut out = BTreeMap::new();
    it.seek_to_first();
    while let Some((k, v)) = it.next() {
        let Some(pos) = k.windows(2).position(|w| w == [0, 1]) else { continue };
        if !k.starts_with(b"_") || !k[..pos].ends_with(b"tauri.localhost") {
            continue;
        }
        let key: String = k[pos + 2..].iter().map(|b| *b as char).collect();
        if key.starts_with("rw-") {
            if let Some(text) = decode(&v) {
                out.insert(key, text);
            }
        }
    }
    Some(out)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_runes: bool,
    pub auto_spells: bool,
    pub auto_items: bool,
    pub auto_accept: bool,
    pub source: String,
    pub ov_timers: bool,
    pub ov_stats: bool,
    pub ov_gold: bool,
    pub ov_enabled: bool,
    pub ov_alerts: bool,
    pub ov_bench: bool,
    pub ov_skill: bool,
    pub ov_deaths: bool,
    pub ov_item: bool,
    pub ov_jungle: bool,
    pub rank_filter: String,
    pub jungle_route: String,
    pub auto_open_live: bool,
    pub auto_open_recap: bool,
    pub close_to_tray: bool,
    pub tilt_advice: bool,
    pub session_goal: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_runes: true,
            auto_spells: true,
            auto_items: true,
            auto_accept: false,
            source: "auto".into(),
            ov_timers: true,
            ov_stats: false,
            ov_gold: false,
            ov_enabled: true,
            ov_alerts: true,
            ov_bench: true,
            ov_skill: true,
            ov_deaths: true,
            ov_item: true,
            ov_jungle: true,
            rank_filter: "emerald_plus".into(),
            jungle_route: "red".into(),
            auto_open_live: true,
            auto_open_recap: true,
            close_to_tray: false,
            tilt_advice: true,
            session_goal: 0,
        }
    }
}

pub fn settings() -> Settings {
    load("rw-settings").unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    save("rw-settings", s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_storage_values() {
        assert_eq!(decode(&[1, b'a', 0xe9]).as_deref(), Some("aé"));
        assert_eq!(decode(&[0, 0x41, 0, 0xe9, 0]).as_deref(), Some("Aé"));
        assert_eq!(decode(&[]), None);
    }

    #[test]
    fn settings_merge_defaults() {
        let s: Settings = serde_json::from_str(r#"{"autoAccept":true,"rankFilter":"all"}"#).unwrap();
        assert!(s.auto_accept && s.auto_runes);
        assert_eq!(s.rank_filter, "all");
    }
}

#[cfg(test)]
mod real {
    #[test]
    #[ignore]
    fn migrate_real_profile() {
        let src = std::path::Path::new(&std::env::var("LOCALAPPDATA").unwrap()).join("dev.ethone.ethra").join("EBWebView").join("Default").join("Local Storage").join("leveldb");
        let m = super::migrate(&src).expect("lecture leveldb");
        let keys: Vec<String> = m.iter().map(|(k, v)| format!("{k}={}o", v.len())).collect();
        println!("{}", keys.join("\n"));
        assert!(!m.is_empty());
    }
}
