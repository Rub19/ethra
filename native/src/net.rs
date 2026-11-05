use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::sync::{Mutex, OnceLock};

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

pub struct Net {
    pub http: reqwest::Client,
    lcu: Mutex<Option<Lcu>>,
    valo: Mutex<Option<Valo>>,
}

const VALO_PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";
const NO_WINDOW: u32 = 0x0800_0000;

pub fn net() -> &'static Net {
    static NET: OnceLock<Net> = OnceLock::new();
    NET.get_or_init(|| Net {
        http: reqwest::Client::builder().danger_accept_invalid_certs(true).user_agent("Kairo").build().unwrap(),
        lcu: Mutex::new(None),
        valo: Mutex::new(None),
    })
}

pub fn readonly() -> bool {
    std::env::var("KAIRO_READONLY").is_ok()
}

pub fn process_running(name: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 {
            let len = e.szExeFile.iter().position(|c| *c == 0).unwrap_or(e.szExeFile.len());
            if String::from_utf16_lossy(&e.szExeFile[..len]).eq_ignore_ascii_case(name) {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut e);
        }
        CloseHandle(snap);
        found
    }
}

fn find_lcu() -> Option<Lcu> {
    if !process_running("LeagueClientUx.exe") {
        return None;
    }
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine"])
        .creation_flags(NO_WINDOW)
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

fn find_valo_files() -> Option<(String, String, String, String)> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let lock = std::fs::read_to_string(format!("{base}\\Riot Games\\Riot Client\\Config\\lockfile")).ok()?;
    let p: Vec<&str> = lock.trim().split(':').collect();
    let log = String::from_utf8_lossy(&std::fs::read(format!("{base}\\VALORANT\\Saved\\Logs\\ShooterGame.log")).ok()?).into_owned();
    let glz = log.split("https://glz-").nth(1)?;
    let region = glz.split('-').next()?.to_string();
    let shard = glz.split('.').nth(1)?.to_string();
    Some((p.get(2)?.to_string(), p.get(3)?.to_string(), region, shard))
}

async fn read(res: reqwest::Response) -> Result<Value, String> {
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status} {text}"));
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

pub async fn lcu(method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    if method != "GET" && readonly() {
        return Err("LECTURE_SEULE".into());
    }
    let n = net();
    let cached = n.lcu.lock().unwrap().clone();
    let l = match cached {
        Some(l) => l,
        None => {
            let l = tokio::task::spawn_blocking(find_lcu).await.ok().flatten().ok_or("LCU_CLOSED")?;
            *n.lcu.lock().unwrap() = Some(l.clone());
            l
        }
    };
    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req = n.http.request(m, format!("https://127.0.0.1:{}{}", l.port, path)).basic_auth("riot", Some(&l.token));
    if let Some(b) = body {
        req = req.json(&b);
    }
    match req.send().await {
        Ok(res) => read(res).await,
        Err(e) => {
            *n.lcu.lock().unwrap() = None;
            Err(e.to_string())
        }
    }
}

pub async fn get(path: &str) -> Result<Value, String> {
    lcu("GET", path, None).await
}

pub async fn live(path: &str) -> Result<Value, String> {
    let res = net().http.get(format!("https://127.0.0.1:2999/liveclientdata/{path}")).send().await.map_err(|e| e.to_string())?;
    read(res).await
}

pub async fn opgg(tool: &str, args: &Value) -> Result<String, String> {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": args}});
    let res = net()
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

pub async fn json_url(url: &str) -> Result<Value, String> {
    read(net().http.get(url).send().await.map_err(|e| e.to_string())?).await
}

async fn valo_conn() -> Result<Valo, String> {
    let n = net();
    if let Some(v) = n.valo.lock().unwrap().clone() {
        return Ok(v);
    }
    let (port, password, region, shard) = tokio::task::spawn_blocking(find_valo_files).await.ok().flatten().ok_or("VALO_CLOSED")?;
    let ver = json_url("https://valorant-api.com/v1/version").await?;
    let version = ver["data"]["riotClientVersion"].as_str().unwrap_or_default().to_string();
    let v = Valo { port, password, region, shard, version };
    *n.valo.lock().unwrap() = Some(v.clone());
    Ok(v)
}

pub async fn valo(host: &str, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    if method != "GET" && !path.starts_with("/name-service/") && readonly() {
        return Err("LECTURE_SEULE".into());
    }
    let n = net();
    let v = valo_conn().await?;
    let local = format!("https://127.0.0.1:{}", v.port);
    let ent = match n.http.get(format!("{local}/entitlements/v1/token")).basic_auth("riot", Some(&v.password)).send().await {
        Ok(r) => read(r).await?,
        Err(e) => {
            *n.valo.lock().unwrap() = None;
            return Err(format!("VALO_CLOSED {e}"));
        }
    };
    let puuid = ent["subject"].as_str().unwrap_or_default();
    if host == "session" {
        return Ok(json!({"puuid": puuid, "region": v.region, "shard": v.shard}));
    }
    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
    let mut req = match host {
        "local" => n.http.request(m, format!("{local}{path}")).basic_auth("riot", Some(&v.password)),
        _ => {
            let base = match host {
                "pd" => format!("https://pd.{}.a.pvp.net", v.shard),
                "glz" => format!("https://glz-{}-1.{}.a.pvp.net", v.region, v.shard),
                "shared" => format!("https://shared.{}.a.pvp.net", v.shard),
                h => return Err(format!("host inconnu {h}")),
            };
            n.http
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

pub fn game_icon(game: &str) -> Option<Vec<u8>> {
    let id = match game {
        "lol" => "league_of_legends",
        "tft" => "teamfighttactics",
        _ => return None,
    };
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:/ProgramData".into());
    std::fs::read(std::path::Path::new(&base).join("Riot Games").join("Metadata").join(format!("{id}.live")).join(format!("{id}.live.ico"))).ok()
}

pub fn open_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://github.com/Rub19/ethra") {
        return Err("lien refusé".into());
    }
    std::process::Command::new("cmd").args(["/C", "start", "", url]).creation_flags(NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
}
