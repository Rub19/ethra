#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "e2e")]
mod e2e;
mod build_data;
mod history;
mod hotkey;
mod images;
mod jungle;
mod lol;
mod net;
mod opgg;
mod opgg_parse;
mod overlay;
mod page_builds;
mod page_misc;
mod page_profile;
mod page_valo;
mod store;
mod text;
mod tft_api;
mod tray;
mod update;
mod valo_api;

slint::include_modules!();

use lol::{Summoner, DD};
use slint::winit_030::{winit, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
pub static OVERLAY_NEXT: AtomicBool = AtomicBool::new(false);

pub fn spawn<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    RT.get().unwrap().spawn(f);
}

thread_local! {
    pub static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

pub fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

pub fn on_ui(f: impl FnOnce(&mut App) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || {
        with_app(f);
    });
}

pub fn ms<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

pub fn ss(s: impl AsRef<str>) -> SharedString {
    s.as_ref().into()
}

pub fn opt(key: impl AsRef<str>, label: impl AsRef<str>, on: bool) -> Opt {
    Opt { key: ss(key), label: ss(label), on, ..Default::default() }
}

pub fn stat(value: impl AsRef<str>, label: impl AsRef<str>, tone: i32) -> Stat {
    Stat { value: ss(value), label: ss(label), tone }
}

pub fn good(ok: bool) -> i32 {
    if ok {
        1
    } else {
        2
    }
}

pub struct App {
    pub ui: AppWindow,
    pub stack: Vec<String>,
    pub at: usize,
    pub game: String,
    pub phase: Option<String>,
    pub self_: Option<Summoner>,
    pub d: Option<Arc<DD>>,
    pub vself: Option<String>,
    pub vph: Option<String>,
    pub vopen: bool,
    pub key: String,
    pub gen: u64,
    pub imported: i64,
    pub toast_timer: slint::Timer,
    pub search: page_misc::SearchState,
    pub profile: page_profile::ProfileState,
    pub match_: page_profile::MatchState,
    pub tier: page_builds::TierState,
    pub champs: page_builds::ChampsState,
    pub builds: page_builds::BuildsState,
    pub live: page_builds::LiveState,
    pub settings: page_misc::SettingsState,
    pub tft: page_misc::TftState,
    pub picker: page_misc::PickerState,
    pub update: update::UpdateState,
    pub valo: page_valo::ValoState,
    pub script: Vec<String>,
}

impl App {
    pub fn data(&self) -> Data<'_> {
        self.ui.global::<Data>()
    }

    pub fn route(&self) -> String {
        self.stack[self.at].clone()
    }

    pub fn page(&self) -> String {
        self.route().split('/').next().unwrap_or_default().to_string()
    }

    pub fn toast(&mut self, msg: impl AsRef<str>) {
        let d = self.data();
        d.set_toast(ss(msg));
        d.set_toast_show(true);
        self.toast_timer.start(slint::TimerMode::SingleShot, Duration::from_secs(6), || {
            with_app(|a| a.data().set_toast_show(false));
        });
    }

    pub fn go(&mut self, r: &str) {
        if self.route() == r {
            return;
        }
        self.stack.truncate(self.at + 1);
        self.stack.push(r.to_string());
        self.at = self.stack.len() - 1;
        self.after_nav();
    }

    pub fn after_nav(&mut self) {
        let owner = owner_of(&self.page());
        if owner.as_deref() == Some(self.game.as_str()) {
            store::set(&format!("rw-route-{}", self.game), self.route());
        }
        self.ui.set_account(false);
        images::trim_if_needed();
        self.refresh_view();
        self.ui.invoke_restore_focus();
    }

    pub fn switch_game(&mut self, g: &str) {
        self.ui.set_gmenu(false);
        self.ui.set_gsticky(false);
        if g == self.game {
            return;
        }
        self.game = g.to_string();
        store::set("rw-game", g.to_string());
        self.ui.global::<Theme>().set_game(ss(g));
        self.data().set_game(ss(g));
        page_misc::render_games(self);
        let saved = store::get(&format!("rw-route-{g}")).map(|r| normalize_route(&r)).filter(|r| !r.is_empty());
        let home = match g {
            "valo" => "v-profil",
            "tft" => "t-profil",
            _ => "profil",
        };
        let target = saved.unwrap_or_else(|| home.to_string());
        if self.route() == target {
            self.refresh_view();
        } else {
            self.go(&target);
        }
    }

    pub fn refresh_view(&mut self) {
        let page = self.page();
        let route = self.route();
        let needs_client = self.phase.is_none() && !["reglages", "overlays", "champions", "champion", "tierlist", "v-profil", "v-match", "v-partie", "v-agents", "t-decks", "t-profil"].contains(&page.as_str());
        let needs_valo = self.vself.is_none() && ["v-profil", "v-match", "v-partie"].contains(&page.as_str());
        let mut notice = ("", "");
        let view = if page == "t-profil" && self.phase.is_none() {
            notice = ("Le client League of Legends est fermé", "Lance-le pour voir ton rang et tes parties TFT. En attendant, voici les decks méta.");
            "t-decks".to_string()
        } else if needs_valo {
            notice = ("VALORANT est fermé", "Lance le jeu pour voir ton profil, tes parties et les rangs en direct. En attendant, voici la méta des agents.");
            "v-agents".to_string()
        } else if needs_client {
            notice = ("Le client League of Legends est fermé", "Lance-le pour voir ton profil, ton historique et ta partie en cours. En attendant, voici la tier list du patch.");
            "tierlist".to_string()
        } else {
            page.clone()
        };
        let d = self.data();
        d.set_notice_title(ss(notice.0));
        d.set_notice_text(ss(notice.1));
        d.set_can_back(self.at > 0);
        d.set_can_fwd(self.at + 1 < self.stack.len());
        self.render_tabs();
        let key = format!("{view}|{route}");
        if key != self.key {
            self.key = key;
            self.gen += 1;
            self.start_view(&view, &route);
            let d = self.data();
            d.set_view(ss(""));
            let v = view.clone();
            slint::Timer::single_shot(Duration::from_millis(0), move || {
                with_app(|a| a.data().set_view(ss(&v)));
            });
        }
        self.render_shell();
    }

    fn start_view(&mut self, view: &str, route: &str) {
        let parts: Vec<&str> = route.split('/').collect();
        let arg = |i: usize| parts.get(i).filter(|_| parts[0] == view).map(|s| s.to_string());
        match view {
            "profil" => page_profile::open_profile(self, arg(1)),
            "match" => page_profile::open_match(self, arg(1).and_then(|x| x.parse().ok()).unwrap_or(0), arg(2).unwrap_or_default()),
            "tierlist" => page_builds::open_tier(self),
            "champions" => page_builds::open_champs(self),
            "champion" => page_builds::open_builds(self, arg(1).and_then(|x| x.parse().ok()).unwrap_or(0), arg(2).unwrap_or_default(), false, "ranked"),
            "partie" => page_builds::open_live(self),
            "overlays" => page_misc::render_overlays(self),
            "reglages" => page_misc::open_settings(self),
            "t-profil" => page_misc::open_tft_profile(self),
            "t-decks" => page_misc::open_tft_decks(self),
            "v-profil" => page_valo::open_profile(self, arg(1)),
            "v-match" => page_valo::open_match(self, arg(1).unwrap_or_default(), arg(2).unwrap_or_default()),
            "v-partie" => page_valo::open_live(self),
            "v-agents" => page_valo::open_agents(self),
            _ => {}
        }
    }

    pub fn reload(&mut self) {
        self.key.clear();
        self.refresh_view();
        self.ui.invoke_restore_focus();
    }

    pub fn render_tabs(&self) {
        let page = self.page();
        let route = self.route();
        let foreign = (page == "profil" && route.contains('/') && !self.self_.as_ref().is_some_and(|s| route.ends_with(&s.puuid))) || (page == "v-profil" && route.contains('/') && !self.vself.as_ref().is_some_and(|s| route.ends_with(s)));
        let active = if page == "champion" {
            "champions".to_string()
        } else if foreign {
            String::new()
        } else {
            page
        };
        let tabs: &[(&str, &str)] = match self.game.as_str() {
            "tft" => &[("t-profil", "Profil"), ("t-decks", "Decks méta"), ("reglages", "Réglages")],
            "valo" => &[("v-profil", "Profil"), ("v-agents", "Agents"), ("v-partie", "Partie en cours"), ("reglages", "Réglages")],
            _ => &[("profil", "Profil"), ("tierlist", "Tier list"), ("champions", "Champions"), ("partie", "Partie en cours"), ("overlays", "Overlays"), ("reglages", "Réglages")],
        };
        let live_route = if self.game == "valo" { "v-partie" } else { "partie" };
        let in_game = self.in_game();
        let list = tabs
            .iter()
            .map(|(k, l)| {
                let mut o = opt(format!("go:{k}"), l, active == *k);
                if *k == live_route && in_game {
                    o.sub = ss("live");
                }
                o
            })
            .collect();
        self.data().set_tabs(ms(list));
    }

    pub fn in_game(&self) -> bool {
        if self.game == "valo" {
            matches!(self.vph.as_deref(), Some("PREGAME") | Some("INGAME"))
        } else {
            matches!(self.phase.as_deref(), Some("ChampSelect") | Some("InProgress") | Some("Reconnect") | Some("GameStart"))
        }
    }

    pub fn render_shell(&self) {
        let d = self.data();
        let valo = self.game == "valo";
        let tft = self.game == "tft";
        d.set_searchable(!tft);
        d.set_search_hint(ss(if valo { "Joueur croisé" } else { "Joueur#TAG ou champion" }));
        let connected = if valo { self.vopen } else { self.phase.is_some() };
        d.set_client_on(connected);
        d.set_client_text(ss(if valo {
            if self.vopen {
                "VALORANT ouvert"
            } else {
                "VALORANT fermé"
            }
        } else if self.phase.is_none() {
            "Client fermé"
        } else {
            "Connexion…"
        }));
        let acc = !valo && self.self_.is_some() && self.d.is_some();
        d.set_has_account(acc);
        if let (true, Some(s), Some(dd)) = (acc, &self.self_, &self.d) {
            d.set_acc_avatar(ss(page_misc::avatar_or(&lol::img_icon(dd, s.profile_icon_id))));
            d.set_acc_name(ss(&s.game_name));
            d.set_acc_tag(ss(&s.tag_line));
            d.set_acc_level(ss(s.summoner_level.to_string()));
        }
        let page = self.page();
        let live_route = if valo { "v-partie" } else { "partie" };
        let banner = self.in_game() && page != live_route;
        d.set_banner(banner);
        if banner {
            let title = if valo {
                if self.vph.as_deref() == Some("PREGAME") {
                    "Sélection des agents"
                } else {
                    "Partie en cours"
                }
            } else if self.phase.as_deref() == Some("ChampSelect") {
                "Sélection des champions"
            } else {
                "Partie en cours"
            };
            d.set_banner_title(ss(title));
            d.set_banner_text(ss(if valo { "Voir les rangs des joueurs" } else { "Voir les équipes, rangs et builds" }));
        }
    }
}

fn owner_of(page: &str) -> Option<String> {
    if page.starts_with("v-") {
        Some("valo".into())
    } else if page.starts_with("t-") {
        Some("tft".into())
    } else if page == "reglages" {
        None
    } else {
        Some("lol".into())
    }
}

pub fn normalize_route(raw: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else { return raw.to_string() };
    let s = |k: &str| v[k].as_str().map(String::from);
    let Some(page) = s("page") else { return String::new() };
    let mut out = page.clone();
    match page.as_str() {
        "profil" | "v-profil" => {
            if let Some(p) = s("puuid") {
                out += &format!("/{p}");
            }
        }
        "match" => out += &format!("/{}/{}", v["gameId"], s("puuid").unwrap_or_default()),
        "v-match" => out += &format!("/{}/{}", s("id").unwrap_or_default(), s("puuid").unwrap_or_default()),
        "champion" => {
            out += &format!("/{}", v["champ"]);
            if let Some(r) = s("role") {
                out += &format!("/{r}");
            }
        }
        _ => {}
    }
    out
}

fn act(key: &str) {
    let key = key.to_string();
    let run = move || {
        with_app(|a| dispatch(a, &key));
    };
    run();
}

pub fn dispatch_pub(a: &mut App, key: &str) {
    dispatch(a, key);
}

fn dispatch(a: &mut App, key: &str) {
    if let Some(r) = key.strip_prefix("go:") {
        if r == "profil" && a.game != "lol" {
            a.switch_game("lol");
        }
        a.go(r);
        return;
    }
    if let Some(g) = key.strip_prefix("game:") {
        a.switch_game(g);
        return;
    }
    match key {
        "kbd" => a.ui.global::<Motion>().set_keyboard(true),
        "back" => {
            if a.at > 0 {
                a.at -= 1;
                a.after_nav();
            }
        }
        "fwd" => {
            if a.at + 1 < a.stack.len() {
                a.at += 1;
                a.after_nav();
            }
        }
        "drag" => {
            a.ui.window().with_winit_window(|w: &winit::window::Window| {
                let _ = w.drag_window();
            });
        }
        "win-min" => a.ui.window().set_minimized(true),
        "win-max" => {
            let m = !a.ui.window().is_maximized();
            a.ui.window().set_maximized(m);
            a.data().set_maximized(m);
        }
        "win-close" => {
            if store::settings().close_to_tray && !offscreen() {
                let _ = a.ui.hide();
            } else {
                let _ = slint::quit_event_loop();
            }
        }
        "go-live" => {
            let r = if a.game == "valo" { "v-partie" } else { "partie" };
            a.go(r);
        }
        "toast-close" => a.data().set_toast_show(false),
        "retry" => a.reload(),
        _ => {
            if let Some(dir) = key.strip_prefix("resize:") {
                use winit::window::ResizeDirection as R;
                let d = match dir {
                    "w" => R::West,
                    "e" => R::East,
                    "s" => R::South,
                    "n" => R::North,
                    "se" => R::SouthEast,
                    _ => R::SouthWest,
                };
                a.ui.window().with_winit_window(|w: &winit::window::Window| {
                    let _ = w.drag_resize_window(d);
                });
                return;
            }
            let _ = page_misc::action(a, key) || page_profile::action(a, key) || page_builds::action(a, key) || page_valo::action(a, key) || update::action(a, key);
        }
    }
}

fn poll_lcu() {
    spawn(async {
        loop {
            let p = net::get("/lol-gameflow/v1/gameflow-phase").await.ok().map(|v| v.as_str().unwrap_or_default().to_string());
            let phase = p.clone();
            on_ui(move |a| page_builds::on_phase(a, phase));
            if p.is_some() && with_self_missing().await {
                if let Ok(s) = lol::me().await {
                    on_ui(move |a| {
                        a.self_ = Some(s);
                        a.refresh_view();
                    });
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn with_self_missing() -> bool {
    let (tx, rx) = tokio::sync::oneshot::channel();
    on_ui(move |a| {
        let _ = tx.send(a.self_.is_none());
    });
    rx.await.unwrap_or(false)
}

pub fn offscreen() -> bool {
    std::env::var("KAIRO_OFFSCREEN").is_ok()
}

fn reduced_motion() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION};
    let mut on: i32 = 1;
    unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut i32 as *mut _, 0);
    }
    on == 0
}

fn install_img(ui: &AppWindow) {
    ui.global::<Api>().on_img(|url, _| images::img(&url));
    ui.global::<Act>().on_act(|k| act(&k));
}

fn main() {
    RT.set(tokio::runtime::Builder::new_multi_thread().worker_threads(3).enable_all().build().unwrap()).ok();
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attrs| {
            if OVERLAY_NEXT.load(Ordering::SeqCst) || offscreen() {
                use winit::platform::windows::WindowAttributesExtWindows;
                attrs.with_skip_taskbar(true).with_active(false).with_window_level(winit::window::WindowLevel::AlwaysOnTop).with_transparent(true).with_decorations(false)
            } else {
                attrs
            }
        })
        .select()
        .unwrap();
    let ui = AppWindow::new().unwrap();
    install_img(&ui);
    ui.global::<Motion>().set_reduced(reduced_motion() || std::env::var("KAIRO_REDUCED").is_ok());
    let game = store::get("rw-game").filter(|g| g == "valo" || g == "tft").unwrap_or_else(|| "lol".to_string());
    ui.global::<Theme>().set_game(ss(&game));
    ui.global::<Data>().set_game(ss(&game));
    let home = match game.as_str() {
        "valo" => "v-profil",
        "tft" => "t-profil",
        _ => "profil",
    };
    let first = store::get(&format!("rw-route-{game}")).map(|r| normalize_route(&r)).filter(|r| !r.is_empty()).unwrap_or_else(|| home.to_string());
    let script = std::env::var("KAIRO_SCRIPT").map(|s| s.split(';').map(String::from).collect()).unwrap_or_default();
    let app = App {
        ui: ui.clone_strong(),
        stack: vec![first],
        at: 0,
        game,
        phase: None,
        self_: None,
        d: None,
        vself: None,
        vph: None,
        vopen: false,
        key: String::new(),
        gen: 0,
        imported: 0,
        toast_timer: slint::Timer::default(),
        search: Default::default(),
        profile: Default::default(),
        match_: Default::default(),
        tier: Default::default(),
        champs: Default::default(),
        builds: Default::default(),
        live: Default::default(),
        settings: Default::default(),
        tft: Default::default(),
        picker: Default::default(),
        update: Default::default(),
        valo: Default::default(),
        script,
    };
    APP.with(|a| *a.borrow_mut() = Some(app));
    let dd_ui = ui.as_weak();
    spawn(async move {
        if let Ok(dd) = lol::ddragon().await {
            let _ = dd_ui;
            on_ui(move |a| {
                a.d = Some(dd);
                a.render_shell();
                page_misc::rerender_current(a);
            });
        }
    });
    with_app(|a| {
        page_misc::render_games(a);
        a.refresh_view();
    });
    poll_lcu();
    page_valo::poll();
    overlay::start();
    if !offscreen() {
        hotkey::start();
    }
    update::start();
    let rev = slint::Timer::default();
    rev.start(slint::TimerMode::Repeated, Duration::from_millis(80), || {
        if images::take_dirty() {
            with_app(|a| {
                let api = a.ui.global::<Api>();
                api.set_rev(api.get_rev() + 1);
                overlay::bump();
            });
        }
    });
    let hooked = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(ui) = hooked.upgrade() else { return };
        ui.window().on_winit_window_event(|_w, e| {
            use winit::event::{ElementState, MouseButton, WindowEvent};
            match e {
                WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                    with_app(|a| a.ui.global::<Motion>().set_keyboard(false));
                    if *button == MouseButton::Back {
                        slint::Timer::single_shot(Duration::ZERO, || {
                            with_app(|a| dispatch(a, "back"));
                        });
                    }
                    if *button == MouseButton::Forward {
                        slint::Timer::single_shot(Duration::ZERO, || {
                            with_app(|a| dispatch(a, "fwd"));
                        });
                    }
                }
                WindowEvent::Resized(_) => {
                    with_app(|a| {
                        let m = a.ui.window().is_maximized();
                        a.data().set_maximized(m);
                    });
                }
                _ => {}
            }
            slint::winit_030::EventResult::Propagate
        });
        ui.window().with_winit_window(|w: &winit::window::Window| {
            use winit::platform::windows::WindowExtWindows;
            w.set_undecorated_shadow(true);
        });
    });
    page_misc::start_script();
    if offscreen() {
        ui.window().set_position(slint::PhysicalPosition::new(-20000, -20000));
    }
    ui.show().unwrap();
    #[cfg(feature = "e2e")]
    e2e::start(&ui, "KAIRO", std::rc::Rc::new(|ui: &AppWindow, name: &str| {
        if let (Ok(dir), Ok(shot)) = (std::env::var("KAIRO_E2E_SNAP"), ui.window().take_snapshot()) {
            let _ = image::save_buffer(format!("{dir}/{name}.png"), shot.as_bytes(), shot.width(), shot.height(), image::ColorType::Rgba8);
        }
    }));
    let _tray = if offscreen() { None } else { tray::start() };
    slint::run_event_loop_until_quit().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_routes() {
        assert_eq!(normalize_route(r#"{"page":"profil"}"#), "profil");
        assert_eq!(normalize_route(r#"{"page":"profil","puuid":"x"}"#), "profil/x");
        assert_eq!(normalize_route(r#"{"page":"match","gameId":12,"puuid":"x"}"#), "match/12/x");
        assert_eq!(normalize_route(r#"{"page":"champion","champ":7,"role":"mid"}"#), "champion/7/mid");
        assert_eq!(normalize_route(r#"{"page":"v-match","id":"a","puuid":"b"}"#), "v-match/a/b");
        assert_eq!(normalize_route("tierlist"), "tierlist");
    }
}
