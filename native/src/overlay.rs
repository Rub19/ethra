use crate::jungle::route;
use crate::lol::{ddragon, img_ability, img_champ, img_item, DD};
use crate::net::live;
use crate::opgg::{champ_id, opgg_build};
use crate::store::settings;
use crate::text::{fr, mmss};
use crate::{ms, spawn, ss, OverlayWindow, OvAlert, OvRow, OvWidget};
use serde_json::Value;
use slint::winit_030::{winit, WinitWindowAccessor};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const DRAGON_FIRST: f64 = 300.0;
const DRAGON_RESPAWN: f64 = 300.0;
const BARON_FIRST: f64 = 1200.0;
const BARON_RESPAWN: f64 = 360.0;
const INHIB_RESPAWN: f64 = 300.0;
const ALERT: f64 = 30.0;
const KEYS: [&str; 4] = ["Q", "W", "E", "R"];
const CD: &str = "https://raw.communitydragon.org/latest/";

thread_local! {
    static WIN: RefCell<Option<OverlayWindow>> = const { RefCell::new(None) };
}

#[derive(Clone, Default)]
pub struct Skills {
    pub key: String,
    pub order: Vec<i64>,
    pub items: Vec<i64>,
}

pub fn icon_dragon() -> String {
    format!("{CD}game/assets/ux/minimap/icons/dragon.png")
}
pub fn icon_baron() -> String {
    format!("{CD}game/assets/ux/minimap/icons/baron.png")
}
pub fn icon_inhib() -> String {
    format!("{CD}plugins/rcp-fe-lol-match-history/global/default/inhibitor-100.png")
}

fn obj_icon(l: &str) -> String {
    if l.starts_with("Dragon") {
        icon_dragon()
    } else if l.starts_with("Nashor") {
        icon_baron()
    } else {
        icon_inhib()
    }
}

fn period(l: &str) -> f64 {
    if l.starts_with("Dragon") {
        DRAGON_RESPAWN
    } else if l.starts_with("Nashor") {
        BARON_RESPAWN
    } else {
        INHIB_RESPAWN
    }
}

pub fn row(icon: impl AsRef<str>, label: impl AsRef<str>, value: impl AsRef<str>, tone: i32, bar: f32) -> OvRow {
    OvRow { icon: ss(icon), label: ss(label), value: ss(value), tone, bar }
}

pub fn widget(title: impl AsRef<str>, rows: Vec<OvRow>) -> OvWidget {
    OvWidget { title: ss(title), rows: ms(rows) }
}

fn tone(x: f64) -> i32 {
    if x >= 0.0 {
        1
    } else {
        2
    }
}

fn sign(x: f64) -> String {
    format!("{}{x}", if x >= 0.0 { "+" } else { "" })
}

pub struct Frame {
    pub widgets: Vec<OvWidget>,
    pub alerts: Vec<OvAlert>,
    pub skill: Option<(String, String, String)>,
}

pub fn objectives(g: &Value, t: f64) -> (Vec<(String, f64)>, Vec<(String, f64)>) {
    let sr = g["gameData"]["mapNumber"].as_i64() == Some(11);
    let events: &[Value] = g["events"]["Events"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let last = |name: &str| events.iter().filter(|e| e["EventName"] == name).filter_map(|e| e["EventTime"].as_f64()).next_back();
    let mut timers = vec![];
    if sr {
        timers.push(("Dragon".to_string(), last("DragonKill").map(|x| x + DRAGON_RESPAWN).unwrap_or(DRAGON_FIRST)));
        if t >= BARON_FIRST - 120.0 {
            timers.push(("Nashor".to_string(), last("BaronKill").map(|x| x + BARON_RESPAWN).unwrap_or(BARON_FIRST)));
        }
    }
    let inhibs = events
        .iter()
        .filter(|e| e["EventName"] == "InhibKilled" && e["EventTime"].as_f64().unwrap_or(0.0) + INHIB_RESPAWN > t)
        .map(|e| {
            let name = e["InhibKilled"].as_str().unwrap_or_default();
            let lane = if name.contains("_L") {
                "top "
            } else if name.contains("_C") {
                "mid "
            } else if name.contains("_R") {
                "bot "
            } else {
                ""
            };
            (format!("Inhib {lane}{}", if name.contains("_T1_") { "bleu" } else { "rouge" }), e["EventTime"].as_f64().unwrap_or(0.0) + INHIB_RESPAWN)
        })
        .collect();
    (timers, inhibs)
}

pub fn wants_of(g: &Value) -> Option<(String, String, i64)> {
    let cfg = settings();
    if !(cfg.ov_skill || cfg.ov_item) {
        return None;
    }
    let players = g["allPlayers"].as_array()?;
    let me = players.iter().find(|p| !g["activePlayer"]["riotId"].is_null() && p["riotId"] == g["activePlayer"]["riotId"])?;
    let champ = me["championName"].as_str().unwrap_or_default().to_string();
    let map = g["gameData"]["mapNumber"].as_i64().unwrap_or(0);
    Some((format!("{champ}-{}-{map}", me["position"].as_str().unwrap_or_default()), champ, map))
}

pub fn build_frame(g: &Value, d: Option<&DD>, skills: Option<&Skills>) -> Option<Frame> {
    let cfg = settings();
    let players: &[Value] = g["allPlayers"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let me = players.iter().find(|p| !g["activePlayer"]["riotId"].is_null() && p["riotId"] == g["activePlayer"]["riotId"])?;
    if !cfg.ov_enabled {
        return None;
    }
    let t = g["gameData"]["gameTime"].as_f64().unwrap_or(0.0);
    let map = g["gameData"]["mapNumber"].as_i64().unwrap_or(0);
    let sr = map == 11;
    let s = &me["scores"];
    let champ_name = me["championName"].as_str().unwrap_or_default().to_string();
    let position = me["position"].as_str().unwrap_or_default().to_string();
    let (timers, inhibs) = objectives(g, t);
    let alerts: Vec<OvAlert> = if cfg.ov_alerts { timers.iter().filter(|x| x.1 > t && x.1 - t <= ALERT).map(|x| OvAlert { icon: ss(obj_icon(&x.0)), name: ss(&x.0), secs: ss(format!("{}s", (x.1 - t).ceil())) }).collect() } else { vec![] };
    let worth = |p: &Value| -> f64 {
        p["items"].as_array().into_iter().flatten().map(|it| d.and_then(|d| d.items.get(&it["itemID"].as_i64().unwrap_or(0))).map(|x| x.total).unwrap_or(0) as f64 * it["count"].as_f64().filter(|c| *c > 0.0).unwrap_or(1.0)).sum()
    };
    let my_team = me["team"].as_str().unwrap_or_default();
    let enemy_team = if my_team == "ORDER" { "CHAOS" } else { "ORDER" };
    let team_worth = |team: &str| players.iter().filter(|p| p["team"] == team).map(worth).sum::<f64>();
    let diff = team_worth(my_team) - team_worth(enemy_team);
    let mut dead: Vec<&Value> = players.iter().filter(|p| p["team"] == enemy_team && p["isDead"] == true && p["respawnTimer"].as_f64().unwrap_or(0.0) > 0.0).collect();
    dead.sort_by(|x, y| y["respawnTimer"].as_f64().unwrap_or(0.0).total_cmp(&x["respawnTimer"].as_f64().unwrap_or(0.0)));
    let owned: HashSet<i64> = me["items"].as_array().into_iter().flatten().filter_map(|i| i["itemID"].as_i64()).collect();
    let next_item = skills.and_then(|k| k.items.iter().find(|id| !owned.contains(id))).copied();
    let next_price = next_item.and_then(|id| d.and_then(|d| d.items.get(&id))).map(|x| x.total).unwrap_or(0);
    let gold = g["activePlayer"]["currentGold"].as_f64().unwrap_or(0.0).floor() as i64;
    let opp = if position.is_empty() { None } else { players.iter().find(|p| p["team"] == enemy_team && p["position"] == position.as_str()) };
    let spent: i64 = KEYS.iter().map(|k| g["activePlayer"]["abilities"][k]["abilityLevel"].as_i64().unwrap_or(0)).sum();
    let level = g["activePlayer"]["level"].as_i64().unwrap_or(0);
    let next_slot = skills.and_then(|k| k.order.get(spent as usize)).copied().unwrap_or(0);
    let next_key = if spent < level && next_slot > 0 { KEYS.get(next_slot as usize - 1).copied() } else { None };
    let champ = d.map(|d| champ_id(d, &champ_name)).unwrap_or(0);
    let skill = match (cfg.ov_skill, next_key, d) {
        (true, Some(k), Some(d)) => Some((k.to_string(), d.champs.get(&champ).map(|c| img_ability(&c.id, k)).unwrap_or_default(), format!("NIVEAU {level}"))),
        _ => None,
    };
    let mut w = vec![];
    if cfg.ov_jungle && sr && position == "JUNGLE" && t < 240.0 {
        let r = route(if my_team == "CHAOS" { "red" } else { "blue" }, if cfg.jungle_route == "blue" { "blue" } else { "red" });
        let last = r.steps.len() - 1;
        let rows = r
            .steps
            .iter()
            .enumerate()
            .map(|(i, x)| {
                if i == last {
                    row("", format!("C. {}", x.0), if t < 210.0 { mmss(210.0 - t) } else { "UP".to_string() }, if t >= 210.0 { 3 } else { 0 }, -1.0)
                } else {
                    row("", format!("{}. {}", i + 1, x.0), "", 0, -1.0)
                }
            })
            .collect();
        w.push(widget(format!("Jungle path, fin côté {}", r.side), rows));
    }
    let all: Vec<&(String, f64)> = timers.iter().chain(inhibs.iter()).collect();
    if cfg.ov_timers && !all.is_empty() {
        w.push(widget(
            "Objectifs",
            all.iter()
                .map(|x| {
                    if x.1 <= t {
                        row(obj_icon(&x.0), &x.0, "UP", 3, -1.0)
                    } else {
                        row(obj_icon(&x.0), &x.0, mmss(x.1 - t), 0, (1.0 - (x.1 - t) / period(&x.0)) as f32)
                    }
                })
                .collect(),
        ));
    }
    if let (true, Some(d), false) = (cfg.ov_deaths, d, dead.is_empty()) {
        w.push(widget(
            "Ennemis morts",
            dead.iter()
                .map(|p| {
                    let name = p["championName"].as_str().unwrap_or_default();
                    let id = champ_id(d, name);
                    row(img_champ(d, id), d.champs.get(&id).map(|c| c.name.as_str()).unwrap_or(name), format!("{}s", p["respawnTimer"].as_f64().unwrap_or(0.0).ceil()), 1, -1.0)
                })
                .collect(),
        ));
    }
    if let (true, Some(d), Some(id)) = (cfg.ov_item, d, next_item) {
        let name = d.items.get(&id).map(|x| x.name.clone()).unwrap_or_default();
        let ok = gold >= next_price;
        w.push(widget("Prochain objet", vec![row(img_item(d, id), name, if ok { "Achetable".to_string() } else { format!("-{}", fr(next_price - gold)) }, if ok { 3 } else { 0 }, if next_price > 0 && !ok { gold as f32 / next_price as f32 } else { -1.0 })]));
    }
    if let (true, Some(o)) = (cfg.ov_bench, opp) {
        let cs = s["creepScore"].as_f64().unwrap_or(0.0) - o["scores"]["creepScore"].as_f64().unwrap_or(0.0);
        let item_diff = worth(me) - worth(o);
        let lvl = me["level"].as_f64().unwrap_or(0.0) - o["level"].as_f64().unwrap_or(0.0);
        w.push(widget(format!("vs {}", o["championName"].as_str().unwrap_or_default()), vec![row("", "CS", sign(cs), tone(cs), -1.0), row("", "Objets", format!("{}k", sign((item_diff / 100.0).round() / 10.0)), tone(item_diff), -1.0), row("", "Niveau", sign(lvl), tone(lvl), -1.0)]));
    }
    if cfg.ov_gold && d.is_some() {
        w.push(widget("Valeur des objets", vec![row("", "Équipes", format!("{}{:.1}k", if diff >= 0.0 { "+" } else { "" }, diff / 1000.0), tone(diff), -1.0)]));
    }
    if cfg.ov_stats {
        let i = |k: &str| s[k].as_i64().unwrap_or(0);
        w.push(widget("Toi", vec![row("", "KDA", format!("{} / {} / {}", i("kills"), i("deaths"), i("assists")), 0, -1.0), row("", "CS/min", format!("{:.1}", s["creepScore"].as_f64().unwrap_or(0.0) / (t / 60.0).max(1.0)), 0, -1.0), row("", "Or", fr(gold), 0, -1.0)]));
    }
    Some(Frame { widgets: w, alerts, skill })
}

fn ensure_window() {
    WIN.with(|w| {
        if w.borrow().is_some() {
            return;
        }
        crate::OVERLAY_NEXT.store(true, Ordering::SeqCst);
        let Ok(win) = OverlayWindow::new() else { return };
        win.global::<crate::Api>().on_img(|u, _| crate::images::img(&u));
        let _ = win.show();
        crate::OVERLAY_NEXT.store(false, Ordering::SeqCst);
        let weak = win.as_weak();
        slint::Timer::single_shot(Duration::from_millis(30), move || {
            let Some(win) = weak.upgrade() else { return };
            win.window().with_winit_window(|w: &winit::window::Window| {
                if let Some(m) = w.primary_monitor() {
                    w.set_outer_position(m.position());
                    let _ = w.request_inner_size(m.size());
                }
                let _ = w.set_cursor_hittest(false);
            });
            make_click_through(&win);
        });
        *w.borrow_mut() = Some(win);
    });
}

fn make_click_through(win: &OverlayWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT};
    let _ = win;
    let title: Vec<u16> = "Kairo overlay\0".encode_utf16().collect();
    unsafe {
        let h = FindWindowW(std::ptr::null(), title.as_ptr());
        if !h.is_null() {
            let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
            SetWindowLongW(h, GWL_EXSTYLE, (ex | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW) as i32);
        }
    }
}

pub fn overlay_styles() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowLongW, GWL_EXSTYLE};
    let title: Vec<u16> = "Kairo overlay\0".encode_utf16().collect();
    unsafe {
        let h = FindWindowW(std::ptr::null(), title.as_ptr());
        if h.is_null() {
            None
        } else {
            Some(GetWindowLongW(h, GWL_EXSTYLE) as u32)
        }
    }
}

pub fn snapshot_overlay(path: &str) {
    WIN.with(|w| {
        if let Some(win) = w.borrow().as_ref() {
            if let Ok(s) = win.window().take_snapshot() {
                let _ = image::save_buffer(path, s.as_bytes(), s.width(), s.height(), image::ColorType::Rgba8);
            }
        }
    });
}

pub fn show_frame(f: Option<Frame>) {
    match f {
        None => {
            WIN.with(|w| {
                if let Some(win) = w.borrow().as_ref() {
                    win.set_visible_all(false);
                }
            });
        }
        Some(f) => {
            ensure_window();
            WIN.with(|w| {
                if let Some(win) = w.borrow().as_ref() {
                    win.set_visible_all(true);
                    win.set_widgets(ms(f.widgets));
                    win.set_alerts(ms(f.alerts));
                    win.set_skill_show(f.skill.is_some());
                    if let Some((k, i, l)) = f.skill {
                        win.set_skill_key(ss(k));
                        win.set_skill_icon(ss(i));
                        win.set_skill_level(ss(l));
                    }
                }
            });
        }
    }
}

pub fn close_window() {
    WIN.with(|w| {
        if let Some(win) = w.borrow_mut().take() {
            let _ = win.hide();
        }
    });
}

pub fn bump() {
    WIN.with(|w| {
        if let Some(win) = w.borrow().as_ref() {
            let api = win.global::<crate::Api>();
            api.set_rev(api.get_rev() + 1);
        }
    });
}

pub fn start() {
    spawn(async {
        let mut d: Option<Arc<DD>> = None;
        let mut skills: Option<Skills> = None;
        let mut idle = 0u32;
        loop {
            let g = if let Ok(path) = std::env::var("KAIRO_FAKE_LIVE") { std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()) } else { live("allgamedata").await.ok() };
            match g {
                Some(g) => {
                    idle = 0;
                    if d.is_none() {
                        d = ddragon().await.ok();
                    }
                    if let (Some(dd), Some((key, champ_name, map))) = (&d, wants_of(&g)) {
                        if skills.as_ref().map(|s| s.key.as_str()) != Some(key.as_str()) {
                            let champ = champ_id(dd, &champ_name);
                            let aram = map == 12;
                            let pos = g["allPlayers"].as_array().and_then(|l| l.iter().find(|p| p["championName"] == champ_name.as_str())).and_then(|p| p["position"].as_str()).unwrap_or_default().to_string();
                            let b = opgg_build(dd, champ, if aram { Some("mid") } else if pos.is_empty() { None } else { Some(&pos) }, if aram { "aram" } else { "ranked" }, &settings().rank_filter).await.ok().flatten();
                            skills = Some(match b {
                                Some(b) => {
                                    let mut items = b.order.clone();
                                    if let Some(x) = b.extra.as_ref().and_then(|x| x.boots.first()) {
                                        items.push(*x);
                                    }
                                    items.extend(b.core.iter());
                                    Skills { key: key.clone(), order: b.skills.clone(), items }
                                }
                                None => Skills { key, ..Default::default() },
                            });
                        }
                    }
                    let (dd, sk) = (d.clone(), skills.clone());
                    let _ = slint::invoke_from_event_loop(move || show_frame(build_frame(&g, dd.as_deref(), sk.as_ref())));
                }
                None => {
                    idle += 1;
                    if idle == 2 {
                        let _ = slint::invoke_from_event_loop(close_window);
                    }
                    skills = None;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(t: f64, events: Value) -> Value {
        json!({"gameData": {"gameTime": t, "mapNumber": 11}, "events": {"Events": events}, "activePlayer": {"riotId": "me#1", "level": 4, "currentGold": 500.4, "abilities": {"Q": {"abilityLevel": 1}, "W": {"abilityLevel": 1}, "E": {"abilityLevel": 1}, "R": {"abilityLevel": 0}}},
            "allPlayers": [
                {"riotId": "me#1", "team": "ORDER", "championName": "Ahri", "position": "MIDDLE", "level": 4, "items": [{"itemID": 1, "count": 1}], "scores": {"kills": 1, "deaths": 2, "assists": 3, "creepScore": 40}},
                {"riotId": "x#1", "team": "CHAOS", "championName": "Yasuo", "position": "MIDDLE", "level": 3, "isDead": true, "respawnTimer": 12.2, "items": [], "scores": {"creepScore": 30}}
            ]})
    }

    fn dd() -> DD {
        let mut d = DD::default();
        d.items.insert(1, crate::lol::Item { name: "A".into(), total: 400, ..Default::default() });
        d.items.insert(2, crate::lol::Item { name: "Rabadon".into(), total: 3600, ..Default::default() });
        d.champs.insert(103, crate::lol::Champ { id: "Ahri".into(), name: "Ahri".into(), ..Default::default() });
        d
    }

    #[test]
    fn timers() {
        let (t, _) = objectives(&game(100.0, json!([])), 100.0);
        assert_eq!(t, vec![("Dragon".to_string(), 300.0)]);
        let (t, _) = objectives(&game(1100.0, json!([{"EventName": "DragonKill", "EventTime": 900.0}])), 1100.0);
        assert_eq!(t, vec![("Dragon".to_string(), 1200.0), ("Nashor".to_string(), 1200.0)]);
        let (_, i) = objectives(&game(100.0, json!([{"EventName": "InhibKilled", "EventTime": 50.0, "InhibKilled": "Barracks_T1_C1"}])), 100.0);
        assert_eq!(i, vec![("Inhib mid bleu".to_string(), 350.0)]);
    }

    #[test]
    fn frame() {
        let d = dd();
        let s = Skills { key: "k".into(), order: vec![1, 2, 3, 1], items: vec![1, 2] };
        let f = build_frame(&game(280.0, json!([])), Some(&d), Some(&s)).unwrap();
        assert_eq!(f.alerts.len(), 1);
        assert_eq!(f.skill.as_ref().unwrap().0, "Q");
        let titles: Vec<String> = f.widgets.iter().map(|w| w.title.to_string()).collect();
        assert!(titles.iter().any(|t| t == "vs Yasuo"));
        assert!(titles.iter().any(|t| t == "Ennemis morts"));
        assert!(titles.iter().any(|t| t == "Prochain objet"));
        assert!(build_frame(&json!({"allPlayers": []}), Some(&d), None).is_none());
    }
}
