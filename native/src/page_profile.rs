use crate::history::{versus, toggle_pin, goal_text, next_goal, insights, Slot, streak, by_queue, champs_in, counted, cs, filter_games, form_of, is_remake, kda, merge_games, n, queues_in, win, Game, HistoryFilter, Participant};
use crate::lol::*;
use crate::text::*;
use crate::{good, ms, on_ui, opt, spawn, ss, stat, store, with_app, App};
use crate::{Versus, Badge, Cell, ChartData, ChartSeries, AxisLabel, Col, LpData, LpPoint, MatchPage, MatchRow, ObjTeam, Perf, PlayerChip, ProfileData, RankRow, Score, ShareData, TRow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slint::{Color, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

const PAGE: usize = 20;
const KEPT_GAMES: usize = 150;
const SMITE: i64 = 11;

#[derive(Default)]
pub struct ProfileState {
    pub puuid: Option<String>,
    pub period: i64,
    pub sum: Summoner,
    pub ranks: HashMap<String, Queue>,
    pub games: Option<Vec<Game>>,
    pub queues: Arc<HashMap<i64, String>>,
    pub err: String,
    pub sub: String,
    pub filter: HistoryFilter,
    pub visible: usize,
    pub details: HashMap<i64, Arc<Game>>,
    pub lp: Vec<LpPt>,
    pub rows: Option<Rc<VecModel<MatchRow>>>,
    pub shown: Vec<i64>,
    pub ready: bool,
    pub session_copy: String,
    pub mastery: Vec<(i64, i64, i64)>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LpPt {
    pub t: i64,
    pub v: i64,
}

thread_local! {
    static MINI: std::cell::RefCell<Option<crate::MiniWindow>> = const { std::cell::RefCell::new(None) };
}

fn fill_mini(record: &str, rate: &str, kda: &str, lp: &str, goal: &str) {
    MINI.with(|m| {
        if let Some(w) = m.borrow().as_ref() {
            w.set_record(ss(record));
            w.set_rate(ss(rate));
            w.set_kda(ss(kda));
            w.set_lp(ss(lp));
            w.set_goal(ss(goal));
        }
    });
}

fn toggle_mini() -> bool {
    use slint::ComponentHandle;
    MINI.with(|m| {
        let mut slot = m.borrow_mut();
        if let Some(w) = slot.take() {
            let _ = w.hide();
            return false;
        }
        match crate::MiniWindow::new() {
            Ok(w) => {
                let _ = w.show();
                *slot = Some(w);
                true
            }
            Err(_) => false,
        }
    })
}

fn keep_games(puuid: &str, fresh: Vec<Game>) -> Vec<Game> {
    let key = format!("rw-games-{puuid}");
    let kept: Vec<Game> = store::load(&key).unwrap_or_default();
    let all = merge_games(fresh, kept, KEPT_GAMES);
    store::save(&key, &all);
    all
}

fn lp_history(puuid: &str, q: Option<&Queue>) -> Vec<LpPt> {
    let key = format!("rw-lp-{puuid}");
    let mut pts: Vec<LpPt> = store::load(&key).unwrap_or_default();
    if let Some(q) = q.filter(|q| !q.tier.is_empty() && q.tier != "NONE") {
        let v = rank_score(q);
        if pts.last().map(|p| p.v) != Some(v) {
            pts.push(LpPt { t: now_ms(), v });
            if pts.len() > 200 {
                pts.drain(..pts.len() - 200);
            }
            store::save(&key, &pts);
        }
    }
    pts
}

pub fn badges(g: &Game, puuid: &str) -> Vec<&'static str> {
    let Some(p) = g.find(puuid) else { return vec![] };
    let team: Vec<&Participant> = g.participants.iter().filter(|x| x.team_id == p.team_id).collect();
    let all: Vec<&Participant> = g.participants.iter().collect();
    let top = |list: &[&Participant], f: &dyn Fn(&Participant) -> f64| list.iter().map(|x| f(x)).fold(f64::MIN, f64::max) == f(p);
    let mut out = vec![];
    if top(&all, &|x| kda(&x.stats) * 3.0 + n(&x.stats, "totalDamageDealtToChampions") / 4000.0) {
        out.push("MVP");
    } else if win(p) && top(&team, &|x| kda(&x.stats)) {
        out.push("ACE");
    }
    if top(&all, &|x| n(&x.stats, "totalDamageDealtToChampions")) {
        out.push("Top dégâts");
    }
    if top(&all, &|x| n(&x.stats, "goldEarned")) {
        out.push("Cash Money");
    }
    if top(&all, &|x| n(&x.stats, "visionScore")) && g.map_id == 11 {
        out.push("Vision");
    }
    if cs(&p.stats) / (g.game_duration as f64 / 60.0) >= 8.0 {
        out.push("Farmeur");
    }
    let multi = n(&p.stats, "largestMultiKill") as usize;
    if multi >= 2 {
        out.push(["", "", "Double", "Triple", "Quadra", "Penta"][multi.min(5)]);
    }
    out
}

fn badge_list(list: &[&str], limit: usize) -> ModelRc<Badge> {
    ms(list.iter().take(limit).map(|b| Badge { text: ss(b), gold: *b == "MVP" || *b == "ACE" }).collect())
}

fn k1(v: f64) -> String {
    format!("{:.1}k", v / 1000.0)
}

fn match_row(g: &Game, puuid: &str, d: &DD, queues: &HashMap<i64, String>, full: Option<&Game>) -> MatchRow {
    let p = g.me();
    let s = &p.stats;
    let min = g.game_duration as f64 / 60.0;
    let remake = g.game_duration < 300;
    let result = if remake { 2 } else if win(p) { 0 } else { 1 };
    let sr = g.map_id == 11;
    let mut kp = String::new();
    if let Some(me) = full.and_then(|f| f.find(puuid)) {
        let f = full.unwrap();
        let team_kills: f64 = f.participants.iter().filter(|x| x.team_id == me.team_id).map(|x| n(&x.stats, "kills")).sum();
        if team_kills > 0.0 {
            kp = format!(", {}% KP", (((n(s, "kills") + n(s, "assists")) / team_kills) * 100.0).round());
        }
    }
    let rune = |id: i64| if d.runes.contains_key(&id) { img_rune(d, id) } else { String::new() };
    let icons = vec![ss(img_spell(d, p.spell1_id)), ss(img_spell(d, p.spell2_id)), ss(rune(n(s, "perk0") as i64)), ss(rune(n(s, "perkSubStyle") as i64))];
    let item = |k: i32| {
        let id = n(s, &format!("item{k}")) as i64;
        if id != 0 {
            ss(img_item(d, id))
        } else {
            ss("")
        }
    };
    let team = |tid: i64| -> ModelRc<PlayerChip> {
        let Some(f) = full else { return ms(vec![]) };
        ms(f.participants
            .iter()
            .filter(|x| x.team_id == tid)
            .map(|x| {
                let who = f.who(x.participant_id);
                let name = who.map(|w| if w.game_name.is_empty() { w.summoner_name.clone() } else { w.game_name.clone() }).filter(|x| !x.is_empty()).unwrap_or_else(|| "?".into());
                let pu = who.map(|w| w.puuid.clone()).unwrap_or_default();
                PlayerChip { icon: ss(img_champ(d, x.champion_id)), name: ss(name), me: pu == puuid, route: ss(if pu.is_empty() { String::new() } else { format!("profil/{pu}") }) }
            })
            .collect())
    };
    let bd = full.map(|f| badges(f, puuid)).unwrap_or_default();
    MatchRow {
        route: ss(format!("match/{}/{puuid}", g.game_id)),
        result,
        title: ss(if remake { "Remake" } else if win(p) { "Victoire" } else { "Défaite" }),
        queue: ss(queues.get(&g.queue_id).cloned().unwrap_or_else(|| format!("File {}", g.queue_id))),
        when: ss(format!("{}, {}", mmss(g.game_duration as f64), ago(g.game_creation))),
        champ: ss(img_champ(d, p.champion_id)),
        level: ss((n(s, "champLevel") as i64).to_string()),
        icons: ms(icons),
        k: ss((n(s, "kills") as i64).to_string()),
        d: ss((n(s, "deaths") as i64).to_string()),
        a: ss((n(s, "assists") as i64).to_string()),
        kda: ss(format!("{:.2} KDA", kda(s))),
        kda_good: kda(s) >= 3.0,
        kp: ss(kp),
        csmin: ss(format!("{:.1}", cs(s) / min.max(0.01))),
        dmg: ss(k1(n(s, "totalDamageDealtToChampions"))),
        third_v: ss(if sr { (n(s, "visionScore") as i64).to_string() } else { k1(n(s, "goldEarned")) }),
        third_l: ss(if sr { "vision" } else { "or" }),
        items: ms((0..6).map(item).collect()),
        trinket: item(6),
        badges: badge_list(&bd, 3),
        team1: team(100),
        team2: team(200),
    }
}

pub fn open_profile(a: &mut App, puuid: Option<String>) {
    a.profile = ProfileState { puuid: puuid.clone(), sub: "apercu".into(), visible: PAGE, ..Default::default() };
    a.data().set_profile(ProfileData { loading: true, games_loading: true, ..Default::default() });
    a.data().set_matches(ms(vec![]));
    let gen = a.gen;
    if puuid.is_none() {
        spawn(async move {
            let Ok(v) = crate::net::get("/lol-champion-mastery/v1/local-player/champion-mastery").await else { return };
            let mut list: Vec<(i64, i64, i64)> = v.as_array().into_iter().flatten().map(|m| (m["championId"].as_i64().unwrap_or(0), m["championLevel"].as_i64().unwrap_or(0), m["championPoints"].as_i64().unwrap_or(0))).collect();
            list.sort_by_key(|x| std::cmp::Reverse(x.2));
            on_ui(move |a| {
                if a.gen == gen {
                    a.profile.mastery = list;
                    if a.profile.ready {
                        render(a, false);
                    }
                }
            });
        });
    }
    spawn(async move {
        let r: Result<_, String> = async {
            let s = match &puuid {
                Some(p) => summoner(p).await?,
                None => me().await?,
            };
            let (r, h, dd, q) = tokio::join!(ranked(&s.puuid), history(&s.puuid, PAGE as i64), ddragon(), queue_names());
            Ok((s, r.unwrap_or_default(), h?, dd?, q.unwrap_or_default()))
        }
        .await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match r {
                Ok((s, ranks, h, dd, queues)) => {
                    let st = &mut a.profile;
                    let games = if st.puuid.is_some() { h } else { keep_games(&s.puuid, h) };
                    st.lp = lp_history(&s.puuid, ranks.get("RANKED_SOLO_5x5"));
                    st.sum = s;
                    st.ranks = ranks;
                    st.queues = queues;
                    st.games = Some(games);
                    st.ready = true;
                    a.d = Some(dd);
                    render(a, true);
                }
                Err(e) => {
                    a.profile.err = e.clone();
                    a.data().set_profile(ProfileData { error: ss(crate::text::friendly_error(&e)), ..Default::default() });
                }
            }
        });
    });
}

fn lp_data(pts: &[LpPt]) -> LpData {
    let lo = pts.iter().map(|p| p.v).min().unwrap_or(0) as f64;
    let hi = pts.iter().map(|p| p.v).max().unwrap_or(0) as f64;
    let (w, h) = (260.0, 60.0);
    let last = pts.len().max(2) - 1;
    let x = |i: usize| i as f64 / last as f64 * w;
    let y = |v: i64| h - 3.0 - (v as f64 - lo) / (hi - lo).max(1.0) * (h - 6.0);
    let path = pts.iter().enumerate().map(|(i, p)| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x(i), y(p.v))).collect::<Vec<_>>().join(" ");
    let first = pts.first().map(|p| p.v).unwrap_or(0);
    let total = pts.last().map(|p| p.v).unwrap_or(0) - first;
    let days = ((pts.last().map(|p| p.t).unwrap_or(0) - pts.first().map(|p| p.t).unwrap_or(0)) as f64 / 864e5).round().max(1.0) as i64;
    LpData {
        total: ss(format!("{} LP", signed(total))),
        tone: good(total >= 0),
        span: ss(if days > 1 { format!("sur {days} jours") } else { "aujourd'hui".to_string() }),
        path: ss(path),
        points: ms(pts.iter().enumerate().map(|(i, p)| LpPoint { x: x(i) as f32, y: y(p.v) as f32, tip: ss(format!("{} : {} LP depuis le début", date_short(p.t), signed(p.v - first))) }).collect()),
    }
}

fn cell(text: impl AsRef<str>, tone: i32) -> Cell {
    Cell { text: ss(text), tone, bar: -1.0, icons: ms(vec![]), ..Default::default() }
}

pub fn txt(text: impl AsRef<str>) -> Cell {
    cell(text, 0)
}

pub fn toned(text: impl AsRef<str>, tone: i32) -> Cell {
    cell(text, tone)
}

pub fn iconed(text: impl AsRef<str>, icon: impl AsRef<str>) -> Cell {
    let mut c = cell(text, 0);
    c.icon = ss(icon);
    c
}

pub fn col(label: &str, px: f32) -> Col {
    Col { label: ss(label), px, ..Default::default() }
}

pub fn sort_col(label: &str, px: f32, key: String, active: bool, desc: bool) -> Col {
    Col { label: ss(label), px, sort: ss(key), active, desc }
}

pub fn row(cells: Vec<Cell>, route: impl AsRef<str>) -> TRow {
    TRow { cells: ms(cells), route: ss(route), me: false, header: false }
}

pub fn render(a: &mut App, rebuild_rows: bool) {
    let Some(dd) = a.d.clone() else { return };
    let dark = Color::from_rgb_u8(0xee, 0xf0, 0xf7);
    let st = &a.profile;
    let Some(games) = &st.games else { return };
    let shown = filter_games(games, &st.filter);
    let played = counted(&shown);
    let mut pool: Vec<(i64, [f64; 4])> = vec![];
    for g in &played {
        let p = g.me();
        let v = [1.0, if win(p) { 1.0 } else { 0.0 }, kda(&p.stats), cs(&p.stats) / (g.game_duration as f64 / 60.0)];
        match pool.iter_mut().find(|x| x.0 == p.champion_id) {
            Some(x) => {
                for i in 0..4 {
                    x.1[i] += v[i];
                }
            }
            None => pool.push((p.champion_id, v)),
        }
    }
    pool.sort_by(|x, y| y.1[0].total_cmp(&x.1[0]));
    let wins = played.iter().filter(|g| win(g.me())).count();
    let plen = played.len();
    let form = form_of(&shown, |g| kda(&g.me().stats));
    let cname = |id: i64| dd.champs.get(&id).map(|c| c.name.clone()).unwrap_or_default();
    let mut quick = vec![];
    if plen > 0 {
        let wr = wins as f64 / plen as f64;
        quick.push(stat(pct0(wr), "Winrate récent", good(wr >= 0.5)));
        quick.push(stat(format!("{:.2}", played.iter().map(|g| kda(&g.me().stats)).sum::<f64>() / plen as f64), "KDA moyen", 0));
        quick.push(stat(plen.to_string(), "Parties analysées", 0));
        if let Some(c) = pool.first() {
            quick.push(stat(cname(c.0), "Champion favori", 0));
        }
    }
    let splash = pool.first().and_then(|c| dd.champs.get(&c.0)).map(|c| img_splash(&c.id)).unwrap_or_default();
    let own = st.puuid.is_none();
    let icon = img_icon(&dd, st.sum.profile_icon_id);
    let avatar = if own { crate::page_misc::avatar_or(&icon) } else { icon };
    let midnight = midnight_ms();
    let today: Vec<&Game> = games.iter().filter(|g| g.game_creation >= midnight && !is_remake(g)).collect();
    let tw = today.iter().filter(|g| win(g.me())).count();
    let before = st.lp.iter().filter(|p| p.t < midnight).next_back();
    let delta = before.and_then(|b| st.lp.last().map(|l| l.v - b.v));
    let tkda = if today.is_empty() { 0.0 } else { today.iter().map(|g| kda(&g.me().stats)).sum::<f64>() / today.len() as f64 };
    let twr = if today.is_empty() { 0.0 } else { tw as f64 / today.len() as f64 };
    let mut session = vec![stat(format!("{tw}V {}D", today.len() - tw), format!("{}% de victoires", (twr * 100.0).round()), 0), stat(format!("{tkda:.2}"), "KDA moyen", 0)];
    if let Some(dl) = delta {
        session.push(stat(format!("{} LP", signed(dl)), "Solo/Duo", good(dl >= 0)));
    }
    let mut recent: Vec<&Game> = games.iter().filter(|g| !is_remake(g)).collect();
    recent.sort_by_key(|g| std::cmp::Reverse(g.game_creation));
    let run = streak(recent.iter().map(|g| win(g.me())));
    let settings = crate::store::settings();
    let session_note = match run {
        Some((false, count)) if count >= 3 && settings.tilt_advice => format!("{count} défaites d'affilée : une pause de 10 minutes aide souvent à repartir."),
        _ => String::new(),
    };
    if own {
        fill_mini(
            &format!("{tw}V {}D", today.len() - tw),
            &format!("{} % de victoires aujourd'hui", (twr * 100.0).round()),
            &format!("{tkda:.2}"),
            &delta.map(|x| format!("{} LP", signed(x))).unwrap_or_default(),
            &if crate::store::settings().session_goal > 0 { goal_text(crate::store::settings().session_goal, tw) } else { String::new() },
        );
    }
    let session_copy = format!("Session du jour : {tw}V {}D ({} %), KDA {tkda:.2}{}", today.len() - tw, (twr * 100.0).round(), delta.map(|x| format!(", {} LP", signed(x))).unwrap_or_default());
    let queue_label = |k: i64| st.queues.get(&k).cloned().unwrap_or_else(|| format!("File {k}"));
    let mut queue_opts = vec![opt("q:0", "Tous les modes", st.filter.queue == 0)];
    queue_opts.extend(queues_in(games).into_iter().map(|q| opt(format!("q:{q}"), queue_label(q), st.filter.queue == q)));
    let mut champ_ids = champs_in(games);
    champ_ids.sort_by_key(|c| fold(&cname(*c)));
    let mut champ_opts = vec![opt("c:0", "Tous les champions", st.filter.champ == 0)];
    champ_opts.extend(champ_ids.into_iter().map(|c| {
        let name = cname(c);
        opt(format!("c:{c}"), if name.is_empty() { c.to_string() } else { name }, st.filter.champ == c)
    }));
    let periods = [(0i64, "Toute période"), (1, "Aujourd'hui"), (7, "7 jours"), (30, "30 jours")].iter().map(|(k, l)| opt(format!("p:{k}"), l, st.period == *k)).collect();
    let results = [("", "Toutes"), ("win", "Victoires"), ("loss", "Défaites")].iter().map(|(k, l)| opt(format!("r:{k}"), l, st.filter.result == *k)).collect();
    let ranks = [("RANKED_SOLO_5x5", "Classé Solo/Duo"), ("RANKED_FLEX_SR", "Classé Flex"), ("CHERRY", "Arena")]
        .iter()
        .map(|(k, l)| {
            let q = st.ranks.get(*k);
            RankRow {
                emblem: ss(q.filter(|q| has_rank(Some(q))).map(|q| img_emblem(&q.tier)).unwrap_or_default()),
                has: has_rank(q),
                label: ss(l),
                value: ss(rank_label(q)),
                color: q.filter(|q| has_rank(Some(q))).map(|q| tier_color(&q.tier)).unwrap_or(dark),
                sub: ss(q.filter(|q| q.wins + q.losses > 0).map(|q| format!("{}V {}D, {}%", q.wins, q.losses, (q.wins as f64 / (q.wins + q.losses) as f64 * 100.0).round())).unwrap_or_default()),
            }
        })
        .collect();
    let perfs = pool
        .iter()
        .take(6)
        .map(|(c, x)| Perf {
            icon: ss(img_champ(&dd, *c)),
            title: ss(cname(*c)),
            sub: ss(format!("{:.1} KDA", x[2] / x[0])),
            right: ss(pct0(x[1] / x[0])),
            right_sub: ss(format!("{} parties", x[0])),
            tone: good(x[1] / x[0] >= 0.5),
            route: ss(format!("champion/{c}")),
        })
        .collect();
    let pool_rows = pool
        .iter()
        .map(|(c, x)| row(vec![iconed(cname(*c), img_champ(&dd, *c)), txt(x[0].to_string()), toned(pct0(x[1] / x[0]), good(x[1] / x[0] >= 0.5)), txt(format!("{:.2}", x[2] / x[0])), txt(format!("{:.1}", x[3] / x[0]))], format!("champion/{c}")))
        .collect();
    let filtered = st.filter != HistoryFilter::default();
    let (name, tag, level) = (st.sum.game_name.clone(), st.sum.tag_line.clone(), st.sum.summoner_level);
    let data = ProfileData {
        loading: false,
        error: ss(""),
        splash: ss(splash),
        avatar: ss(avatar),
        level: ss(level.to_string()),
        name: ss(name),
        tag: ss(tag),
        quick: ms(quick),
        sub: ss(&st.sub),
        tabs: ms(vec![opt("sub:apercu", "Aperçu", st.sub == "apercu"), opt("sub:pool", "Champions", st.sub == "pool")]),
        show_filters: !games.is_empty(),
        queue_label: ss(if st.filter.queue == 0 { "Tous les modes".to_string() } else { queue_label(st.filter.queue) }),
        queue_opts: ms(queue_opts),
        champ_label: ss(if st.filter.champ == 0 { "Tous les champions".to_string() } else { cname(st.filter.champ) }),
        champ_opts: ms(champ_opts),
        results: ms(results),
        periods: ms(periods),
        period_label: ss(match st.period { 1 => "Aujourd'hui", 7 => "7 jours", 30 => "30 jours", _ => "Toute période" }),
        show_session: own,
        session_empty: today.is_empty(),
        session: ms(session),
        session_strip: ms(today.iter().take(12).rev().map(|g| win(g.me())).collect()),
        session_note: ss(session_note),
        goal_text: ss(goal_text(settings.session_goal, tw)),
        versus_title: ss(if own { String::new() } else { format!("Toi face à {}", st.sum.game_name) }),
        versus: ms(if own {
            Vec::new()
        } else {
            let mut own_games: Vec<Game> = crate::store::keys_with("rw-games-")
                .iter()
                .filter_map(|k| crate::store::load::<Vec<Game>>(k))
                .max_by_key(|l| l.iter().map(|g| g.game_creation).max().unwrap_or(0))
                .unwrap_or_default();
            own_games.retain(|g| !is_remake(g));
            let mine: Vec<&Game> = own_games.iter().collect();
            let theirs: Vec<&Game> = games.iter().filter(|g| !is_remake(g)).collect();
            versus(&mine, &theirs, &|id| cname(id)).into_iter().map(|(label, me, them)| Versus { label: ss(label), mine: ss(me), theirs: ss(them) }).collect()
        }),
        pin_label: ss(match st.puuid.as_deref() {
            Some(puuid) => {
                let pinned: Vec<Value> = crate::store::load("rw-pinned").unwrap_or_default();
                if pinned.iter().any(|p| p["puuid"] == puuid) { "Retirer des épinglés" } else { "Épingler ce joueur" }
            }
            None => "",
        }),
        insights: ms(insights(
            &recent,
            chrono::Utc::now().timestamp_millis(),
            midnight,
            &|t| {
                use chrono::{Datelike, TimeZone, Timelike};
                chrono::Local.timestamp_millis_opt(t).single().map_or(Slot { hour: 0, weekday: 0 }, |d| Slot { hour: d.hour(), weekday: d.weekday().num_days_from_monday() })
            },
            &|id| cname(id),
        ).into_iter().map(ss).collect()),
        ranks: ms(ranks),
        has_lp: st.lp.len() > 1,
        lp: lp_data(&st.lp),
        recent_title: ss(format!("{plen} dernières parties")),
        wr: if plen > 0 { wins as f32 / plen as f32 } else { 0.0 },
        wr_text: ss(format!("{wins}V - {}D", plen - wins)),
        form_line: ss(form.streak.filter(|x| x.1 >= 2).map(|x| format!("{} {} de suite", x.1, if x.0 { "victoires" } else { "défaites" })).unwrap_or_default()),
        form_tone: good(form.streak.is_some_and(|x| x.0)),
        form_strip: ms(form.strip.clone()),
        kda_line: ss(match (form.kda_now, form.kda_before) {
            (Some(x), Some(y)) => format!("KDA {x:.2} sur les 5 dernières, {y:.2} sur les 5 d'avant"),
            _ => String::new(),
        }),
        perfs: ms(perfs),
        games_loading: false,
        empty_title: ss(if !shown.is_empty() { "" } else if games.is_empty() { "Aucune partie" } else { "Aucune partie avec ces filtres" }),
        empty_filtered: filtered && !games.is_empty(),
        more: st.sub == "apercu" && shown.len() > st.visible,
        queues: ms(by_queue(&shown)
            .iter()
            .map(|(q, t)| {
                let wr = t.wins as f64 / t.games as f64;
                Perf { icon: ss(""), title: ss(queue_label(*q)), sub: ss(format!("{:.1} KDA", t.kda / t.games as f64)), right: ss(pct0(wr)), right_sub: ss(format!("{} parties", t.games)), tone: good(wr >= 0.5), route: ss("") }
            })
            .collect()),
        mastery: ms(st
            .mastery
            .iter()
            .take(5)
            .map(|(c, level, points)| Perf { icon: ss(img_champ(&dd, *c)), title: ss(cname(*c)), sub: ss(format!("Niveau {level}")), right: ss(fr(*points)), right_sub: ss("points"), tone: 0, route: ss(format!("champion/{c}")) })
            .collect()),
        pool_cols: ms(vec![col("Champion", 0.0), col("Parties", 90.0), col("Winrate", 90.0), col("KDA", 90.0), col("CS/min", 90.0)]),
        pool_rows: ms(pool_rows),
    };
    a.data().set_profile(data);
    a.profile.session_copy = session_copy;
    if rebuild_rows {
        let st = &a.profile;
        let vis: Vec<Game> = shown.iter().take(st.visible).cloned().collect();
        let puuid = st.sum.puuid.clone();
        let rows: Vec<MatchRow> = vis.iter().map(|g| match_row(g, &puuid, &dd, &st.queues, st.details.get(&g.game_id).map(|x| &**x))).collect();
        let model = Rc::new(VecModel::from(rows));
        a.data().set_matches(ModelRc::from(model.clone()));
        a.profile.rows = Some(model);
        a.profile.shown = vis.iter().map(|g| g.game_id).collect();
        fetch_details(a, vis);
    }
}

fn fetch_details(a: &App, vis: Vec<Game>) {
    let gen = a.gen;
    let missing: Vec<i64> = vis.iter().map(|g| g.game_id).filter(|id| !a.profile.details.contains_key(id)).collect();
    spawn(async move {
        let gate = Arc::new(tokio::sync::Semaphore::new(5));
        for id in missing {
            let gate = gate.clone();
            spawn(async move {
                let Ok(_p) = gate.acquire().await else { return };
                if let Ok(full) = game_details(id).await {
                    on_ui(move |a| {
                        if a.gen != gen {
                            return;
                        }
                        a.profile.details.insert(id, full.clone());
                        let Some(i) = a.profile.shown.iter().position(|x| *x == id) else { return };
                        let Some(dd) = a.d.clone() else { return };
                        let st = &a.profile;
                        let Some(g) = st.games.as_ref().and_then(|l| l.iter().find(|g| g.game_id == id)) else { return };
                        let r = match_row(g, &st.sum.puuid, &dd, &st.queues, Some(&full));
                        if let Some(m) = &st.rows {
                            if i < m.row_count() {
                                m.set_row_data(i, r);
                            }
                        }
                    });
                }
            });
        }
    });
}

#[derive(Default)]
pub struct MatchState {
    pub game: Option<Arc<Game>>,
    pub frames: Vec<Value>,
    pub puuid: String,
    pub metric: String,
    pub queues: Arc<HashMap<i64, String>>,
    pub share: Option<ShareData>,
    pub share_win: Option<crate::ShareWindow>,
}

pub fn open_match(a: &mut App, game_id: i64, puuid: String) {
    a.match_.game = None;
    a.match_.frames.clear();
    a.match_.puuid = puuid;
    a.match_.metric = "cs".into();
    a.data().set_match(MatchPage { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let (dd, g, q) = tokio::join!(ddragon(), game_details(game_id), queue_names());
        let res = match (dd, g) {
            (Ok(d), Ok(g)) => Ok((d, g, q.unwrap_or_default())),
            (Err(e), _) | (_, Err(e)) => Err(e),
        };
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match res {
                Ok((d, g, q)) => {
                    a.d = Some(d);
                    a.match_.game = Some(g);
                    a.match_.queues = q;
                    render_match(a);
                }
                Err(e) => a.data().set_match(MatchPage { error: ss(crate::text::friendly_error(&e)), ..Default::default() }),
            }
        });
    });
    spawn(async move {
        if let Ok(t) = crate::net::get(&format!("/lol-match-history/v1/game-timelines/{game_id}")).await {
            let frames = t["frames"].as_array().cloned().unwrap_or_default();
            on_ui(move |a| {
                if a.gen == gen {
                    a.match_.frames = frames;
                    render_match(a);
                }
            });
        }
    });
}

fn frame_val(f: &Value, pid: i64, m: &str) -> i64 {
    let x = &f["participantFrames"][pid.to_string()];
    let i = |k: &str| x[k].as_i64().unwrap_or(0);
    match m {
        "cs" => i("minionsKilled") + i("jungleMinionsKilled"),
        "gold" => i("totalGold"),
        _ => i("xp"),
    }
}

pub fn lane_opponent<'a>(g: &'a Game, p: &Participant, frames: &[Value]) -> Option<&'a Participant> {
    let smite = |x: &Participant| x.spell1_id == SMITE || x.spell2_id == SMITE;
    let enemies: Vec<&Participant> = g.participants.iter().filter(|x| x.team_id != p.team_id).collect();
    if smite(p) {
        return enemies.into_iter().find(|x| smite(x));
    }
    let f = frames.get(10.min(frames.len().saturating_sub(1)))?;
    let cs_at = |x: &Participant| f["participantFrames"][x.participant_id.to_string()]["minionsKilled"].as_i64().unwrap_or(0);
    let by_cs = |list: Vec<&'a Participant>| {
        let mut l: Vec<&'a Participant> = list.into_iter().filter(|x| !smite(x)).collect();
        l.sort_by_key(|x| std::cmp::Reverse(cs_at(x)));
        l
    };
    let mine = by_cs(g.participants.iter().filter(|x| x.team_id == p.team_id).collect());
    let idx = mine.iter().position(|x| x.participant_id == p.participant_id)?;
    by_cs(enemies).get(idx).copied()
}

pub fn score(g: &Game, p: &Participant, f: &dyn Fn(&Participant) -> f64) -> i64 {
    let v = f(p);
    (100.0 * g.participants.iter().filter(|x| f(x) < v).count() as f64 / (g.participants.len().max(2) - 1) as f64).round() as i64
}

fn grade(s: i64) -> &'static str {
    if s >= 80 {
        "Excellent"
    } else if s >= 60 {
        "Bon"
    } else if s >= 40 {
        "Moyen"
    } else {
        "À travailler"
    }
}

fn hex(v: u32) -> Color {
    Color::from_argb_encoded(0xff000000 | v)
}

pub fn scoreboard(g: &Game, d: &DD, puuid: &str) -> (Vec<Col>, Vec<TRow>) {
    let cols = vec![col("", 0.0), col("K / D / A", 120.0), col("KP", 70.0), col("CS", 110.0), col("Dégâts", 170.0), col("Objets", 240.0)];
    let min = g.game_duration as f64 / 60.0;
    let max_dmg = g.participants.iter().map(|x| n(&x.stats, "totalDamageDealtToChampions")).fold(1.0, f64::max);
    let my_team = g.find(puuid).map(|x| x.team_id).unwrap_or(100);
    let mut rows = vec![];
    for t in [my_team, if my_team == 100 { 200 } else { 100 }] {
        let list: Vec<&Participant> = g.participants.iter().filter(|x| x.team_id == t).collect();
        let won = list.first().is_some_and(|p| win(p));
        let sum = |k: &str| list.iter().map(|x| n(&x.stats, k)).sum::<f64>() as i64;
        let (k, de, asst) = (sum("kills"), sum("deaths"), sum("assists"));
        let team_kills = k.max(1) as f64;
        let mut head = toned(if won { "Victoire" } else { "Défaite" }, good(won));
        head.sub = ss(if t == my_team { "Ton équipe" } else { "Adversaires" });
        let mut hr = row(vec![head, toned(format!("{k} / {de} / {asst}"), 4), txt(""), txt(""), txt(""), txt("")], "");
        hr.header = true;
        rows.push(hr);
        for x in list {
            let who = g.who(x.participant_id);
            let dmg = n(&x.stats, "totalDamageDealtToChampions");
            let mut c0 = iconed(who.map(|w| w.game_name.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "?".into()), img_champ(d, x.champion_id));
            c0.sub = ss(format!("#{}", who.map(|w| w.tag_line.clone()).unwrap_or_default()));
            let mut c1 = txt(format!("{} / {} / {}", n(&x.stats, "kills"), n(&x.stats, "deaths"), n(&x.stats, "assists")));
            c1.sub = ss(format!("{:.2} KDA", kda(&x.stats)));
            let mut c3 = txt((cs(&x.stats) as i64).to_string());
            c3.sub = ss(format!("{:.1}/min", cs(&x.stats) / min.max(0.01)));
            let mut c4 = txt(fr(dmg as i64));
            c4.bar = (dmg / max_dmg) as f32;
            c4.use_color = true;
            c4.color = hex(0xff6b7d);
            let mut c5 = txt("");
            c5.kind = 2;
            c5.icons = ms((0..7).map(|k| n(&x.stats, &format!("item{k}")) as i64).map(|id| ss(if id != 0 { img_item(d, id) } else { String::new() })).collect());
            let pu = who.map(|w| w.puuid.clone()).unwrap_or_default();
            let mut r = row(vec![c0, c1, txt(format!("{}%", ((n(&x.stats, "kills") + n(&x.stats, "assists")) / team_kills * 100.0).round())), c3, c4, c5], if pu.is_empty() { String::new() } else { format!("profil/{pu}") });
            r.me = pu == puuid;
            rows.push(r);
        }
    }
    (cols, rows)
}

fn render_match(a: &mut App) {
    let Some(d) = a.d.clone() else { return };
    let Some(g) = a.match_.game.clone() else { return };
    let puuid = a.match_.puuid.clone();
    let Some(p) = g.find(&puuid) else {
        a.data().set_match(MatchPage { error: ss("Joueur absent de cette partie."), ..Default::default() });
        return;
    };
    let frames = &a.match_.frames;
    let s = &p.stats;
    let min = g.game_duration as f64 / 60.0;
    let opp = if frames.is_empty() { None } else { lane_opponent(&g, p, frames) };
    let opp_name = opp.and_then(|o| g.who(o.participant_id)).map(|w| w.game_name.clone()).filter(|x| !x.is_empty());
    let at = 14.min(frames.len().saturating_sub(1));
    let diff = |m: &str| match opp {
        Some(o) if at > 0 => frame_val(&frames[at], p.participant_id, m) - frame_val(&frames[at], o.participant_id, m),
        _ => 0,
    };
    let team_kills: f64 = g.participants.iter().filter(|x| x.team_id == p.team_id).map(|x| n(&x.stats, "kills")).sum();
    let kp = |x: &Participant| (n(&x.stats, "kills") + n(&x.stats, "assists")) / g.participants.iter().filter(|y| y.team_id == x.team_id).map(|y| n(&y.stats, "kills")).sum::<f64>().max(1.0);
    let first = if g.map_id == 12 { ("Résistance", score(&g, p, &|x| n(&x.stats, "totalDamageTaken") + n(&x.stats, "damageSelfMitigated"))) } else { ("Farming", score(&g, p, &|x| cs(&x.stats))) };
    let mut scores: Vec<(&str, i64)> = vec![first, ("Combat", score(&g, p, &|x| kda(&x.stats) + kp(x) * 3.0 + n(&x.stats, "totalDamageDealtToChampions") / 10000.0))];
    if g.map_id == 11 {
        scores.push(("Vision", score(&g, p, &|x| n(&x.stats, "visionScore"))));
    }
    scores.push(("Objectifs", score(&g, p, &|x| n(&x.stats, "damageDealtToObjectives"))));
    let best = scores.iter().max_by_key(|x| x.1).map(|x| x.0.to_lowercase()).unwrap_or_default();
    let (cs_d, gold_d) = (diff("cs"), diff("gold"));
    let summary = match opp {
        Some(o) if at >= 10 => format!("À {at} min, tu avais {} CS et {} or {} sur {} ({}).", cs_d.abs(), fr(gold_d.abs()), if gold_d >= 0 { "d'avance" } else { "de retard" }, opp_name.clone().unwrap_or_else(|| "ton adversaire".into()), d.champs.get(&o.champion_id).map(|c| c.name.clone()).unwrap_or_default()),
        _ if g.map_id == 12 => format!("{}% de participation aux kills, {} dégâts subis et {} s de contrôle sur les adversaires.", ((n(s, "kills") + n(s, "assists")) / team_kills.max(1.0) * 100.0).round(), fr(n(s, "totalDamageTaken") as i64), n(s, "timeCCingOthers")),
        _ => format!("{:.1} CS/min, {}% de participation aux kills.", cs(s) / min.max(0.01), ((n(s, "kills") + n(s, "assists")) / team_kills.max(1.0) * 100.0).round()),
    };
    let remake = g.game_duration < 300;
    let qlabel = a.match_.queues.get(&g.queue_id).cloned().unwrap_or_else(|| format!("File {}", g.queue_id));
    let bd = badges(&g, &puuid);
    let splash = d.champs.get(&p.champion_id).map(|c| img_splash(&c.id)).unwrap_or_default();
    let hero = crate::history::highlight(&g, p);
    let kda_text = format!("{} / {} / {}", n(s, "kills"), n(s, "deaths"), n(s, "assists"));
    let side: Vec<(&str, String, &str)> = if g.map_id == 12 {
        vec![
            ("", kda_text, "KDA"),
            ("totalDamageDealtToChampions", fr(n(s, "totalDamageDealtToChampions") as i64), "Dégâts aux champions"),
            ("totalDamageTaken", fr(n(s, "totalDamageTaken") as i64), "Dégâts subis"),
            ("timeCCingOthers", format!("{} s", n(s, "timeCCingOthers")), "Contrôle des foules"),
            ("totalHeal", fr(n(s, "totalHeal") as i64), "Soins"),
            ("goldEarned", fr(n(s, "goldEarned") as i64), "Or gagné"),
        ]
    } else {
        vec![
            ("", kda_text, "KDA"),
            ("totalDamageDealtToChampions", fr(n(s, "totalDamageDealtToChampions") as i64), "Dégâts aux champions"),
            ("largestCriticalStrike", fr(n(s, "largestCriticalStrike") as i64), "Plus gros critique"),
            ("goldEarned", fr(n(s, "goldEarned") as i64), "Or gagné"),
            ("", fr(cs(s) as i64), "CS"),
            ("totalHeal", fr(n(s, "totalHeal") as i64), "Soins"),
        ]
    };
    let share = ShareData {
        label: ss(&qlabel),
        result: ss(if remake { "REMAKE" } else if win(p) { "VICTOIRE" } else { "DÉFAITE" }),
        tone: if remake { 0 } else { good(win(p)) },
        mvp: bd.contains(&"MVP"),
        duration: ss(mmss(g.game_duration as f64)),
        damage: ss(fr(hero.2 as i64)),
        hero: ss(hero.1),
        stats: ms(side.into_iter().filter(|(key, _, _)| *key != hero.0).take(5).map(|(_, value, label)| stat(value, label, 0)).collect()),
        runes: ms([n(s, "perk0") as i64, n(s, "perkSubStyle") as i64].iter().filter(|id| d.runes.contains_key(id)).map(|id| ss(img_rune(&d, *id))).collect()),
        items: ms((0..7).map(|k| n(s, &format!("item{k}")) as i64).filter(|x| *x != 0).map(|id| ss(img_item(&d, id))).collect()),
        splash: ss(splash),
    };
    a.match_.share = Some(share.clone());
    let chart = build_chart(a, &g, p, opp, &d);
    let objs: Vec<ObjTeam> = {
        let mut teams: Vec<&Value> = g.teams.iter().collect();
        teams.sort_by_key(|t| t["teamId"].as_i64() != Some(p.team_id));
        teams
            .iter()
            .map(|t| {
                let tid = t["teamId"].as_i64().unwrap_or(0);
                let kills: f64 = g.participants.iter().filter(|x| x.team_id == tid).map(|x| n(&x.stats, "kills")).sum();
                let v = |k: &str| t[k].as_i64().unwrap_or(0).to_string();
                let mut stats = vec![stat((kills as i64).to_string(), "Kills", 0), stat(v("towerKills"), "Tours", 0)];
                if g.map_id == 11 {
                    stats.extend([stat(v("dragonKills"), "Dragons", 0), stat(v("hordeKills"), "Larves", 0), stat(v("riftHeraldKills"), "Héraut", 0), stat(v("baronKills"), "Nashor", 0)]);
                }
                ObjTeam { title: ss(if tid == p.team_id { "Ton équipe" } else { "Adversaires" }), mine: tid == p.team_id, stats: ms(stats) }
            })
            .collect()
    };
    let (cols, rows) = scoreboard(&g, &d, &puuid);
    let msg = a.data().get_match().share_msg;
    let page = MatchPage {
        loading: false,
        error: ss(""),
        result: if remake { 2 } else if win(p) { 0 } else { 1 },
        title: ss(if remake { "Remake" } else if win(p) { "Victoire" } else { "Défaite" }),
        sub: ss(format!("{qlabel}, {}, le {}", mmss(g.game_duration as f64), date_long_time(g.game_creation))),
        champ: ss(img_champ(&d, p.champion_id)),
        badges: badge_list(&bd, 8),
        build_label: ss(format!("Build {}", d.champs.get(&p.champion_id).map(|c| c.name.clone()).unwrap_or_default())),
        build_route: ss(format!("champion/{}", p.champion_id)),
        note: ss(crate::store::get(&format!("rw-note-{}", g.game_id)).unwrap_or_default()),
        share,
        share_msg: msg,
        scores: ms(scores
            .iter()
            .map(|(k, v)| Score {
                k: ss(k),
                v: ss(v.to_string()),
                value: *v as f32 / 100.0,
                grade: ss(grade(*v)),
                color: if *v >= 80 {
                    hex(0x3ddc97)
                } else if *v >= 60 {
                    hex(0xe3b34c)
                } else if *v >= 40 {
                    hex(0x8fa6ff)
                } else {
                    hex(0xff6b7d)
                },
            })
            .collect()),
        best: ss(best),
        summary: ss(summary),
        has_chart: frames.len() > 1,
        chart,
        objs: ms(objs),
        cols: ms(cols),
        rows: ms(rows),
    };
    a.data().set_match(page);
}

fn build_chart(a: &App, g: &Game, p: &Participant, opp: Option<&Participant>, d: &DD) -> ChartData {
    let frames = &a.match_.frames;
    let m = a.match_.metric.as_str();
    let len = frames.len();
    let val = |pid: i64| frames.iter().map(|f| frame_val(f, pid, m)).collect::<Vec<_>>();
    let mut sets = vec![("Toi".to_string(), hex(0xc08a1e), val(p.participant_id))];
    if let Some(o) = opp {
        sets.push((d.champs.get(&o.champion_id).map(|c| c.name.clone()).unwrap_or_else(|| "Adversaire".into()), hex(0x7189f0), val(o.participant_id)));
    }
    let max = sets.iter().flat_map(|s| s.2.iter().copied()).max().unwrap_or(1).max(1) as f64;
    let fmt = |v: i64| if m == "gold" { fr(v) } else { v.to_string() };
    let last = len.max(2) - 1;
    let mut ends: Vec<f32> = sets.iter().map(|s| 1.0 - (*s.2.last().unwrap_or(&0) as f64 / max) as f32).collect();
    if ends.len() == 2 && (ends[0] - ends[1]).abs() < 0.07 {
        if ends[0] <= ends[1] {
            ends[1] = ends[0] + 0.07;
        } else {
            ends[0] = ends[1] + 0.07;
        }
    }
    let series = sets
        .iter()
        .zip(ends.iter())
        .map(|(s, e)| {
            let ys: Vec<f32> = s.2.iter().map(|v| 1.0 - (*v as f64 / max) as f32).collect();
            let path = s.2.iter().enumerate().map(|(i, v)| format!("{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, i as f64 / last as f64 * 1000.0, (1.0 - *v as f64 / max) * 1000.0)).collect::<Vec<_>>().join(" ");
            ChartSeries { name: ss(&s.0), color: s.1, path: ss(path), end_y: *e, values: ms(s.2.iter().map(|v| ss(fmt(*v))).collect()), ys: ms(ys) }
        })
        .collect();
    let _ = g;
    ChartData {
        series: ms(series),
        yticks: ms([0.0, 0.5, 1.0].iter().map(|k| AxisLabel { pos: *k as f32, text: ss(fmt((max * k).round() as i64)) }).collect()),
        xticks: ms((0..len).filter(|i| i % 5 == 0).map(|i| AxisLabel { pos: i as f32 / last as f32, text: ss(i.to_string()) }).collect()),
        len: len as i32,
        mark14: if len > 14 { 14.0 / last as f32 } else { -1.0 },
        tabs: ms([("cs", "CS"), ("gold", "Or"), ("xp", "XP")].iter().map(|(k, l)| opt(format!("metric:{k}"), l, m == *k)).collect()),
        unit: ss(match m {
            "gold" => "or",
            "cs" => "CS",
            _ => "XP",
        }),
    }
}

fn set_share_msg(a: &App, msg: &str) {
    let mut m = a.data().get_match();
    m.share_msg = ss(msg);
    a.data().set_match(m);
}

fn share_image(a: &mut App) {
    use slint::ComponentHandle;
    use slint::winit_030::{winit, WinitWindowAccessor};
    let Some(share) = a.match_.share.clone() else { return };
    set_share_msg(a, "…");
    let win = match crate::ShareWindow::new() {
        Ok(w) => w,
        Err(e) => {
            set_share_msg(a, &format!("Copie impossible : {e}"));
            return;
        }
    };
    win.global::<crate::Api>().on_img(|u, _| crate::images::img(&u));
    win.global::<crate::Theme>().set_game(ss(&a.game));
    win.set_share(share);
    crate::OVERLAY_NEXT.store(true, std::sync::atomic::Ordering::SeqCst);
    win.window().set_position(slint::PhysicalPosition::new(-20000, -20000));
    let _ = win.show();
    crate::OVERLAY_NEXT.store(false, std::sync::atomic::Ordering::SeqCst);
    win.window().with_winit_window(|w: &winit::window::Window| w.set_cursor_hittest(false).ok());
    a.match_.share_win = Some(win);
    slint::Timer::single_shot(std::time::Duration::from_millis(900), move || {
        with_app(|a| {
            let Some(win) = a.match_.share_win.take() else { return };
            let done = win.window().take_snapshot().map_err(|e| e.to_string()).and_then(|b| copy_rgba(b.width() as usize, b.height() as usize, b.as_bytes()));
            let _ = win.hide();
            match done {
                Ok(()) => set_share_msg(a, "Image copiée, colle-la sur Discord"),
                Err(e) => set_share_msg(a, &format!("Copie impossible : {e}")),
            }
        });
    });
}

pub fn copy_rgba(w: usize, h: usize, rgba: &[u8]) -> Result<(), String> {
    if let Ok(dir) = std::env::var("KAIRO_SNAP") {
        let _ = image::save_buffer(format!("{dir}/share.png"), rgba, w as u32, h as u32, image::ColorType::Rgba8);
        return Ok(());
    }
    let mut c = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    c.set_image(arboard::ImageData { width: w, height: h, bytes: rgba.into() }).map_err(|e| e.to_string())
}

pub fn copy_text(a: &mut App, text: &str, label: &str) {
    let done = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string()));
    match done {
        Ok(()) => a.toast(format!("{label} copié")),
        Err(_) => a.toast("Copie impossible"),
    }
}

pub fn action(a: &mut App, key: &str) -> bool {
    if let Some(k) = key.strip_prefix("sub:") {
        a.profile.sub = k.into();
        render(a, true);
    } else if let Some(k) = key.strip_prefix("q:") {
        a.profile.filter.queue = k.parse().unwrap_or(0);
        a.profile.visible = PAGE;
        render(a, true);
    } else if let Some(k) = key.strip_prefix("c:") {
        a.profile.filter.champ = k.parse().unwrap_or(0);
        a.profile.visible = PAGE;
        render(a, true);
    } else if let Some(k) = key.strip_prefix("p:") {
        let days: i64 = k.parse().unwrap_or(0);
        a.profile.period = days;
        a.profile.filter.since = match days {
            0 => 0,
            1 => midnight_ms(),
            n => chrono::Utc::now().timestamp_millis() - n * 86_400_000,
        };
        a.profile.visible = PAGE;
        render(a, true);
    } else if let Some(k) = key.strip_prefix("r:") {
        a.profile.filter.result = k.into();
        a.profile.visible = PAGE;
        render(a, true);
    } else if let Some(k) = key.strip_prefix("metric:") {
        a.match_.metric = k.into();
        render_match(a);
    } else {
        match key {
            "clear-filters" => {
                a.profile.filter = HistoryFilter::default();
                render(a, true);
            }
            "more" => {
                a.profile.visible += PAGE;
                render(a, true);
            }
            other if other.starts_with("match-note:") => {
                if let Some(id) = a.match_.game.as_ref().map(|g| g.game_id) {
                    let text = other["match-note:".len()..].trim().chars().take(300).collect::<String>();
                    let key = format!("rw-note-{id}");
                    if text.is_empty() { crate::store::remove(&[key]) } else { crate::store::set(&key, text) }
                }
            }
            "pin-toggle" => {
                if let Some(puuid) = a.profile.puuid.clone() {
                    let name = format!("{}#{}", a.profile.sum.game_name, a.profile.sum.tag_line);
                    let mut list: Vec<Value> = crate::store::load("rw-pinned").unwrap_or_default();
                    let on = toggle_pin(&mut list, &puuid, &name);
                    crate::store::save("rw-pinned", &list);
                    a.toast(if on { format!("{name} épinglé : il apparaît en tête de la recherche") } else { format!("{name} retiré des épinglés") });
                    render(a, false);
                }
            }
            "mini-toggle" => {
                let on = toggle_mini();
                a.toast(if on { "Mini fenêtre ouverte, toujours devant les autres" } else { "Mini fenêtre fermée" });
                render(a, false);
            }
            "goal-next" => {
                let mut s = crate::store::settings();
                s.session_goal = next_goal(s.session_goal);
                crate::store::save_settings(&s);
                render(a, false);
            }
            "copy-session" => {
                let t = a.profile.session_copy.clone();
                copy_text(a, &t, "Résumé");
            }
            "share" => share_image(a),
            "first-match" if std::env::var("KAIRO_SNAP").is_ok() => {
                if let Some(r) = a.profile.rows.as_ref().and_then(|m| m.row_data(0)) {
                    let route = r.route.to_string();
                    a.go(&route);
                }
            }
            "copy-id" if a.game == "lol" => {
                let t = format!("{}#{}", a.profile.sum.game_name, a.profile.sum.tag_line);
                copy_text(a, &t, "Riot ID");
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game() -> Game {
        let part = |id: i64, team: i64, spell: i64, k: i64, dmg: i64| json!({"participantId": id, "teamId": team, "championId": id, "spell1Id": spell, "spell2Id": 4, "stats": {"win": team == 100, "kills": k, "deaths": 1, "assists": 0, "totalDamageDealtToChampions": dmg, "goldEarned": dmg, "visionScore": 1, "totalMinionsKilled": 10 * id, "largestMultiKill": k}});
        serde_json::from_value(json!({"gameId": 1, "gameDuration": 1800, "mapId": 11,
            "participants": [part(1, 100, 14, 5, 9000), part(2, 100, 11, 1, 100), part(3, 200, 14, 0, 50), part(4, 200, 11, 0, 60)],
            "participantIdentities": [{"participantId": 1, "player": {"puuid": "a", "gameName": "Moi"}}, {"participantId": 3, "player": {"puuid": "b"}}, {"participantId": 2, "player": {"puuid": "c"}}, {"participantId": 4, "player": {"puuid": "d"}}]})).unwrap()
    }

    #[test]
    fn badges_and_opponent() {
        let g = game();
        let b = badges(&g, "a");
        assert_eq!(b[0], "MVP");
        assert!(b.contains(&"Top dégâts") && b.contains(&"Penta"));
        assert!(badges(&g, "zz").is_empty());
        let frames = vec![json!({"participantFrames": {"1": {"minionsKilled": 5}, "3": {"minionsKilled": 7}}}); 11];
        let p = g.find("a").unwrap();
        assert_eq!(lane_opponent(&g, p, &frames).unwrap().participant_id, 3);
        let j = g.find("c").unwrap();
        assert_eq!(lane_opponent(&g, j, &frames).unwrap().participant_id, 4);
        assert_eq!(score(&g, p, &|x| n(&x.stats, "kills")), 100);
    }

    #[test]
    fn lp() {
        let d = lp_data(&[LpPt { t: 0, v: 100 }, LpPt { t: 864_000_00 * 3, v: 150 }]);
        assert_eq!(d.total.as_str(), "+50 LP");
        assert_eq!(d.span.as_str(), "sur 3 jours");
    }
}
