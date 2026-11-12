use crate::net::valo;
use crate::opgg;
use crate::page_profile::{col, copy_text, iconed, row, sort_col, toned, txt};
use crate::store::settings;
use crate::text::*;
use crate::valo_api::*;
use crate::{good, ms, on_ui, opt, spawn, ss, stat, App};
use crate::{VParty, Opt, Perf, PlayerChip, RankRow, VAgentsData, VKillV, VLiveData, VLiveRow, VLiveTeam, VMatchData, VProfileData, VRoundBtn, VRowV, VWeapon};
use serde_json::Value;
use slint::{Color, Model, ModelRc, VecModel};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

const PAGE: usize = 20;
const FILTERS: [(&str, &str); 6] = [("", "Toutes"), ("competitive", "Compétition"), ("unrated", "Non classé"), ("swiftplay", "Vélocité"), ("deathmatch", "Combat à mort"), ("premier", "Premier")];

#[derive(Default)]
pub struct ValoState {
    pub d: Option<Arc<VD>>,
    pub names: HashMap<String, VName>,
    pub asked: HashSet<String>,
    pub who: String,
    pub name: Option<VName>,
    pub rank: Option<VRank>,
    pub rr: HashMap<String, VRR>,
    pub queue: String,
    pub ids: Option<Vec<String>>,
    pub more: String,
    pub by_id: HashMap<String, Arc<VMatch>>,
    pub failed: HashSet<String>,
    pub tab: String,
    pub err: String,
    pub rows: Option<Rc<VecModel<VRowV>>>,
    pub m: Option<Arc<VMatch>>,
    pub m_puuid: String,
    pub round: Option<usize>,
    pub agents: Option<Vec<AgentRow>>,
    pub patch: String,
    pub role: String,
    pub sort: String,
    pub desc: bool,
    pub live: Option<LiveGame>,
    pub ranks: HashMap<String, VRank>,
    pub forms: HashMap<String, Vec<i64>>,
    pub parties: HashMap<String, String>,
    pub tick: u32,
}

#[derive(Clone, Default)]
pub struct LiveGame {
    pub map: String,
    pub queue: String,
    pub players: Vec<LiveP>,
}

#[derive(Clone, Default)]
pub struct LiveP {
    pub puuid: String,
    pub agent: String,
    pub team: String,
    pub incognito: bool,
    pub level: i64,
    pub locked: Option<bool>,
}

#[derive(Clone, Default)]
pub struct AgentRow {
    pub id: String,
    pub games: i64,
    pub wr: f64,
    pub pick: f64,
    pub kd: f64,
    pub acs: f64,
    pub hs: f64,
    pub fkfd: f64,
    pub tier: i64,
}

fn vdata_ready(a: &mut App, then: fn(&mut App)) {
    let gen = a.gen;
    spawn(async move {
        match vdata().await {
            Ok(d) => on_ui(move |a| {
                a.valo.d = Some(d);
                if a.gen == gen {
                    then(a);
                }
            }),
            Err(e) => on_ui(move |a| {
                if a.gen == gen {
                    a.valo.err = e;
                    then(a);
                }
            }),
        }
    });
}

pub fn poll() {
    spawn(async {
        loop {
            let game = {
                let (tx, rx) = tokio::sync::oneshot::channel();
                on_ui(move |a| {
                    let _ = tx.send(a.game.clone());
                });
                rx.await.unwrap_or_default()
            };
            if game == "valo" {
                let s = valo("session", "GET", "", None).await.ok();
                let puuid = s.and_then(|v| v["puuid"].as_str().map(String::from)).filter(|x| !x.is_empty());
                let presence = match &puuid {
                    Some(p) => Some(v_presence(p).await.unwrap_or((false, "MENUS".to_string()))),
                    None => None,
                };
                on_ui(move |a| {
                    let old = a.valo_phase();
                    a.vself = puuid;
                    a.vopen = presence.as_ref().is_some_and(|x| x.0);
                    a.vph = presence.map(|x| x.1);
                    if a.vph.as_deref() == Some("PREGAME") && old.as_deref() != Some("PREGAME") && settings().auto_open_live {
                        a.go("v-partie");
                    }
                    let changed = old != a.vph;
                    a.refresh_view();
                    if changed && a.page() == "v-partie" {
                        load_live(a);
                    }
                });
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
}

impl App {
    fn valo_phase(&self) -> Option<String> {
        self.vph.clone()
    }
}

fn name_of(a: &App, subject: &str, p_name: &str, p_tag: &str) -> (String, String) {
    let n = a.valo.names.get(subject);
    (if p_name.is_empty() { n.map(|x| x.game_name.clone()).unwrap_or_default() } else { p_name.to_string() }, if p_tag.is_empty() { n.map(|x| x.tag_line.clone()).unwrap_or_default() } else { p_tag.to_string() })
}

fn ask_names(a: &mut App, list: Vec<String>) {
    let wanted: Vec<String> = list.into_iter().filter(|p| !a.valo.names.contains_key(p) && a.valo.asked.insert(p.clone())).collect();
    if wanted.is_empty() {
        return;
    }
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(60)).await;
        if let Ok(names) = v_names(&wanted).await {
            on_ui(move |a| {
                for n in names {
                    a.valo.names.insert(n.subject.clone(), n);
                }
                match a.page().as_str() {
                    "v-profil" => rebuild_rows(a),
                    "v-match" => render_match(a),
                    "v-partie" => render_live(a),
                    _ => {}
                }
            });
        } else {
            on_ui(move |a| {
                for p in &wanted {
                    a.valo.asked.remove(p);
                }
            });
        }
    });
}

fn pct(x: f64) -> String {
    pct0(x)
}

fn agent_icon(d: &VD, id: &str) -> String {
    d.agents.get(&id.to_lowercase()).map(|x| x.icon.clone()).unwrap_or_default()
}

pub fn open_profile(a: &mut App, puuid: Option<String>) {
    let who = puuid.or(a.vself.clone()).unwrap_or_default();
    let st = &mut a.valo;
    st.who = who.clone();
    st.name = None;
    st.rank = None;
    st.rr.clear();
    st.queue.clear();
    st.ids = None;
    st.more = "idle".into();
    st.by_id.clear();
    st.failed.clear();
    st.tab = "parties".into();
    st.err.clear();
    a.data().set_vprofile(VProfileData { loading: true, ids_loading: true, ..Default::default() });
    vdata_ready(a, |a| render_profile(a, true));
    let gen = a.gen;
    let w = who.clone();
    spawn(async move {
        let names = v_names(std::slice::from_ref(&w)).await.ok();
        let (rank, rr) = tokio::join!(v_rank(&w), v_rr_updates(&w));
        on_ui(move |a| {
            if a.gen == gen {
                a.valo.name = names.and_then(|mut n| if n.is_empty() { None } else { Some(n.remove(0)) });
                a.valo.rank = Some(rank);
                a.valo.rr = rr.into_iter().collect();
                render_profile(a, false);
            }
        });
    });
    load_ids(a);
}

fn load_ids(a: &mut App) {
    a.valo.ids = None;
    a.valo.more = "idle".into();
    let (who, queue, gen) = (a.valo.who.clone(), a.valo.queue.clone(), a.gen);
    spawn(async move {
        let r = v_history(&who, PAGE, &queue, 0).await;
        on_ui(move |a| {
            if a.gen != gen || a.valo.queue != queue {
                return;
            }
            match r {
                Ok(list) => {
                    if list.len() < PAGE {
                        a.valo.more = "end".into();
                    }
                    a.valo.ids = Some(list.clone());
                    render_profile(a, true);
                    track(a, list);
                }
                Err(e) => {
                    a.valo.err = e;
                    render_profile(a, false);
                }
            }
        });
    });
}

fn track(a: &App, list: Vec<String>) {
    let gen = a.gen;
    for id in list {
        spawn(async move {
            let r = v_match(&id).await;
            on_ui(move |a| {
                if a.gen != gen {
                    return;
                }
                match r {
                    Ok(m) => {
                        a.valo.failed.remove(&id);
                        a.valo.by_id.insert(id.clone(), m.clone());
                        let names: Vec<String> = m.players.iter().filter(|p| p.game_name.is_empty()).map(|p| p.subject.clone()).collect();
                        ask_names(a, names);
                    }
                    Err(_) => {
                        a.valo.failed.insert(id.clone());
                    }
                }
                update_row(a, &id);
                render_profile(a, false);
            });
        });
    }
}

fn update_row(a: &App, id: &str) {
    let Some(d) = a.valo.d.clone() else { return };
    let Some(pos) = a.valo.ids.as_ref().and_then(|l| l.iter().position(|x| x == id)) else { return };
    if let Some(m) = &a.valo.rows {
        if pos < m.row_count() {
            m.set_row_data(pos, v_row(a, id, &d));
        }
    }
}

fn rebuild_rows(a: &mut App) {
    let Some(d) = a.valo.d.clone() else { return };
    let ids = a.valo.ids.clone().unwrap_or_default();
    match &a.valo.rows {
        Some(m) if m.row_count() == ids.len() => {
            for (i, id) in ids.iter().enumerate() {
                m.set_row_data(i, v_row(a, id, &d));
            }
        }
        _ => {
            let model = Rc::new(VecModel::from(ids.iter().map(|id| v_row(a, id, &d)).collect::<Vec<_>>()));
            a.valo.rows = Some(model);
        }
    }
}

fn v_row(a: &App, id: &str, d: &VD) -> VRowV {
    let st = &a.valo;
    let who = &st.who;
    let mut out = VRowV { id: ss(id), state: 1, ..Default::default() };
    if st.failed.contains(id) {
        out.state = 2;
        return out;
    }
    let Some(m) = st.by_id.get(id) else { return out };
    let Some(p) = m.players.iter().find(|x| &x.subject == who) else { return VRowV { id: ss(id), state: 3, ..Default::default() } };
    let Some(pst) = &p.stats else { return VRowV { id: ss(id), state: 3, ..Default::default() } };
    let stats = v_stats(m);
    let s = &stats[who];
    let dm = m.match_info.queue_id == "deathmatch";
    let kills_of = |x: &VPlayer| x.stats.as_ref().map(|s| s.kills).unwrap_or(0);
    let won = if dm { m.players.iter().max_by_key(|x| kills_of(x)).is_some_and(|x| &x.subject == who) } else { v_team_won(m, &p.team_id) };
    let draw = !dm && m.teams.as_ref().is_some_and(|t| t.iter().all(|x| !x.won));
    let mut ranked: Vec<&VPlayer> = m.players.iter().collect();
    ranked.sort_by(|x, y| stats[&y.subject].acs.total_cmp(&stats[&x.subject].acs));
    let place = ranked.iter().position(|x| &x.subject == who).unwrap_or(0) + 1;
    let team_best = ranked.iter().find(|x| x.team_id == p.team_id).is_some_and(|x| &x.subject == who);
    let map = d.maps.get(&m.match_info.map_id);
    let rr = st.rr.get(id);
    let chip = |x: &VPlayer| {
        let (name, _) = name_of(a, &x.subject, &x.game_name, &x.tag_line);
        PlayerChip { icon: ss(agent_icon(d, &x.character_id)), name: ss(if name.is_empty() { "…".to_string() } else { name }), me: &x.subject == who, route: ss(format!("v-profil/{}", x.subject)) }
    };
    let team1: Vec<PlayerChip> = m.players.iter().filter(|x| x.team_id == p.team_id).take(5).map(chip).collect();
    let team2: Vec<PlayerChip> = m.players.iter().filter(|x| x.team_id != p.team_id).take(5).map(chip).collect();
    let score = v_score(m, &p.team_id);
    let tier = d.tiers.get(&p.competitive_tier).filter(|_| p.competitive_tier > 2);
    VRowV {
        id: ss(id),
        route: ss(format!("v-match/{id}/{who}")),
        state: 0,
        result: if draw { 2 } else if won { 0 } else { 1 },
        title: ss(if draw { "Égalité" } else if won { "Victoire" } else { "Défaite" }),
        queue: ss(queue_label(&m.match_info.queue_id)),
        when: ss(ago(m.match_info.game_start_millis)),
        duration: ss(mmss(m.match_info.game_length_millis.unwrap_or(0) as f64 / 1000.0)),
        face: ss(agent_icon(d, &p.character_id)),
        tier: ss(tier.map(|t| t.icon.clone()).unwrap_or_default()),
        mvp: ss(if place == 1 { "MVP" } else if team_best { "MVP équipe" } else { "" }),
        multi: ss(mk_label(&s.mk).first().cloned().unwrap_or_default()),
        rr: ss(rr.filter(|r| !r.placement).map(|r| format!("{}{} RR", if r.earned > 0 { "+" } else { "" }, r.earned)).unwrap_or_default()),
        rr_up: rr.is_some_and(|r| r.earned >= 0),
        promo: ss(match rr {
            Some(r) if r.tier_after > r.tier_before => format!("Promu {}", d.tiers.get(&r.tier_after).map(|t| t.name.clone()).unwrap_or_default()),
            Some(r) if r.tier_after < r.tier_before => "Rétrogradé".to_string(),
            _ => String::new(),
        }),
        promo_up: rr.is_some_and(|r| r.tier_after > r.tier_before),
        map: ss(map.map(|x| x.1.clone()).unwrap_or_default()),
        map_name: ss(map.map(|x| x.0.clone()).unwrap_or_else(|| "Carte".into())),
        score: ss(if score.is_empty() { format!("{place}{}", if place == 1 { "er" } else { "e" }) } else { score }),
        kda_k: ss(pst.kills.to_string()),
        kda_d: ss(pst.deaths.to_string()),
        kda_a: ss(pst.assists.to_string()),
        kd: ss(format!("{:.2}", s.kd)),
        kd_good: s.kd >= 1.0,
        kpr: ss(format!("{:.2}", pst.kills as f64 / pst.rounds_played.max(1) as f64)),
        adr: ss((s.adr.round() as i64).to_string()),
        acs: ss((s.acs.round() as i64).to_string()),
        hs: ss(pct(s.hs)),
        dm,
        team1: ms(team1),
        team2: ms(team2),
    }
}

#[derive(Clone, Copy, Default)]
struct Sum {
    g: f64,
    w: f64,
    k: f64,
    d: f64,
    acs: f64,
    hs: f64,
    adr: f64,
    r: f64,
    rw: f64,
}

fn tally(list: &[(&Arc<VMatch>, &VPlayer)], who: &str) -> Sum {
    let mut a = Sum::default();
    for (m, p) in list {
        let st = v_stats(m);
        let s = &st[who];
        let ps = p.stats.as_ref().unwrap();
        a.g += 1.0;
        a.w += if v_team_won(m, &p.team_id) { 1.0 } else { 0.0 };
        a.k += ps.kills as f64;
        a.d += ps.deaths as f64;
        a.acs += s.acs;
        a.hs += s.hs;
        a.adr += s.adr;
        a.r += ps.rounds_played as f64;
        a.rw += m.teams.iter().flatten().find(|t| t.team_id == p.team_id).map(|t| t.rounds_won).unwrap_or(0) as f64;
    }
    a
}

fn group<'a>(mine: &[(&'a Arc<VMatch>, &'a VPlayer)], who: &str, key: impl Fn(&VMatch, &VPlayer) -> String) -> Vec<(String, Sum)> {
    let mut g: Vec<(String, Vec<(&Arc<VMatch>, &VPlayer)>)> = vec![];
    for x in mine {
        let k = key(x.0, x.1);
        match g.iter_mut().find(|e| e.0 == k) {
            Some(e) => e.1.push(*x),
            None => g.push((k, vec![*x])),
        }
    }
    let mut out: Vec<(String, Sum)> = g.into_iter().map(|(k, l)| (k, tally(&l, who))).collect();
    out.sort_by(|x, y| y.1.g.total_cmp(&x.1.g));
    out
}

fn tier_rank(d: &VD, tier: i64, rr: Option<i64>, label: &str, sub: String) -> RankRow {
    let t = d.tiers.get(&tier).filter(|_| tier > 2);
    RankRow {
        emblem: ss(t.map(|x| x.icon.clone()).unwrap_or_default()),
        has: t.is_some(),
        label: ss(label),
        value: ss(match t {
            Some(x) => format!("{}{}", x.name, rr.map(|r| format!(", {r} RR")).unwrap_or_default()),
            None => "Non classé".to_string(),
        }),
        color: t.map(|x| hex_color(&x.color)).unwrap_or(Color::from_rgb_u8(0xee, 0xf0, 0xf7)),
        sub: ss(sub),
    }
}

pub fn matches_csv(lines: &[Vec<String>]) -> String {
    let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let head = ["date", "carte", "agent", "mode", "résultat", "score", "éliminations", "morts", "assistances", "ACS", "HS"];
    let mut out = vec![head.iter().map(|h| esc(h)).collect::<Vec<_>>().join(";")];
    out.extend(lines.iter().map(|l| l.iter().map(|c| esc(c)).collect::<Vec<_>>().join(";")));
    format!("\u{feff}{}", out.join("\n"))
}

pub fn export_csv(a: &mut App) {
    let (Some(d), Some(me)) = (a.valo.d.clone(), a.vself.clone()) else {
        a.toast("Ouvre d'abord ton profil VALORANT");
        return;
    };
    let mut list: Vec<Arc<VMatch>> = a.valo.by_id.values().cloned().collect();
    list.sort_by_key(|m| std::cmp::Reverse(m.match_info.game_start_millis));
    let lines: Vec<Vec<String>> = list
        .iter()
        .filter_map(|m| {
            let p = m.players.iter().find(|x| x.subject == me)?;
            let ps = p.stats.as_ref()?;
            let stats = v_stats(m);
            let s = stats.get(&me)?;
            Some(vec![
                crate::text::date_long_time(m.match_info.game_start_millis),
                d.maps.get(&m.match_info.map_id).map(|y| y.0.clone()).unwrap_or_else(|| m.match_info.map_id.clone()),
                d.agents.get(&p.character_id.to_lowercase()).map(|y| y.name.clone()).unwrap_or_default(),
                queue_label(&m.match_info.queue_id),
                if v_team_won(m, &p.team_id) { "Victoire".into() } else { "Défaite".into() },
                v_score(m, &p.team_id),
                ps.kills.to_string(),
                ps.deaths.to_string(),
                ps.assists.to_string(),
                format!("{}", s.acs.round()),
                pct(s.hs),
            ])
        })
        .collect();
    if lines.is_empty() {
        a.toast("Aucune partie VALORANT chargée pour l'instant");
        return;
    }
    let Some(path) = crate::page_misc::csv_path("kairo-valorant.csv") else { return };
    match std::fs::write(&path, matches_csv(&lines)) {
        Ok(()) => a.toast(format!("{} parties VALORANT exportées", lines.len())),
        Err(e) => a.toast(format!("Export impossible : {e}")),
    }
}

pub fn render_profile(a: &mut App, rows_too: bool) {
    if !a.valo.err.is_empty() {
        a.data().set_vprofile(VProfileData { error: ss(crate::text::friendly_error(&a.valo.err)), ..Default::default() });
        return;
    }
    let Some(d) = a.valo.d.clone() else { return };
    if rows_too {
        a.valo.rows = None;
        rebuild_rows(a);
    }
    let st = &a.valo;
    let who = st.who.clone();
    let ids = st.ids.clone().unwrap_or_default();
    let mine: Vec<(&Arc<VMatch>, &VPlayer)> = ids.iter().filter_map(|id| st.by_id.get(id)).filter_map(|m| m.players.iter().find(|x| x.subject == who).filter(|p| p.stats.is_some()).map(|p| (m, p))).collect();
    let agg = tally(&mine, &who);
    let n = agg.g.max(1.0);
    let agents = group(&mine, &who, |_, p| p.character_id.to_lowercase());
    let maps = group(&mine, &who, |m, _| m.match_info.map_id.clone());
    let role_of = |p: &VPlayer| d.agents.get(&p.character_id.to_lowercase()).map(|y| y.role.clone()).filter(|r| !r.is_empty()).unwrap_or_else(|| "?".into());
    let roles = group(&mine, &who, |_, p| role_of(p));
    let fav = agents.first().and_then(|x| d.agents.get(&x.0));
    let kd = |x: &Sum| format!("{:.2}", x.k / x.d.max(1.0));
    let mut quick = vec![];
    if !mine.is_empty() {
        quick.push(stat(pct(agg.w / n), "Winrate", good(agg.w / n >= 0.5)));
        quick.push(stat(kd(&agg), "K/D", 0));
        quick.push(stat(((agg.acs / n).round() as i64).to_string(), "ACS moyen", 0));
        quick.push(stat(((agg.adr / n).round() as i64).to_string(), "ADR moyen", 0));
        quick.push(stat(pct(agg.hs / n), "Headshots", 0));
        if let Some(f) = fav {
            quick.push(stat(&f.name, "Agent favori", 0));
        }
    }
    let midnight = midnight_ms();
    let today: Vec<(&Arc<VMatch>, &VPlayer)> = mine.iter().filter(|x| x.0.match_info.game_start_millis >= midnight).copied().collect();
    let ta = tally(&today, &who);
    let today_rr: i64 = today.iter().filter_map(|x| st.rr.get(&x.0.match_info.match_id)).map(|r| r.earned).sum();
    let has_rr = today.iter().any(|x| st.rr.contains_key(&x.0.match_info.match_id));
    let mut session = vec![stat(format!("{}V {}D", ta.w, ta.g - ta.w), format!("{} de victoires", pct(ta.w / ta.g.max(1.0))), 0), stat(kd(&ta), "K/D", 0)];
    if has_rr {
        session.push(stat(format!("{} RR", signed(today_rr)), "Compétition", good(today_rr >= 0)));
    }
    let rank = st.rank;
    let top = rank.and_then(|r| d.tiers.get(&r.tier).filter(|_| r.tier > 2).map(|t| (t.clone(), r.rr)));
    let ranks = match rank {
        Some(r) => vec![
            tier_rank(&d, r.tier, if r.tier > 2 { Some(r.rr) } else { None }, "Compétition, acte en cours", if r.games > 0 { format!("{}V sur {} parties ({})", r.wins, r.games, pct(r.wins as f64 / r.games as f64)) } else { String::new() }),
            tier_rank(&d, r.peak, None, "Meilleur rang", String::new()),
        ],
        None => vec![],
    };
    let perfs = agents
        .iter()
        .take(5)
        .map(|(k, x)| Perf {
            icon: ss(d.agents.get(k).map(|y| y.icon.clone()).unwrap_or_default()),
            title: ss(d.agents.get(k).map(|y| y.name.clone()).unwrap_or_else(|| "?".into())),
            sub: ss(format!("{} K/D, {} ACS", kd(x), (x.acs / x.g).round())),
            right: ss(pct(x.w / x.g)),
            right_sub: ss(format!("{} {}", x.g, if x.g > 1.0 { "parties" } else { "partie" })),
            tone: good(x.w / x.g >= 0.5),
            route: ss(""),
        })
        .collect();
    let tab = st.tab.clone();
    let source = match tab.as_str() {
        "agents" => &agents,
        "roles" => &roles,
        _ => &maps,
    };
    let mut rr: HashMap<String, i64> = HashMap::new();
    for (m, p) in &mine {
        if let Some(r) = st.rr.get(&m.match_info.match_id) {
            let key = match tab.as_str() {
                "agents" => p.character_id.to_lowercase(),
                "roles" => role_of(p),
                _ => m.match_info.map_id.clone(),
            };
            *rr.entry(key).or_default() += r.earned;
        }
    }
    let table = source
        .iter()
        .map(|(k, x)| {
            let c0 = if tab == "roles" {
                iconed(k.clone(), d.agents.values().find(|y| y.role == *k).map(|y| y.role_icon.clone()).unwrap_or_default())
            } else if tab == "agents" {
                iconed(d.agents.get(k).map(|y| y.name.clone()).unwrap_or_else(|| "?".into()), d.agents.get(k).map(|y| y.icon.clone()).unwrap_or_default())
            } else {
                iconed(d.maps.get(k).map(|y| y.0.clone()).unwrap_or_else(|| "?".into()), d.maps.get(k).map(|y| y.1.clone()).unwrap_or_default())
            };
            row(vec![c0, txt(x.g.to_string()), toned(pct(x.w / x.g), good(x.w / x.g >= 0.5)), txt(kd(x)), txt(((x.acs / x.g).round() as i64).to_string()), txt(((x.adr / x.g).round() as i64).to_string()), txt(pct(x.hs / x.g)), txt(if x.r > 0.0 { pct(x.rw / x.r) } else { "–".to_string() }), rr.get(k).map_or_else(|| txt("–".to_string()), |v| toned(signed(*v), good(*v >= 0)))], "")
        })
        .collect();
    let mut kills: HashMap<String, i64> = HashMap::new();
    for (m, p) in &mine {
        for (w, c) in v_weapon_kills(m, &who) {
            let key = if w == "ability" || w == "melee" || d.weapons.contains_key(&w) { w } else { format!("agent:{}", p.character_id.to_lowercase()) };
            *kills.entry(key).or_default() += c;
        }
    }
    let total: i64 = kills.values().sum::<i64>().max(1);
    let mut wrows: Vec<(String, i64)> = kills.into_iter().collect();
    wrows.sort_by_key(|x| std::cmp::Reverse(x.1));
    let topk = wrows.first().map(|x| x.1).unwrap_or(1).max(1);
    let weapons = wrows
        .iter()
        .map(|(w, c)| {
            let agent = w.strip_prefix("agent:");
            VWeapon {
                icon: ss(agent.and_then(|x| d.agents.get(x)).map(|y| y.icon.clone()).or(d.weapons.get(w).map(|y| y.1.clone())).unwrap_or_default()),
                agent: agent.is_some(),
                label: ss(match w.as_str() {
                    "ability" => "Compétences".to_string(),
                    "melee" => "Couteau".to_string(),
                    _ => match agent {
                        Some(x) => format!("Arme de {}", d.agents.get(x).map(|y| y.name.clone()).unwrap_or_else(|| "l'agent".into())),
                        None => d.weapons.get(w).map(|y| y.0.clone()).unwrap_or_else(|| "Arme".into()),
                    },
                }),
                kills: ss(format!("{c} {}", if *c > 1 { "éliminations" } else { "élimination" })),
                part: ss(pct(*c as f64 / total as f64)),
                value: *c as f32 / topk as f32,
            }
        })
        .collect::<Vec<_>>();
    let tabs: Vec<Opt> = [("parties", "Parties"), ("agents", "Agents"), ("roles", "Rôles"), ("cartes", "Cartes"), ("armes", "Armes")].iter().map(|(k, l)| opt(format!("vtab:{k}"), l, tab == *k)).collect();
    let more = st.more.as_str();
    let name = st.name.clone();
    let data = VProfileData {
        loading: false,
        error: ss(""),
        portrait: ss(fav.map(|f| f.portrait.clone()).unwrap_or_default()),
        name: ss(name.as_ref().map(|n| n.game_name.clone()).unwrap_or_else(|| "…".into())),
        tag: ss(name.as_ref().map(|n| n.tag_line.clone()).unwrap_or_default()),
        can_copy: name.is_some(),
        quick: ms(quick),
        rank_icon: ss(top.as_ref().map(|t| t.0.icon.clone()).unwrap_or_default()),
        rank_name: ss(top.as_ref().map(|t| t.0.name.clone()).unwrap_or_default()),
        rank_color: top.as_ref().map(|t| hex_color(&t.0.color)).unwrap_or_default(),
        rank_rr: ss(top.as_ref().map(|t| format!("{} RR", t.1)).unwrap_or_default()),
        tabs: ms(tabs),
        tab: ss(&tab),
        filters: ms(FILTERS.iter().map(|(k, l)| opt(format!("vq:{k}"), l, st.queue == *k)).collect()),
        me: a.vself.as_deref() == Some(who.as_str()),
        session_text: ss(if today.is_empty() { format!("Aucune partie aujourd'hui{}.", if st.queue.is_empty() { "" } else { " dans ce mode" }) } else { String::new() }),
        session: ms(session),
        session_strip: ms(today.iter().take(14).rev().map(|x| v_team_won(x.0, &x.1.team_id)).collect()),
        ranks: ms(ranks),
        title: ss(format!("{} parties analysées", agg.g)),
        wr: (agg.w / n) as f32,
        wr_text: ss(format!("{}V, {}D, {} de manches gagnées", agg.w, agg.g - agg.w, if agg.r > 0.0 { pct(agg.rw / agg.r) } else { "0%".to_string() })),
        perfs: ms(perfs),
        ids_loading: st.ids.is_none(),
        empty: ss(if st.ids.as_ref().is_some_and(|l| l.is_empty()) { format!("Aucune partie récente{}.", if st.queue.is_empty() { "" } else { " dans ce mode" }) } else { String::new() }),
        rows: st.rows.clone().map(ModelRc::from).unwrap_or_default(),
        more: ss(match more {
            "end" => String::new(),
            "loading" => "Chargement…".to_string(),
            _ => format!("Charger {PAGE} parties de plus"),
        }),
        more_enabled: more == "idle",
        weapons_empty: weapons.is_empty(),
        weapons: ms(weapons),
        cols: ms(vec![col(match tab.as_str() { "agents" => "Agent", "roles" => "Rôle", _ => "Carte" }, 0.0), col("Parties", 80.0), col("Winrate", 80.0), col("K/D", 70.0), col("ACS", 70.0), col("ADR", 70.0), col("HS", 70.0), col("Manches", 90.0), col("RR", 70.0)]),
        table: ms(table),
    };
    a.data().set_vprofile(data);
}

pub fn open_match(a: &mut App, id: String, puuid: String) {
    a.valo.m = None;
    a.valo.m_puuid = puuid;
    a.valo.round = None;
    a.valo.err.clear();
    a.data().set_vmatch(VMatchData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let (m, d) = tokio::join!(v_match(&id), vdata());
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match (m, d) {
                (Ok(m), Ok(d)) => {
                    a.valo.m = Some(m.clone());
                    a.valo.d = Some(d);
                    let names: Vec<String> = m.players.iter().filter(|p| p.game_name.is_empty()).map(|p| p.subject.clone()).collect();
                    ask_names(a, names);
                    render_match(a);
                }
                (Err(e), _) | (_, Err(e)) => a.data().set_vmatch(VMatchData { error: ss(crate::text::friendly_error(&e)), ..Default::default() }),
            }
        });
    });
}

fn result_fr(r: &str) -> String {
    match r {
        "Eliminated" => "Élimination",
        "Bomb detonated" => "Spike explosé",
        "Bomb defused" => "Spike désamorcé",
        "Round timer expired" => "Temps écoulé",
        "Surrendered" => "Abandon",
        x => x,
    }
    .to_string()
}

fn result_letter(r: &str) -> &'static str {
    match r {
        "Eliminated" => "É",
        "Bomb detonated" => "S",
        "Bomb defused" => "D",
        "Round timer expired" => "T",
        "Surrendered" => "F",
        _ => "·",
    }
}

pub fn render_match(a: &mut App) {
    let (Some(m), Some(d)) = (a.valo.m.clone(), a.valo.d.clone()) else { return };
    let puuid = a.valo.m_puuid.clone();
    let st = v_stats(&m);
    let me = m.players.iter().find(|x| x.subject == puuid);
    let my_team = me.map(|x| x.team_id.clone()).unwrap_or_else(|| "Blue".into());
    let mut teams = vec![my_team.clone()];
    for p in &m.players {
        if !teams.contains(&p.team_id) {
            teams.push(p.team_id.clone());
        }
    }
    let map = d.maps.get(&m.match_info.map_id);
    let max_acs = st.values().map(|s| s.acs).fold(1.0, f64::max);
    let eco = v_economy(&m, &my_team);
    let enemy = teams.iter().find(|t| **t != my_team).cloned().unwrap_or_else(|| my_team.clone());
    let eco_enemy = v_economy(&m, &enemy);
    let mut quick = vec![];
    let score = v_score(&m, &my_team);
    if me.is_some() && !score.is_empty() {
        let w = v_team_won(&m, &my_team);
        quick.push(stat(score, if w { "Victoire" } else { "Défaite" }, good(w)));
    }
    quick.push(stat(mmss(m.match_info.game_length_millis.unwrap_or(0) as f64 / 1000.0), "Durée", 0));
    quick.push(stat(date_short_time(m.match_info.game_start_millis), "Date", 0));
    let rounds = m.round_results.clone().unwrap_or_default();
    let round = a.valo.round;
    let btns = rounds
        .iter()
        .map(|r| {
            let i = r.round_num as usize;
            let e = eco.get(i).filter(|x| x.0 > 0);
            VRoundBtn {
                label: ss(result_letter(&r.round_result)),
                won: r.winning_team == my_team,
                selected: round == Some(i),
                half: r.round_num == 11,
                eco: ss(e.map(|x| x.1).unwrap_or("")),
                eco_tone: match e.map(|x| x.1) {
                    Some("Full") => 1,
                    Some("Semi") => 2,
                    _ => 0,
                },
                key: ss(format!("vround:{i}")),
            }
        })
        .collect();
    let groups = crate::valo_api::match_parties(&m.players);
    let player = |id: &str| m.players.iter().find(|x| x.subject == id);
    let label = |id: &str| {
        player(id)
            .map(|x| {
                let (name, _) = name_of(a, &x.subject, &x.game_name, &x.tag_line);
                if !name.is_empty() {
                    name
                } else {
                    d.agents.get(&x.character_id.to_lowercase()).map(|y| y.name.clone()).unwrap_or_else(|| "?".into())
                }
            })
            .unwrap_or_else(|| "?".into())
    };
    let face = |id: &str| player(id).map(|x| agent_icon(&d, &x.character_id)).unwrap_or_default();
    let ally = |id: &str| player(id).is_some_and(|x| x.team_id == my_team);
    let mut data = VMatchData {
        loading: false,
        error: ss(""),
        splash: ss(map.map(|x| x.2.clone()).unwrap_or_default()),
        map: ss(map.map(|x| x.0.clone()).unwrap_or_else(|| "Carte".into())),
        queue: ss(queue_label(&m.match_info.queue_id)),
        quick: ms(quick),
        has_rounds: !rounds.is_empty(),
        rounds: ms(btns),
        selected: false,
        ..Default::default()
    };
    if let Some(r) = round.and_then(|i| rounds.get(i)) {
        let i = round.unwrap();
        data.selected = true;
        data.detail_won = r.winning_team == my_team;
        data.detail_title = ss(format!("Manche {} : {}", i + 1, if data.detail_won { "gagnée" } else { "perdue" }));
        data.detail_result = ss(result_fr(&r.round_result));
        data.detail_eco = ss(eco.get(i).map(|x| format!("Équipement : {} contre {} crédits", fr(x.0), fr(eco_enemy.get(i).map(|y| y.0).unwrap_or(0)))).unwrap_or_default());
        if let Some(pl) = r.bomb_planter.as_deref().filter(|x| !x.is_empty()) {
            let mut s = format!("Spike posé{} à {} par {}", r.plant_site.as_deref().filter(|x| !x.is_empty()).map(|x| format!(" sur le site {x}")).unwrap_or_default(), mmss(r.plant_round_time.unwrap_or(0) as f64 / 1000.0), label(pl));
            if let Some(de) = r.bomb_defuser.as_deref().filter(|x| !x.is_empty()) {
                s += &format!(", désamorcé à {} par {}", mmss(r.defuse_round_time.unwrap_or(0) as f64 / 1000.0), label(de));
            }
            data.spike = ss(s);
        }
        let mut kills: Vec<&VKill> = r.player_stats.iter().flat_map(|p| p.kills.iter().flatten()).collect();
        kills.sort_by_key(|k| k.round_time);
        data.kills = ms(kills
            .iter()
            .map(|k| {
                let (wi, wt) = match &k.finishing_damage {
                    Some(f) if f.damage_type == "Weapon" => match d.weapons.get(&f.damage_item.to_lowercase()) {
                        Some(w) => (w.1.clone(), String::new()),
                        None => (String::new(), "Arme d'agent".to_string()),
                    },
                    Some(f) => (String::new(), match f.damage_type.as_str() {
                        "Melee" => "Couteau",
                        "Bomb" => "Spike",
                        _ => "Compétence",
                    }.to_string()),
                    None => (String::new(), String::new()),
                };
                VKillV { time: ss(mmss(k.round_time as f64 / 1000.0)), killer: ss(label(&k.killer)), killer_icon: ss(face(&k.killer)), killer_ally: ally(&k.killer), victim: ss(label(&k.victim)), victim_icon: ss(face(&k.victim)), victim_ally: ally(&k.victim), weapon_icon: ss(wi), weapon_text: ss(wt) }
            })
            .collect());
    }
    data.cols = ms(vec![col("", 0.0), col("Rang", 60.0), col("ACS", 130.0), col("K / D / A", 120.0), col("+/-", 60.0), col("ADR", 70.0), col("HS", 70.0), col("FK", 54.0), col("FD", 54.0), col("Multi", 96.0)]);
    let mut rows = vec![];
    for t in &teams {
        let mut list: Vec<&VPlayer> = m.players.iter().filter(|x| &x.team_id == t).collect();
        list.sort_by(|x, y| st[&y.subject].acs.total_cmp(&st[&x.subject].acs));
        if teams.len() > 1 {
            let w = v_team_won(&m, t);
            let mut head = toned(if w { "Victoire" } else { "Défaite" }, good(w));
            head.sub = ss(if *t == my_team { "Ton équipe" } else { "Adversaires" });
            let mut hr = row(vec![head, txt(""), txt(""), txt(""), txt(""), txt(""), txt(""), txt(""), txt(""), txt("")], "");
            hr.header = true;
            rows.push(hr);
        }
        for x in list {
            let s = &st[&x.subject];
            let (name, tag) = name_of(a, &x.subject, &x.game_name, &x.tag_line);
            let ps = x.stats.clone().unwrap_or_default();
            let diff = ps.kills - ps.deaths;
            let tier = d.tiers.get(&x.competitive_tier).filter(|_| x.competitive_tier > 2);
            let mut c0 = iconed(if name.is_empty() { "…".to_string() } else { name }, agent_icon(&d, &x.character_id));
            let group = groups.get(&x.subject);
            c0.sub = ss(if tag.is_empty() { String::new() } else { format!("#{tag}") });
            if let Some((_, n)) = group {
                c0.mark = Color::from_argb_encoded(0xff000000 | crate::history::party_rgb(*n));
            }
            let mut rk = txt(if tier.is_some() { "" } else { "–" });
            if let Some(t) = tier {
                rk.kind = 2;
                rk.icons = ms(vec![ss(&t.icon)]);
            }
            let mut acs = txt(((s.acs.round()) as i64).to_string());
            acs.bar = (s.acs / max_acs) as f32;
            let mk = mk_label(&s.mk).join(", ");
            let mut r = row(
                vec![c0, rk, acs, txt(format!("{} / {} / {}", ps.kills, ps.deaths, ps.assists)), toned(format!("{}{diff}", if diff > 0 { "+" } else { "" }), if diff > 0 { 1 } else if diff < 0 { 2 } else { 0 }), txt((s.adr.round() as i64).to_string()), txt(pct(s.hs)), txt(s.fk.to_string()), txt(s.fd.to_string()), txt(if mk.is_empty() { "–".to_string() } else { mk })],
                format!("v-profil/{}", x.subject),
            );
            r.me = x.subject == puuid;
            rows.push(r);
        }
    }
    data.rows = ms(rows);
    a.data().set_vmatch(data);
}

pub fn open_agents(a: &mut App) {
    a.valo.agents = None;
    a.valo.role.clear();
    a.valo.sort = "wr".into();
    a.valo.desc = true;
    a.valo.err.clear();
    a.data().set_vagents(VAgentsData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let (d, r) = tokio::join!(vdata(), opgg::call("valorant_list_agent_statistics", serde_json::json!({})));
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match (d, r) {
                (Ok(d), Ok(r)) => {
                    a.valo.d = Some(d);
                    a.valo.agents = Some(parse_agents(&r));
                    a.valo.patch = r["meta"]["gameVersion"].as_str().map(String::from).or(r["meta"]["gameVersion"].as_f64().map(|x| x.to_string())).unwrap_or_default();
                    render_agents(a);
                }
                (Err(e), _) | (_, Err(e)) => a.data().set_vagents(VAgentsData { error: ss(crate::text::friendly_error(&e)), ..Default::default() }),
            }
        });
    });
}

pub fn parse_agents(r: &Value) -> Vec<AgentRow> {
    let list = r["data"].as_array().cloned().unwrap_or_default();
    let f = |x: &Value, k: &str| x[k].as_f64().unwrap_or(0.0);
    let picks = list.iter().map(|x| f(x, "gameCount")).sum::<f64>().max(1.0);
    let mut out: Vec<AgentRow> = list
        .iter()
        .map(|x| AgentRow {
            id: x["characterId"].as_str().unwrap_or_default().to_lowercase(),
            games: f(x, "gameCount") as i64,
            wr: f(x, "wins") / (f(x, "gameCount") - f(x, "draws")).max(1.0),
            pick: f(x, "gameCount") / picks,
            kd: f(x, "kills") / f(x, "deaths").max(1.0),
            acs: f(x, "acs"),
            hs: f(x, "headShots") / (f(x, "headShots") + f(x, "bodyShots") + f(x, "legShots")).max(1.0),
            fkfd: f(x, "firstKills") / f(x, "firstDeaths").max(1.0),
            tier: 0,
        })
        .collect();
    let mut idx: Vec<usize> = (0..out.len()).collect();
    idx.sort_by(|p, q| out[*q].wr.total_cmp(&out[*p].wr));
    let len = out.len() as f64;
    for (i, k) in idx.into_iter().enumerate() {
        let r = i as f64;
        out[k].tier = if r < len * 0.15 {
            1
        } else if r < len * 0.4 {
            2
        } else if r < len * 0.75 {
            3
        } else {
            4
        };
    }
    out
}

fn render_agents(a: &mut App) {
    let (Some(d), Some(rows)) = (a.valo.d.clone(), a.valo.agents.clone()) else { return };
    let st = &a.valo;
    let mut roles: Vec<String> = rows.iter().filter_map(|x| d.agents.get(&x.id).map(|y| y.role.clone())).filter(|x| !x.is_empty()).collect();
    roles.sort();
    roles.dedup();
    let mut role_opts = vec![opt("vrole:", "Tous", st.role.is_empty())];
    for r in &roles {
        let mut o = opt(format!("vrole:{r}"), r, st.role == *r);
        o.icon = ss(d.agents.values().find(|x| x.role == *r).map(|x| x.role_icon.clone()).unwrap_or_default());
        role_opts.push(o);
    }
    let mut list: Vec<&AgentRow> = rows.iter().filter(|x| d.agents.get(&x.id).is_some_and(|y| st.role.is_empty() || y.role == st.role)).collect();
    let val = |x: &AgentRow| match st.sort.as_str() {
        "pick" => x.pick,
        "kd" => x.kd,
        "acs" => x.acs,
        "hs" => x.hs,
        "fkfd" => x.fkfd,
        "games" => x.games as f64,
        _ => x.wr,
    };
    list.sort_by(|x, y| if st.desc { val(y).total_cmp(&val(x)) } else { val(x).total_cmp(&val(y)) });
    let th = |k: &str, l: &str, px: f32| sort_col(l, px, format!("vsort:{k}"), st.sort == k, st.desc);
    let cols = vec![col("#", 44.0), col("Agent", 0.0), col("Tier", 70.0), th("wr", "Winrate", 90.0), th("pick", "Part des choix", 120.0), th("kd", "K/D", 70.0), th("acs", "ACS", 70.0), th("hs", "HS", 70.0), th("fkfd", "FK/FD", 80.0), th("games", "Parties", 100.0)];
    let out = list
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let ag = &d.agents[&x.id];
            let mut name = iconed(&ag.name, &ag.icon);
            name.sub = ss(&ag.role);
            let mut tier = txt(["", "S", "A", "B", "C"][x.tier as usize]);
            tier.kind = 1;
            tier.tone = x.tier as i32;
            row(vec![toned((i + 1).to_string(), 4), name, tier, toned(pct1(x.wr), good(x.wr >= 0.5)), txt(pct1(x.pick)), txt(format!("{:.2}", x.kd)), txt((x.acs.round() as i64).to_string()), txt(pct1(x.hs)), toned(format!("{:.2}", x.fkfd), good(x.fkfd >= 1.0)), txt(fr(x.games))], "")
        })
        .collect();
    a.data().set_vagents(VAgentsData {
        loading: false,
        error: ss(""),
        sub: ss(format!("Compétition{}", if st.patch.is_empty() { String::new() } else { format!(", patch {}", st.patch) })),
        roles: ms(role_opts),
        cols: ms(cols),
        rows: ms(out),
    });
}

pub fn open_live(a: &mut App) {
    a.valo.live = None;
    vdata_ready(a, |a| {
        render_live(a);
        load_live(a);
    });
    render_live(a);
    let gen = a.gen;
    spawn(async move {
        loop {
            let (tx, rx) = tokio::sync::oneshot::channel();
            on_ui(move |a| {
                let alive = a.gen == gen;
                let pre = a.vph.as_deref() == Some("PREGAME");
                if alive {
                    load_live(a);
                }
                let _ = tx.send((alive, pre));
            });
            let (alive, pre) = rx.await.unwrap_or((false, false));
            if !alive {
                return;
            }
            tokio::time::sleep(Duration::from_secs(if pre { 3 } else { 15 })).await;
        }
    });
}

pub fn parse_live(m: &Value, pre: bool) -> LiveGame {
    let raw = if pre { m["AllyTeam"]["Players"].as_array() } else { m["Players"].as_array() };
    let ally = m["AllyTeam"]["TeamID"].as_str().unwrap_or_default();
    let players = raw
        .into_iter()
        .flatten()
        .map(|x| LiveP {
            puuid: x["Subject"].as_str().unwrap_or_default().into(),
            agent: x["CharacterID"].as_str().unwrap_or_default().into(),
            team: x["TeamID"].as_str().unwrap_or(ally).into(),
            incognito: x["PlayerIdentity"]["Incognito"] == true,
            level: if x["PlayerIdentity"]["HideAccountLevel"] == true { 0 } else { x["PlayerIdentity"]["AccountLevel"].as_i64().unwrap_or(0) },
            locked: if pre { Some(x["CharacterSelectionState"] == "locked") } else { None },
        })
        .collect();
    LiveGame { map: m["MapID"].as_str().unwrap_or_default().into(), queue: m["QueueID"].as_str().or(m["MatchmakingData"]["QueueID"].as_str()).unwrap_or_default().into(), players }
}

pub fn load_live(a: &mut App) {
    let phase = a.vph.clone().unwrap_or_default();
    if phase != "PREGAME" && phase != "INGAME" {
        if a.valo.live.take().is_some() {
            render_live(a);
        }
        return;
    }
    let (Some(me), gen) = (a.vself.clone(), a.gen) else { return };
    spawn(async move {
        let pre = phase == "PREGAME";
        let base = if pre { "pregame" } else { "core-game" };
        let r: Result<Value, String> = async {
            let p = valo("glz", "GET", &format!("/{base}/v1/players/{me}"), None).await?;
            valo("glz", "GET", &format!("/{base}/v1/matches/{}", p["MatchID"].as_str().unwrap_or_default()), None).await
        }
        .await;
        let Ok(m) = r else { return };
        let game = parse_live(&m, pre);
        let ids: Vec<String> = game.players.iter().map(|p| p.puuid.clone()).collect();
        let parties = v_parties(&ids, &me).await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            a.valo.parties = parties;
            a.valo.live = Some(game);
            ask_names(a, ids.clone());
            let st = &a.valo;
            let need: Vec<String> = ids.iter().filter(|p| !st.ranks.contains_key(*p)).cloned().collect();
            for p in need {
                spawn(async move {
                    let (r, u) = tokio::join!(v_rank(&p), v_rr_updates(&p));
                    let form: Vec<i64> = u.iter().take(5).map(|x| x.1.earned).collect();
                    on_ui(move |a| {
                        a.valo.ranks.insert(p.clone(), r);
                        a.valo.forms.insert(p, form);
                        if a.page() == "v-partie" {
                            render_live(a);
                        }
                    });
                });
            }
            render_live(a);
        });
    });
}

fn party_color(c: usize) -> Color {
    Color::from_argb_encoded(0xff000000 | [0xf6c867, 0x6fa8ff, 0xc58cff, 0xff9a62, 0x5fe0c8][c % 5])
}

pub fn render_live(a: &mut App) {
    let st = &a.valo;
    let phase = a.vph.clone().unwrap_or_default();
    let active = phase == "PREGAME" || phase == "INGAME";
    let (Some(game), Some(d)) = (&st.live, &st.d) else {
        a.data().set_vlive(VLiveData { waiting: true, status: ss(if active { "Chargement de la partie" } else { "En attente d'une partie" }), ..Default::default() });
        return;
    };
    let self_ = a.vself.clone().unwrap_or_default();
    let my_team = game.players.iter().find(|p| p.puuid == self_).map(|p| p.team.clone()).unwrap_or_default();
    let mut teams = vec![my_team.clone()];
    for p in &game.players {
        if !teams.contains(&p.team) {
            teams.push(p.team.clone());
        }
    }
    let mut groups: Vec<(String, Vec<String>)> = vec![];
    for p in &game.players {
        if let Some(id) = st.parties.get(&p.puuid) {
            match groups.iter_mut().find(|g| &g.0 == id) {
                Some(g) => g.1.push(p.puuid.clone()),
                None => groups.push((id.clone(), vec![p.puuid.clone()])),
            }
        }
    }
    let mut party: HashMap<String, (&'static str, usize)> = HashMap::new();
    for (i, g) in groups.iter().filter(|g| g.1.len() > 1).enumerate() {
        for id in &g.1 {
            party.insert(id.clone(), (PARTY_LABEL[g.1.len().min(5)], i % 5));
        }
    }
    let pre = phase == "PREGAME";
    let locked = game.players.iter().filter(|p| p.locked == Some(true)).count();
    let tlist: Vec<VLiveTeam> = teams
        .iter()
        .filter(|t| !t.is_empty() || teams.len() == 1)
        .map(|t| {
            let list: Vec<&LiveP> = game.players.iter().filter(|p| &p.team == t).collect();
            let tiers: Vec<i64> = list.iter().filter_map(|p| st.ranks.get(&p.puuid).map(|r| r.tier)).filter(|x| *x > 2).collect();
            let avg = if tiers.is_empty() { 0 } else { (tiers.iter().sum::<i64>() as f64 / tiers.len() as f64).round() as i64 };
            let at = d.tiers.get(&avg).filter(|_| avg > 0);
            let mut seen = vec![];
            let mut parties = vec![];
            for p in &list {
                if let Some(x) = party.get(&p.puuid) {
                    if !seen.contains(&x.1) {
                        seen.push(x.1);
                        parties.push(VParty { text: ss(x.0), color: party_color(x.1) });
                    }
                }
            }
            let rows = list
                .iter()
                .map(|p| {
                    let ag = d.agents.get(&p.agent.to_lowercase());
                    let hidden = p.incognito && p.puuid != self_;
                    let name = st.names.get(&p.puuid);
                    let r = st.ranks.get(&p.puuid);
                    let tier = r.and_then(|r| d.tiers.get(&r.tier).filter(|_| r.tier > 2));
                    let form = st.forms.get(&p.puuid);
                    let net: i64 = form.map(|f| f.iter().sum()).unwrap_or(0);
                    let pinfo = party.get(&p.puuid);
                    VLiveRow {
                        me: p.puuid == self_,
                        hidden,
                        route: ss(if hidden { String::new() } else { format!("v-profil/{}", p.puuid) }),
                        face: ss(ag.map(|x| x.icon.clone()).unwrap_or_default()),
                        hover: p.locked == Some(false),
                        role_icon: ss(ag.map(|x| x.role_icon.clone()).unwrap_or_default()),
                        name: ss(if hidden { "Joueur masqué".to_string() } else { name.map(|n| n.game_name.clone()).unwrap_or_else(|| "…".into()) }),
                        tag: ss(if hidden { String::new() } else { name.map(|n| n.tag_line.clone()).unwrap_or_default() }),
                        sub: ss(ag.map(|x| x.name.clone()).unwrap_or_else(|| "Pas encore choisi".into())),
                        party: ss(pinfo.map(|x| x.0).unwrap_or("")),
                        party_color: pinfo.map(|x| party_color(x.1)).unwrap_or_default(),
                        in_party: pinfo.is_some(),
                        locked: p.locked == Some(true),
                        level: ss(if p.level > 0 { format!("niv. {}", p.level) } else { String::new() }),
                        loaded: r.is_some(),
                        rank_icon: ss(tier.map(|x| x.icon.clone()).unwrap_or_default()),
                        rank: ss(match (tier, r) {
                            (Some(t), Some(r)) => format!("{}, {} RR", t.name, r.rr),
                            _ => "Non classé".to_string(),
                        }),
                        rank_color: tier.map(|t| hex_color(&t.color)).unwrap_or(Color::from_rgb_u8(0xee, 0xf0, 0xf7)),
                        peak: ss(r.filter(|r| r.peak > 2).and_then(|r| d.tiers.get(&r.peak)).map(|t| format!("Max {}", t.name)).unwrap_or_else(|| "Pas de rang max".into())),
                        wr: ss(r.filter(|r| r.games > 0).map(|r| pct(r.wins as f64 / r.games as f64)).unwrap_or_else(|| "–".into())),
                        wr_tone: r.map(|r| good(r.games > 0 && r.wins as f64 / r.games.max(1) as f64 >= 0.5)).unwrap_or(4),
                        games: ss(r.filter(|r| r.games > 0).map(|r| format!("{} parties", r.games)).unwrap_or_default()),
                        form: ms(form.map(|f| f.iter().rev().map(|x| *x as i32).collect()).unwrap_or_default()),
                        net: ss(if form.is_some_and(|f| !f.is_empty()) { format!("{} RR", signed(net)) } else { "–".to_string() }),
                        net_tone: good(net >= 0),
                    }
                })
                .collect();
            VLiveTeam {
                title: ss(if *t == my_team { "Ton équipe" } else { "Adversaires" }),
                mine: *t == my_team,
                parties: ms(parties),
                avg_icon: ss(at.map(|x| x.icon.clone()).unwrap_or_default()),
                avg: ss(at.map(|x| x.name.clone()).unwrap_or_default()),
                avg_color: at.map(|x| hex_color(&x.color)).unwrap_or_default(),
                rows: ms(rows),
            }
        })
        .collect();
    let map = d.maps.get(&game.map);
    let data = VLiveData {
        waiting: false,
        status: ss(""),
        splash: ss(map.map(|x| x.2.clone()).unwrap_or_default()),
        pre,
        phase: ss(if pre { "Sélection des agents" } else { "En partie" }),
        map: ss(map.map(|x| x.0.clone()).unwrap_or_else(|| "Carte".into())),
        queue: ss(queue_label(&game.queue)),
        text: ss(if pre { format!("{locked} agent{} verrouillé{} sur {}. Les adversaires apparaîtront au lancement de la partie.", if locked > 1 { "s" } else { "" }, if locked > 1 { "s" } else { "" }, game.players.len()) } else { "Clique sur un joueur pour ouvrir son profil.".to_string() }),
        teams: ms(tlist),
    };
    a.data().set_vlive(data);
}

fn load_more(a: &mut App) {
    if a.valo.more != "idle" {
        return;
    }
    let Some(ids) = a.valo.ids.clone() else { return };
    a.valo.more = "loading".into();
    render_profile(a, false);
    let (who, queue, gen) = (a.valo.who.clone(), a.valo.queue.clone(), a.gen);
    spawn(async move {
        let r = v_history(&who, PAGE, &queue, ids.len()).await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match r {
                Ok(h) => {
                    let list: Vec<String> = h.into_iter().filter(|x| !ids.contains(x)).collect();
                    a.valo.more = if list.len() < PAGE { "end".into() } else { "idle".into() };
                    let mut all = ids.clone();
                    all.extend(list.clone());
                    a.valo.ids = Some(all);
                    render_profile(a, true);
                    track(a, list);
                }
                Err(_) => {
                    a.valo.more = "idle".into();
                    render_profile(a, false);
                }
            }
        });
    });
}

pub fn action(a: &mut App, key: &str) -> bool {
    if let Some(k) = key.strip_prefix("vtab:") {
        a.valo.tab = k.into();
        render_profile(a, false);
    } else if let Some(k) = key.strip_prefix("vq:") {
        a.valo.queue = k.into();
        load_ids(a);
        render_profile(a, true);
    } else if let Some(id) = key.strip_prefix("vretry:") {
        a.valo.failed.remove(id);
        update_row(a, id);
        track(a, vec![id.to_string()]);
    } else if key == "vmore" {
        load_more(a);
    } else if let Some(k) = key.strip_prefix("vround:") {
        let i: usize = k.parse().unwrap_or(0);
        a.valo.round = if a.valo.round == Some(i) { None } else { Some(i) };
        render_match(a);
    } else if let Some(k) = key.strip_prefix("vrole:") {
        a.valo.role = k.into();
        render_agents(a);
    } else if let Some(k) = key.strip_prefix("vsort:") {
        let same = a.valo.sort == k;
        a.valo.desc = if same { !a.valo.desc } else { true };
        a.valo.sort = k.into();
        render_agents(a);
    } else if key == "copy-id" && a.game == "valo" {
        if let Some(n) = a.valo.name.clone() {
            copy_text(a, &format!("{}#{}", n.game_name, n.tag_line), "Riot ID");
        }
    } else {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valorant_csv_escapes_and_has_a_header() {
        let csv = matches_csv(&[vec!["1 nov".into(), "Ascent".into(), "Jett".into(), "Compétition".into(), "Victoire".into(), "13-7".into(), "20".into(), "10".into(), "5".into(), "250".into(), "25%".into()]]);
        assert!(csv.starts_with("\u{feff}\"date\";\"carte\""));
        assert!(csv.contains("\"Ascent\";\"Jett\""));
        assert_eq!(csv.lines().count(), 2);
    }
    use serde_json::json;

    #[test]
    fn agents_tiers() {
        let list: Vec<Value> = (0..20).map(|i| json!({"characterId": format!("A{i}"), "gameCount": 100, "wins": 100 - i, "kills": 10, "deaths": 5, "headShots": 1, "bodyShots": 1, "legShots": 2, "firstKills": 3, "firstDeaths": 2, "acs": 200})).collect();
        let r = parse_agents(&json!({"data": list}));
        assert_eq!(r[0].id, "a0");
        assert_eq!(r[0].tier, 1);
        assert_eq!(r[19].tier, 4);
        assert_eq!(r[0].kd, 2.0);
        assert_eq!(r[0].hs, 0.25);
        assert_eq!(r[0].pick, 0.05);
    }

    #[test]
    fn live_parse() {
        let m = json!({"AllyTeam": {"TeamID": "Blue", "Players": [{"Subject": "a", "CharacterID": "X", "CharacterSelectionState": "locked", "PlayerIdentity": {"AccountLevel": 7}}, {"Subject": "b", "CharacterSelectionState": "selected", "PlayerIdentity": {"Incognito": true, "HideAccountLevel": true, "AccountLevel": 9}}]}, "MapID": "/m"});
        let g = parse_live(&m, true);
        assert_eq!(g.players.len(), 2);
        assert_eq!(g.players[0].team, "Blue");
        assert_eq!(g.players[0].locked, Some(true));
        assert!(g.players[1].incognito);
        assert_eq!(g.players[1].level, 0);
    }
}
