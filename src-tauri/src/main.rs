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

struct AppState {
    lcu: Mutex<Option<Lcu>>,
    http: reqwest::Client,
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
            let l = tauri::async_runtime::spawn_blocking(find_lcu).await.ok().flatten().ok_or("LCU_CLOSED")?;
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
        .manage(AppState { lcu: Mutex::new(None), http })
        .invoke_handler(tauri::generate_handler![lcu, live, opgg])
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
