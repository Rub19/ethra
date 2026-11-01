#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{env, fs, thread};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

const PAYLOAD: &[u8] = include_bytes!("../payload/ethra.exe.gz");
const VERSION: &str = env!("KAIRO_VERSION");
const EXE: &str = "ethra.exe";
const UNINSTALLER: &str = "uninstall.exe";
const UNINST_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Kairo";
const OLD_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Ethra";
const DATA_ID: &str = "dev.ethone.ethra";
const NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Default)]
struct Args {
    silent: bool,
    passive: bool,
    update: bool,
    restart: bool,
    uninstall: bool,
    purge: bool,
    app_args: Vec<String>,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut rest = env::args().skip(1);
    while let Some(x) = rest.next() {
        match x.as_str() {
            "/S" => a.silent = true,
            "/P" => a.passive = true,
            "/UPDATE" => a.update = true,
            "/R" => a.restart = true,
            "--uninstall" => a.uninstall = true,
            "--purge" => a.purge = true,
            "/ARGS" => a.app_args = rest.by_ref().collect(),
            _ => {}
        }
    }
    a
}

#[derive(Clone, Serialize)]
struct Progress {
    pct: u8,
    label: String,
}

#[derive(Serialize)]
struct State {
    mode: &'static str,
    auto: bool,
    version: &'static str,
    installed: Option<String>,
    dir: String,
}

#[derive(Deserialize)]
struct Options {
    desktop: bool,
}

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

fn read_location(key: &str) -> Option<PathBuf> {
    let k = hkcu().open_subkey_with_flags(key, KEY_READ).ok()?;
    let v: String = k.get_value("InstallLocation").ok()?;
    let v = v.trim().trim_matches('"');
    (!v.is_empty()).then(|| PathBuf::from(v))
}

fn install_dir() -> PathBuf {
    read_location(UNINST_KEY).unwrap_or_else(|| Path::new(&env::var("LOCALAPPDATA").unwrap_or_default()).join("Kairo"))
}

fn installed_version() -> Option<String> {
    hkcu().open_subkey_with_flags(UNINST_KEY, KEY_READ).ok()?.get_value("DisplayVersion").ok()
}

fn hidden(cmd: &str) -> Command {
    let mut c = Command::new(cmd);
    c.creation_flags(NO_WINDOW);
    c
}

fn close_app() {
    let _ = hidden("taskkill").args(["/F", "/IM", EXE]).output();
    thread::sleep(Duration::from_millis(300));
}

fn remove_old_ethra() {
    let Some(dir) = read_location(OLD_KEY) else { return };
    let un = dir.join(UNINSTALLER);
    if un.exists() {
        let _ = hidden(un.to_str().unwrap_or_default()).arg("/S").arg(format!("_?={}", dir.display())).output();
        let _ = fs::remove_file(&un);
        let _ = fs::remove_dir(&dir);
    }
    let _ = hkcu().delete_subkey_all(OLD_KEY);
}

fn extract(dir: &Path, report: &dyn Fn(u8, &str)) -> Result<(), String> {
    let total = u32::from_le_bytes(PAYLOAD[PAYLOAD.len() - 4..].try_into().unwrap()) as u64;
    let tmp = dir.join(format!("{EXE}.new"));
    let mut out = fs::File::create(&tmp).map_err(|e| format!("Impossible d'écrire dans {} : {e}", dir.display()))?;
    let mut gz = GzDecoder::new(PAYLOAD);
    let mut buf = vec![0u8; 256 * 1024];
    let (mut done, mut last) = (0u64, 0u8);
    loop {
        let n = gz.read(&mut buf).map_err(|e| format!("Fichier d'installation corrompu : {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| format!("Écriture impossible : {e}"))?;
        done += n as u64;
        let pct = 8 + (done * 80 / total.max(1)).min(80) as u8;
        if pct != last {
            last = pct;
            report(pct, "Copie des fichiers");
        }
    }
    drop(out);
    let target = dir.join(EXE);
    for attempt in 0..40 {
        match fs::rename(&tmp, &target) {
            Ok(()) => return Ok(()),
            Err(e) if attempt == 39 => {
                let _ = fs::remove_file(&tmp);
                return Err(format!("Kairo est encore ouvert, ferme-le puis réessaie ({e})"));
            }
            Err(_) => thread::sleep(Duration::from_millis(250)),
        }
    }
    Ok(())
}

fn powershell(script: &str) -> Result<(), String> {
    let out = hidden("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn create_shortcuts(dir: &Path, desktop: bool) -> Result<(), String> {
    let target = dir.join(EXE).display().to_string().replace('\'', "''");
    let work = dir.display().to_string().replace('\'', "''");
    let places = if desktop { "[Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Desktop')" } else { "[Environment]::GetFolderPath('Programs')" };
    powershell(&format!(
        "$s=New-Object -ComObject WScript.Shell;foreach($d in {places}){{$l=$s.CreateShortcut((Join-Path $d 'Kairo.lnk'));$l.TargetPath='{target}';$l.WorkingDirectory='{work}';$l.Save()}}"
    ))
}

fn write_registry(dir: &Path) -> Result<(), String> {
    let (k, _) = hkcu().create_subkey(UNINST_KEY).map_err(|e| e.to_string())?;
    let exe = dir.join(EXE).display().to_string();
    let un = dir.join(UNINSTALLER).display().to_string();
    let set = |name: &str, value: &str| k.set_value(name, &value).map_err(|e| e.to_string());
    set("DisplayName", "Kairo")?;
    set("DisplayVersion", VERSION)?;
    set("Publisher", "Rub19")?;
    set("DisplayIcon", &exe)?;
    set("InstallLocation", &dir.display().to_string())?;
    set("UninstallString", &format!("\"{un}\" --uninstall"))?;
    set("QuietUninstallString", &format!("\"{un}\" --uninstall /S"))?;
    let size = fs::metadata(dir.join(EXE)).map(|m| m.len() / 1024).unwrap_or(0) as u32;
    k.set_value("EstimatedSize", &size).map_err(|e| e.to_string())?;
    k.set_value("NoModify", &1u32).map_err(|e| e.to_string())?;
    k.set_value("NoRepair", &1u32).map_err(|e| e.to_string())
}

fn do_install(desktop: bool, update: bool, report: &dyn Fn(u8, &str)) -> Result<(), String> {
    report(2, "Préparation");
    close_app();
    remove_old_ethra();
    let dir = install_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Impossible de créer {} : {e}", dir.display()))?;
    report(8, "Copie des fichiers");
    extract(&dir, report)?;
    report(90, "Finalisation");
    let me = env::current_exe().map_err(|e| e.to_string())?;
    fs::copy(&me, dir.join(UNINSTALLER)).map_err(|e| format!("Copie du désinstalleur impossible : {e}"))?;
    if !update {
        report(94, "Création des raccourcis");
        create_shortcuts(&dir, desktop)?;
    }
    write_registry(&dir)?;
    report(100, "Terminé");
    Ok(())
}

fn do_uninstall(purge: bool, report: &dyn Fn(u8, &str)) -> Result<(), String> {
    report(10, "Fermeture de Kairo");
    close_app();
    report(35, "Suppression des raccourcis");
    powershell("foreach($d in [Environment]::GetFolderPath('Programs'),[Environment]::GetFolderPath('Desktop')){Remove-Item (Join-Path $d 'Kairo.lnk') -ErrorAction SilentlyContinue};exit 0")?;
    report(60, "Suppression des fichiers");
    let dir = install_dir();
    let exe = dir.join(EXE);
    if exe.exists() {
        fs::remove_file(&exe).map_err(|e| format!("Suppression impossible : {e}"))?;
    }
    let _ = hkcu().delete_subkey_all(UNINST_KEY);
    if purge {
        report(80, "Suppression des réglages");
        for var in ["LOCALAPPDATA", "APPDATA"] {
            if let Ok(base) = env::var(var) {
                let _ = fs::remove_dir_all(Path::new(&base).join(DATA_ID));
            }
        }
    }
    report(100, "Terminé");
    Ok(())
}

// The uninstaller cannot delete itself while running, cmd does it once this process is gone.
// Only our file and an empty folder are removed, never a recursive delete on a path from the registry.
fn remove_self_later() {
    let Ok(me) = env::current_exe() else { return };
    if me.file_name().and_then(|n| n.to_str()) != Some(UNINSTALLER) {
        return;
    }
    let dir = me.parent().map(|p| p.display().to_string()).unwrap_or_default();
    let line = format!("ping -n 3 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{dir}\"", me.display());
    let _ = hidden("cmd").arg("/C").raw_arg(&line).spawn();
}

fn launch(args: &[String]) {
    let dir = install_dir();
    let _ = Command::new(dir.join(EXE)).args(args).current_dir(&dir).spawn();
}

#[tauri::command]
fn state(args: tauri::State<'_, Args>) -> State {
    State {
        mode: if args.uninstall { "uninstall" } else if args.update { "update" } else { "install" },
        auto: args.passive || args.update,
        version: VERSION,
        installed: installed_version(),
        dir: install_dir().display().to_string(),
    }
}

#[tauri::command]
async fn install(options: Options, on: Channel<Progress>, args: tauri::State<'_, Args>) -> Result<(), String> {
    let update = args.update;
    tauri::async_runtime::spawn_blocking(move || {
        do_install(options.desktop, update, &|pct, label| {
            let _ = on.send(Progress { pct, label: label.into() });
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn uninstall(purge: bool, on: Channel<Progress>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        do_uninstall(purge, &|pct, label| {
            let _ = on.send(Progress { pct, label: label.into() });
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn finish(open: bool, app: AppHandle, args: tauri::State<'_, Args>) {
    if args.uninstall {
        remove_self_later();
    } else if open || args.restart {
        launch(&args.app_args);
    }
    app.exit(0);
}

fn ensure_webview2() -> bool {
    if tauri::webview_version().is_ok() {
        return true;
    }
    let script = "$f=Join-Path $env:TEMP 'MicrosoftEdgeWebview2Setup.exe';Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $f -UseBasicParsing;Start-Process $f -ArgumentList '/silent','/install' -Wait";
    let _ = powershell(script);
    if tauri::webview_version().is_ok() {
        return true;
    }
    let _ = powershell("Add-Type -AssemblyName PresentationFramework;[System.Windows.MessageBox]::Show(\"Kairo a besoin de Microsoft Edge WebView2. Installe-le depuis le site de Microsoft, puis relance l'installation.\",'Kairo')");
    false
}

fn main() {
    let args = parse_args();
    if args.silent {
        let quiet = |_: u8, _: &str| {};
        let result = if args.uninstall { do_uninstall(args.purge, &quiet) } else { do_install(true, args.update, &quiet) };
        if result.is_ok() {
            if args.uninstall {
                remove_self_later();
            } else if args.restart {
                launch(&args.app_args);
            }
        }
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if !ensure_webview2() {
        std::process::exit(1);
    }
    tauri::Builder::default()
        .manage(args)
        .invoke_handler(tauri::generate_handler![state, install, uninstall, finish])
        .setup(|app| {
            let title = if app.state::<Args>().uninstall { "Désinstallation de Kairo" } else { "Installation de Kairo" };
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title(title)
                .inner_size(760.0, 460.0)
                .resizable(false)
                .maximizable(false)
                .decorations(false)
                .shadow(true)
                .center()
                .data_directory(env::temp_dir().join("kairo-setup-webview"))
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri");
}
