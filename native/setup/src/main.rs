#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

use flate2::read::GzDecoder;
use slint::winit_030::{winit, WinitWindowAccessor};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;
use std::{env, fs, thread};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

#[cfg(not(test))]
const PAYLOAD: &[u8] = include_bytes!("../payload/ethra.exe.gz");
#[cfg(test)]
const PAYLOAD: &[u8] = &[];
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

fn parse_args(list: impl Iterator<Item = String>) -> Args {
    let mut a = Args::default();
    let mut rest = list;
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

fn extract(gz_data: &[u8], dir: &Path, report: &dyn Fn(u8, &str)) -> Result<(), String> {
    if gz_data.len() < 4 {
        return Err("Fichier d'installation vide".into());
    }
    let total = u32::from_le_bytes(gz_data[gz_data.len() - 4..].try_into().unwrap()) as u64;
    let tmp = dir.join(format!("{EXE}.new"));
    let mut out = fs::File::create(&tmp).map_err(|e| format!("Impossible d'écrire dans {} : {e}", dir.display()))?;
    let mut gz = GzDecoder::new(gz_data);
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
    powershell(&format!("$s=New-Object -ComObject WScript.Shell;foreach($d in {places}){{$l=$s.CreateShortcut((Join-Path $d 'Kairo.lnk'));$l.TargetPath='{target}';$l.IconLocation='{target},0';$l.WorkingDirectory='{work}';$l.Save()}}"))
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
    extract(PAYLOAD, &dir, report)?;
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

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Ready,
    Busy,
    Done,
    Failed,
}

struct Ui {
    win: SetupWindow,
    args: Args,
    stage: RefCell<Stage>,
}

impl Ui {
    fn show_ready(&self) {
        let w = &self.win;
        w.set_busy(false);
        w.set_failed(false);
        w.set_show_opts(true);
        w.set_uninstalling(self.args.uninstall);
        if self.args.uninstall {
            w.set_title_text("Désinstaller Kairo".into());
            w.set_lead("Kairo sera retiré de cet ordinateur.".into());
            w.set_go_label("Désinstaller".into());
        } else {
            let old = installed_version();
            w.set_title_text("Kairo".into());
            w.set_sub(format!("Version {VERSION}").into());
            w.set_lead(install_dir().display().to_string().into());
            w.set_go_label(if old.is_some() { "Mettre à jour" } else { "Installer" }.into());
        }
        w.set_alt_label("Annuler".into());
    }

    fn run(self: &Rc<Self>) {
        *self.stage.borrow_mut() = Stage::Busy;
        let w = &self.win;
        w.set_busy(true);
        w.set_show_opts(false);
        w.set_go_label("".into());
        w.set_alt_label("".into());
        w.set_sub("".into());
        w.set_progress(0.0);
        w.set_lead("Préparation".into());
        w.set_title_text(if self.args.uninstall { "Désinstallation" } else if self.args.update { "Mise à jour" } else { "Installation" }.into());
        let (uninstall, update, purge, desktop) = (self.args.uninstall, self.args.update, w.get_purge(), w.get_desktop());
        let weak = w.as_weak();
        let me = self.clone();
        let report = move |pct: u8, label: &str| {
            let (weak, label) = (weak.clone(), label.to_string());
            let _ = weak.upgrade_in_event_loop(move |w| {
                w.set_progress(pct as f32 / 100.0);
                w.set_lead(label.into());
            });
        };
        let done_weak = w.as_weak();
        thread::spawn(move || {
            let result = if uninstall { do_uninstall(purge, &report) } else { do_install(desktop, update, &report) };
            let _ = done_weak.upgrade_in_event_loop(move |_| finished(result));
        });
        UI.with(|u| *u.borrow_mut() = Some(me));
    }
}

thread_local! {
    static UI: RefCell<Option<Rc<Ui>>> = const { RefCell::new(None) };
}

fn finished(result: Result<(), String>) {
    let Some(ui) = UI.with(|u| u.borrow().clone()) else { return };
    let w = &ui.win;
    w.set_busy(false);
    match result {
        Ok(()) => {
            *ui.stage.borrow_mut() = Stage::Done;
            if ui.args.uninstall {
                w.set_title_text("Kairo est désinstallé".into());
                w.set_lead("".into());
                w.set_go_label("Fermer".into());
            } else if ui.args.passive || ui.args.update {
                if ui.args.restart {
                    launch(&ui.args.app_args);
                }
                let _ = slint::quit_event_loop();
            } else {
                w.set_title_text("Kairo est prêt".into());
                w.set_lead("".into());
                w.set_go_label(if w.get_open_after() { "Ouvrir Kairo" } else { "Fermer" }.into());
            }
        }
        Err(e) => {
            *ui.stage.borrow_mut() = Stage::Failed;
            w.set_failed(true);
            w.set_title_text("Échec".into());
            w.set_lead(e.into());
            w.set_go_label("Réessayer".into());
            w.set_alt_label("Fermer".into());
        }
    }
}

fn leave(ui: &Rc<Ui>) {
    if ui.args.uninstall {
        remove_self_later();
    }
    let _ = slint::quit_event_loop();
}

fn main() {
    let args = parse_args(env::args().skip(1));
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
    let win = SetupWindow::new().unwrap();
    let auto = !args.uninstall && (args.passive || args.update);
    let ui = Rc::new(Ui { win: win.clone_strong(), args, stage: RefCell::new(Stage::Ready) });
    UI.with(|u| *u.borrow_mut() = Some(ui.clone()));
    ui.show_ready();
    {
        let ui = ui.clone();
        win.on_go(move || match *ui.stage.borrow() {
            Stage::Done => {
                if !ui.args.uninstall && ui.win.get_open_after() {
                    launch(&ui.args.app_args);
                }
                leave(&ui);
            }
            Stage::Ready | Stage::Failed => {
                let again = ui.clone();
                slint::Timer::single_shot(Duration::ZERO, move || again.run());
            }
            Stage::Busy => {}
        });
    }
    win.on_alt(|| {
        let _ = slint::quit_event_loop();
    });
    win.on_close(|| {
        let _ = slint::quit_event_loop();
    });
    let weak = win.as_weak();
    win.on_minimize(move || {
        if let Some(w) = weak.upgrade() {
            w.window().set_minimized(true);
        }
    });
    let weak = win.as_weak();
    win.on_drag(move || {
        if let Some(w) = weak.upgrade() {
            w.window().with_winit_window(|x: &winit::window::Window| {
                let _ = x.drag_window();
            });
        }
    });
    win.show().unwrap();
    if auto {
        let ui = ui.clone();
        slint::Timer::single_shot(Duration::from_millis(200), move || ui.run());
    }
    slint::run_event_loop_until_quit().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;

    #[test]
    fn args() {
        let a = parse_args(["/P", "/UPDATE", "/R", "/ARGS", "x", "y"].iter().map(|s| s.to_string()));
        assert!(a.passive && a.update && a.restart && !a.silent);
        assert_eq!(a.app_args, vec!["x", "y"]);
        assert!(parse_args(["--uninstall", "--purge"].iter().map(|s| s.to_string())).purge);
    }

    #[test]
    fn extracts_payload() {
        let dir = env::temp_dir().join(format!("kairo-setup-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut gz = GzEncoder::new(vec![], flate2::Compression::fast());
        gz.write_all(b"contenu de l'exe").unwrap();
        let data = gz.finish().unwrap();
        extract(&data, &dir, &|_, _| {}).unwrap();
        assert_eq!(fs::read(dir.join(EXE)).unwrap(), b"contenu de l'exe");
        assert!(extract(b"", &dir, &|_, _| {}).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
