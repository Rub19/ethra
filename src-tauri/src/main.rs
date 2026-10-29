#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Clone)]
struct Lcu {
    port: String,
    token: String,
}

#[derive(Clone)]
struct Valo {
    port: String,
    password: String,
    region: String,
    shard: String,
    version: String,
}

struct AppState {
    lcu: Mutex<Option<Lcu>>,
    valo: Mutex<Option<Valo>>,
    http: reqwest::Client,
}

const VALO_PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

fn find_valo_files() -> Option<(String, String, String, String)> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let lock = std::fs::read_to_string(format!("{base}\\Riot Games\\Riot Client\\Config\\lockfile")).ok()?;
    let p: Vec<&str> = lock.trim().split(':').collect();
    let log = std::fs::read_to_string(format!("{base}\\VALORANT\\Saved\\Logs\\ShooterGame.log")).ok()?;
    let glz = log.split("https://glz-").nth(1)?;
    let region = glz.split('-').next()?.to_string();
    let shard = glz.split('.').nth(1)?.to_string();
    Some((p.get(2)?.to_string(), p.get(3)?.to_string(), region, shard))
}

async fn valo_conn(state: &AppState) -> Result<Valo, String> {
    if let Some(v) = state.valo.lock().unwrap().clone() {
        return Ok(v);
    }
    let (port, password, region, shard) = tauri::async_runtime::spawn_blocking(find_valo_files).await.ok().flatten().ok_or("VALO_CLOSED")?;
    let ver = read(state.http.get("https://valorant-api.com/v1/version").send().await.map_err(|e| e.to_string())?).await?;
    let version = ver["data"]["riotClientVersion"].as_str().unwrap_or_default().to_string();
    let v = Valo { port, password, region, shard, version };
    *state.valo.lock().unwrap() = Some(v.clone());
    Ok(v)
}

#[tauri::command]
async fn valo(state: tauri::State<'_, AppState>, host: String, method: String, path: String, body: Option<Value>) -> Result<Value, String> {
    let v = valo_conn(&state).await?;
    let local = format!("https://127.0.0.1:{}", v.port);
    let ent = match state.http.get(format!("{local}/entitlements/v1/token")).basic_auth("riot", Some(&v.password)).send().await {
        Ok(r) => read(r).await?,
        Err(e) => {
            *state.valo.lock().unwrap() = None;
            return Err(format!("VALO_CLOSED {e}"));
        }
    };
    let puuid = ent["subject"].as_str().unwrap_or_default();
    if host == "session" {
        return Ok(serde_json::json!({"puuid": puuid, "region": v.region, "shard": v.shard}));
    }
    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req = match host.as_str() {
        "local" => state.http.request(m, format!("{local}{path}")).basic_auth("riot", Some(&v.password)),
        _ => {
            let base = match host.as_str() {
                "pd" => format!("https://pd.{}.a.pvp.net", v.shard),
                "glz" => format!("https://glz-{}-1.{}.a.pvp.net", v.region, v.shard),
                "shared" => format!("https://shared.{}.a.pvp.net", v.shard),
                h => return Err(format!("host inconnu {h}")),
            };
            state
                .http
                .request(m, format!("{base}{path}"))
                .bearer_auth(ent["accessToken"].as_str().unwrap_or_default())
                .header("X-Riot-Entitlements-JWT", ent["token"].as_str().unwrap_or_default())
                .header("X-Riot-ClientVersion", &v.version)
                .header("X-Riot-ClientPlatform", VALO_PLATFORM)
        }
    };
    if let Some(b) = body {
        req = req.json(&b);
    }
    read(req.send().await.map_err(|e| e.to_string())?).await
}

// ponytail: spawns powershell on every call while the client is closed, switch to a process watcher if CPU shows up
fn find_lcu() -> Option<Lcu> {
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine"])
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    let cmd = String::from_utf8_lossy(&out.stdout).to_string();
    let arg = |k: &str| {
        cmd.split(&format!("--{k}="))
            .nth(1)
            .map(|r| r.split(['"', ' ']).next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
    };
    Some(Lcu { port: arg("app-port")?, token: arg("remoting-auth-token")? })
}

async fn read(res: reqwest::Response) -> Result<Value, String> {
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status} {text}"));
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

#[tauri::command]
async fn lcu(state: tauri::State<'_, AppState>, method: String, path: String, body: Option<Value>) -> Result<Value, String> {
    let cached = state.lcu.lock().unwrap().clone();
    let l = match cached {
        Some(l) => l,
        None => {
            let l = tauri::async_runtime::spawn_blocking(find_lcu).await.ok().flatten().ok_or_else(|| {
                eprintln!("[lcu] client introuvable");
                "LCU_CLOSED".to_string()
            })?;
            *state.lcu.lock().unwrap() = Some(l.clone());
            l
        }
    };
    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req = state.http.request(m, format!("https://127.0.0.1:{}{}", l.port, path)).basic_auth("riot", Some(&l.token));
    if let Some(b) = body {
        req = req.json(&b);
    }
    match req.send().await {
        Ok(res) => read(res).await,
        Err(e) => {
            eprintln!("[lcu] {path}: {e:?}");
            *state.lcu.lock().unwrap() = None;
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn live(state: tauri::State<'_, AppState>, path: String) -> Result<Value, String> {
    let res = state.http.get(format!("https://127.0.0.1:2999/liveclientdata/{path}")).send().await.map_err(|e| e.to_string())?;
    read(res).await
}

#[tauri::command]
async fn opgg(state: tauri::State<'_, AppState>, tool: String, args: Value) -> Result<String, String> {
    let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": args}});
    let res = state
        .http
        .post("https://mcp-api.op.gg/mcp")
        .header("Accept", "application/json, text/event-stream")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v = read(res).await?;
    let text = v["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    if v.get("error").is_some() || v["result"]["isError"] == true || text.is_empty() {
        return Err(format!("OPGG {}", if text.is_empty() { v.to_string() } else { text }));
    }
    Ok(text)
}

fn main() {
    let http = reqwest::Client::builder().danger_accept_invalid_certs(true).build().unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState { lcu: Mutex::new(None), valo: Mutex::new(None), http })
        .invoke_handler(tauri::generate_handler![lcu, live, opgg, valo])
        .register_asynchronous_uri_scheme_protocol("ethone", |ctx, req, responder| {
            let http = ctx.app_handle().state::<AppState>().http.clone();
            let path = req.uri().path().to_string();
            tauri::async_runtime::spawn(async move {
                let fetched = async {
                    let r = http.get(format!("https://ethone.dev/avatars/library{path}")).send().await.ok()?;
                    if !r.status().is_success() {
                        return None;
                    }
                    let ct = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("image/webp").to_string();
                    Some((ct, r.bytes().await.ok()?.to_vec()))
                }
                .await;
                let res = match fetched {
                    Some((ct, body)) => tauri::http::Response::builder().header("Content-Type", ct).header("Cache-Control", "max-age=604800").body(body),
                    None => tauri::http::Response::builder().status(404).body(Vec::new()),
                };
                responder.respond(res.unwrap());
            });
        })
        .setup(|app| {
            let o = app.get_webview_window("overlay").unwrap();
            if let Some(m) = o.primary_monitor()? {
                o.set_position(*m.position())?;
                o.set_size(*m.size())?;
            }
            o.set_ignore_cursor_events(true)?;
            Ok(())
        })
        .on_window_event(|w, e| {
            if w.label() == "main" && matches!(e, tauri::WindowEvent::Destroyed) {
                w.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("tauri");
}
