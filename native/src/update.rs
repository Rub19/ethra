use crate::net::{net, open_url};
use crate::{on_ui, spawn, ss, App, UpdateData};
use base64::Engine;
use serde_json::Value;
use std::time::Duration;

const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEI4RjNGOEI2QTNERUU0QjgKUldTNDVONmp0dmp6dUtlMVlXWENNWG9ta0pCZ09FZWFkWUF1YUEyYTR1WW5VTWVhbHQyM08zYTkK";
const ENDPOINT: &str = "https://github.com/Rub19/ethra/releases/latest/download/latest.json";
pub const REPO: &str = "https://github.com/Rub19/ethra";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Default)]
pub struct Signed {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub signature: String,
}

#[derive(Default)]
pub struct UpdateState {
    pub signed: Option<Signed>,
    pub fallback: Option<(String, String)>,
    pub later: String,
    pub state: String,
    pub progress: f32,
}

fn parts(v: &str) -> Vec<i64> {
    v.trim_start_matches('v').split('.').map(|x| x.parse().unwrap_or(0)).collect()
}

pub fn newer(a: &str, b: &str) -> bool {
    let (x, y) = (parts(a), parts(b));
    for i in 0..3 {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p > q;
        }
    }
    false
}

fn current() -> String {
    std::env::var("KAIRO_FAKE_VERSION").unwrap_or_else(|_| VERSION.to_string())
}

pub fn parse_latest(v: &Value) -> Option<Signed> {
    let p = &v["platforms"]["windows-x86_64"];
    Some(Signed { version: v["version"].as_str()?.trim_start_matches('v').to_string(), notes: v["notes"].as_str().unwrap_or_default().to_string(), url: p["url"].as_str()?.to_string(), signature: p["signature"].as_str()?.to_string() })
}

pub fn verify(bytes: &[u8], signature_b64: &str) -> Result<(), String> {
    let e = base64::engine::general_purpose::STANDARD;
    let key = String::from_utf8(e.decode(PUBKEY).map_err(|x| x.to_string())?).map_err(|x| x.to_string())?;
    let pk = minisign_verify::PublicKey::decode(&key).map_err(|x| x.to_string())?;
    let sig = String::from_utf8(e.decode(signature_b64.trim()).map_err(|x| x.to_string())?).map_err(|x| x.to_string())?;
    let s = minisign_verify::Signature::decode(&sig).map_err(|x| x.to_string())?;
    pk.verify(bytes, &s, false).map_err(|x| x.to_string())
}

pub fn clean_notes(body: &str) -> String {
    for l in body.split('\n') {
        let t: String = l.trim_start_matches(|c: char| c == '#' || c == '-' || c.is_whitespace()).chars().filter(|c| !matches!(c, '*' | '_' | '`')).collect();
        let t = t.trim();
        if !t.is_empty() && !t.to_lowercase().starts_with("nouveaut") {
            return t.to_string();
        }
    }
    String::new()
}

async fn check() {
    let signed = match net().http.get(ENDPOINT).send().await {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.ok().as_ref().and_then(parse_latest),
        _ => None,
    };
    let fallback = if signed.is_some() {
        None
    } else {
        match net().http.get("https://api.github.com/repos/Rub19/ethra/releases/latest").send().await {
            Ok(r) if r.status().is_success() => r.json::<Value>().await.ok().and_then(|j| Some((j["tag_name"].as_str()?.trim_start_matches('v').to_string(), j["html_url"].as_str()?.to_string()))),
            _ => None,
        }
    };
    on_ui(move |a| {
        let cur = current();
        a.update.signed = signed.filter(|s| newer(&s.version, &cur));
        a.update.fallback = fallback.filter(|f| newer(&f.0, &cur));
        a.settings.checked = true;
        render(a);
        crate::page_misc::render_settings(a);
    });
}

pub fn start() {
    spawn(async {
        loop {
            check().await;
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        }
    });
}

pub fn latest_info(a: &App) -> Option<(String, String)> {
    if let Some(s) = &a.update.signed {
        return Some((s.version.clone(), format!("{REPO}/releases/latest")));
    }
    a.update.fallback.clone()
}

pub fn render(a: &mut App) {
    let st = &a.update;
    let Some((version, _)) = latest_info(a) else {
        a.data().set_update(UpdateData::default());
        return;
    };
    if st.later == version {
        a.data().set_update(UpdateData::default());
        return;
    }
    let err = st.state == "error";
    let busy = st.state == "downloading" || st.state == "installing";
    let notes = st.signed.as_ref().map(|s| clean_notes(&s.notes)).unwrap_or_default();
    a.data().set_update(UpdateData {
        show: true,
        error: err,
        title: ss(if err { "La mise à jour a échoué" } else { "Mise à jour disponible" }),
        text: ss(if err {
            "Télécharge la nouvelle version depuis la page de Kairo.".to_string()
        } else if st.state == "installing" {
            "Installation, Kairo va redémarrer…".to_string()
        } else {
            format!("Kairo {version} est prête. Tes réglages sont conservés.")
        }),
        notes: ss(if st.state.is_empty() { notes } else { String::new() }),
        progress: st.progress,
        busy,
        progress_text: ss(if st.state == "installing" { "Installation…".to_string() } else { format!("Téléchargement {} %", (st.progress * 100.0).round()) }),
        primary: ss(if err {
            "Ouvrir la page"
        } else if st.signed.is_some() {
            "Redémarrer et mettre à jour"
        } else {
            "Télécharger"
        }),
    });
}

fn install(a: &mut App) {
    let Some(s) = a.update.signed.clone() else {
        let url = a.update.fallback.as_ref().map(|f| f.1.clone()).unwrap_or_else(|| format!("{REPO}/releases/latest"));
        let _ = open_url(&url);
        return;
    };
    if a.update.state == "error" {
        let _ = open_url(&format!("{REPO}/releases/latest"));
        return;
    }
    a.update.state = "downloading".into();
    a.update.progress = 0.0;
    render(a);
    spawn(async move {
        let r: Result<std::path::PathBuf, String> = async {
            let mut res = net().http.get(&s.url).send().await.map_err(|e| e.to_string())?;
            if !res.status().is_success() {
                return Err(res.status().to_string());
            }
            let total = res.content_length().unwrap_or(0);
            let mut buf = Vec::with_capacity(total as usize);
            while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
                buf.extend_from_slice(&chunk);
                if total > 0 {
                    let p = buf.len() as f32 / total as f32;
                    on_ui(move |a| {
                        a.update.progress = p;
                        render(a);
                    });
                }
            }
            verify(&buf, &s.signature)?;
            let path = std::env::temp_dir().join(format!("Kairo_{}_setup.exe", s.version));
            std::fs::write(&path, &buf).map_err(|e| e.to_string())?;
            Ok(path)
        }
        .await;
        on_ui(move |a| match r {
            Ok(path) => {
                a.update.state = "installing".into();
                render(a);
                if std::process::Command::new(&path).args(["/P", "/UPDATE", "/R"]).spawn().is_ok() {
                    let _ = slint::quit_event_loop();
                } else {
                    a.update.state = "error".into();
                    render(a);
                }
            }
            Err(_) => {
                a.update.state = "error".into();
                render(a);
            }
        });
    });
}

pub fn action(a: &mut App, key: &str) -> bool {
    match key {
        "update-go" => install(a),
        "update-later" => {
            a.update.later = latest_info(a).map(|x| x.0).unwrap_or_default();
            render(a);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions() {
        assert!(newer("v0.3.5", "0.3.4"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.3.4", "0.3.4"));
        assert!(!newer("0.3.3", "0.3.4"));
    }

    #[test]
    fn latest() {
        let v = json!({"version": "0.4.0", "notes": "n", "platforms": {"windows-x86_64": {"url": "u", "signature": "s"}}});
        assert_eq!(parse_latest(&v).unwrap().version, "0.4.0");
        assert!(parse_latest(&json!({})).is_none());
    }

    #[test]
    fn notes() {
        assert_eq!(clean_notes("## Nouveautés\n- **Mieux** ici"), "Mieux ici");
        assert_eq!(clean_notes(""), "");
    }

    #[test]
    fn rejects_bad_signature() {
        assert!(verify(b"abc", "bm90IGEgc2lnbmF0dXJl").is_err());
    }

    #[test]
    #[ignore]
    fn real_release_signature() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let c = reqwest::Client::builder().user_agent("Kairo").build().unwrap();
            let v: Value = c.get(ENDPOINT).send().await.unwrap().json().await.unwrap();
            let s = parse_latest(&v).unwrap();
            let bytes = c.get(&s.url).send().await.unwrap().bytes().await.unwrap();
            verify(&bytes, &s.signature).unwrap();
            let mut bad = bytes.to_vec();
            bad[100] ^= 1;
            assert!(verify(&bad, &s.signature).is_err());
        });
    }
}
