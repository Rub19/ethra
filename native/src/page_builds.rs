use crate::build_data::*;
use crate::history::{cs, kda, win, Game};
use crate::jungle;
use crate::lol::*;
use crate::net;
use crate::opgg::*;
use crate::page_profile::{col, iconed, row, sort_col, toned, txt};
use crate::store::{self, save_settings, settings};
use crate::text::*;
use crate::{good, ms, on_ui, opt, spawn, ss, stat, App};
use crate::{BuildsData, Cell, ChampionsData, Chip, CounterCell, DmgBar, GridChamp, IOpt, IOptRow, ItemStep, ItemsPlain, JungleData, JungleStep, LiveData, Opt, PlayerCard, RuneCell, RuneSlot, RuneTree, SkillCell, SkillRow, SkillVariant, TierData};
use slint::Color;
use std::collections::HashMap;

pub fn fav_champs() -> Vec<i64> {
    store::load("rw-fav-champs").unwrap_or_default()
}

fn toggle_fav(id: i64) {
    let mut f = fav_champs();
    if f.contains(&id) {
        f.retain(|x| *x != id);
    } else {
        f.insert(0, id);
    }
    store::save("rw-fav-champs", &f);
}

#[derive(Default)]
pub struct TierState {
    pub rows: Option<Vec<MetaRow>>,
    pub err: String,
    pub pos: String,
    pub q: String,
    pub fav_only: bool,
    pub sort: String,
    pub desc: bool,
}

pub fn open_tier(a: &mut App) {
    a.tier = TierState { pos: "all".into(), sort: "tier".into(), ..Default::default() };
    a.data().set_tier_q(ss(""));
    a.data().set_tier(TierData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let r: Result<_, String> = async {
            let d = ddragon().await?;
            let rows = meta(&d).await?;
            Ok((d, rows))
        }
        .await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match r {
                Ok((d, rows)) => {
                    a.d = Some(d);
                    a.tier.rows = Some(rows);
                    render_tier(a);
                }
                Err(e) => a.data().set_tier(TierData { error: ss(crate::text::friendly_error(&e)), ..Default::default() }),
            }
        });
    });
}

fn tier_cell(t: i64) -> Cell {
    let mut c = txt(tier_label(t));
    c.kind = 1;
    c.tone = t.min(4) as i32;
    c
}

fn render_tier(a: &mut App) {
    let Some(d) = a.d.clone() else { return };
    let st = &a.tier;
    let Some(rows) = &st.rows else { return };
    let favs = fav_champs();
    let q = fold(st.q.trim());
    let mut list: Vec<&MetaRow> = rows.iter().filter(|r| (!st.fav_only || favs.contains(&r.champ)) && (st.pos == "all" || r.position == st.pos) && fold(&r.name).contains(&q)).collect();
    list.sort_by(|x, y| {
        let c = match st.sort.as_str() {
            "tier" => x.tier.cmp(&y.tier).then(x.rank.cmp(&y.rank)),
            "winRate" => x.win_rate.total_cmp(&y.win_rate),
            "pickRate" => x.pick_rate.total_cmp(&y.pick_rate),
            "banRate" => x.ban_rate.total_cmp(&y.ban_rate),
            "play" => x.play.cmp(&y.play),
            _ => x.kda.total_cmp(&y.kda),
        };
        if st.desc {
            c.reverse()
        } else {
            c
        }
    });
    let moves = rows.iter().any(|r| r.rank_prev != 0 && r.rank_prev != r.rank);
    let th = |k: &str, l: &str, px: f32| sort_col(l, px, format!("tsort:{k}"), st.sort == k, st.desc);
    let mut cols = vec![col("#", 44.0), col("Rôle", 110.0), col("Champion", 0.0), th("tier", "Tier", 80.0), th("winRate", "Winrate", 90.0), th("pickRate", "Pick", 80.0), th("banRate", "Ban", 80.0), th("kda", "KDA", 70.0), th("play", "Parties", 100.0)];
    if moves {
        cols.push(col("Évolution", 90.0));
    }
    let out = list
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let name = d.champs.get(&r.champ).map(|c| c.name.clone()).unwrap_or_else(|| r.name.clone());
            let mut role = txt(pos_label(&r.position));
            role.kind = 4;
            role.icon = ss(img_role(&r.position));
            let mut star = txt("");
            star.kind = 3;
            star.on = favs.contains(&r.champ);
            star.key = ss(format!("fav:{}", r.champ));
            let delta = if r.rank_prev != 0 { r.rank_prev - r.rank } else { 0 };
            let mut cells = vec![toned((i + 1).to_string(), 4), role, iconed(name, img_champ(&d, r.champ)), tier_cell(r.tier), toned(pct1(r.win_rate), good(r.win_rate >= 0.5)), txt(pct1(r.pick_rate)), txt(pct1(r.ban_rate)), txt(format!("{:.2}", r.kda)), txt(fr(r.play))];
            cells.insert(3, star);
            if moves {
                cells.push(if delta != 0 {
                    let mut c = toned(delta.abs().to_string(), if delta > 0 { 1 } else { 2 });
                    c.kind = 6;
                    c
                } else {
                    toned("–", 4)
                });
            }
            let route = if r.champ != 0 { format!("champion/{}/{}", r.champ, r.position) } else { String::new() };
            row(cells, route)
        })
        .collect();
    cols.insert(3, col("", 42.0));
    let mut roles: Vec<Opt> = vec![opt("tpos:all", "Tous", st.pos == "all")];
    for (k, l) in POS_LABEL {
        let mut o = opt(format!("tpos:{k}"), l, st.pos == k);
        o.icon = ss(img_role(k));
        roles.push(o);
    }
    let empty_title = if !list.is_empty() {
        ""
    } else if st.fav_only && favs.is_empty() {
        "Aucun favori"
    } else {
        "Aucun champion"
    };
    let empty_text = if st.fav_only && favs.is_empty() { "Clique sur l'étoile d'un champion pour le retrouver ici et dans la recherche." } else { "Change de rôle ou de recherche." };
    let fav_label = if favs.is_empty() { "Favoris".to_string() } else { format!("Favoris ({})", favs.len()) };
    a.data().set_tier(TierData {
        loading: false,
        error: ss(""),
        q_opts: ms(vec![]),
        fav: ms(vec![opt("tfav", fav_label, st.fav_only)]),
        roles: ms(roles),
        cols: ms(cols),
        rows: ms(out),
        empty_title: ss(empty_title),
        empty_text: ss(empty_text),
    });
}

const CLASSES: [(&str, &str); 7] = [("", "Tous"), ("Assassin", "Assassins"), ("Fighter", "Combattants"), ("Mage", "Mages"), ("Marksman", "Tireurs"), ("Support", "Supports"), ("Tank", "Tanks")];

#[derive(Default)]
pub struct ChampsState {
    pub games: Option<Vec<Game>>,
    pub q: String,
    pub cls: String,
    pub played: bool,
    pub sort: String,
    pub desc: bool,
}

#[derive(Clone, Default)]
struct CRow {
    id: i64,
    name: String,
    g: f64,
    w: f64,
    k: f64,
    cs: f64,
    last: i64,
}

pub fn open_champs(a: &mut App) {
    a.champs = ChampsState { played: true, sort: "g".into(), desc: true, ..Default::default() };
    a.data().set_champ_q(ss(""));
    a.data().set_champions(ChampionsData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let d = ddragon().await;
        on_ui(move |a| {
            if let Ok(d) = d {
                a.d = Some(d);
                if a.gen == gen {
                    render_champs(a);
                }
            }
        });
        let games = match me().await {
            Ok(s) => history(&s.puuid, 100).await.unwrap_or_default(),
            Err(_) => vec![],
        };
        on_ui(move |a| {
            if a.gen == gen {
                a.champs.games = Some(games);
                render_champs(a);
            }
        });
    });
}

fn render_champs(a: &mut App) {
    let Some(d) = a.d.clone() else { return };
    let st = &a.champs;
    let mut stats: HashMap<i64, CRow> = HashMap::new();
    for g in st.games.iter().flatten() {
        let p = g.me();
        let r = stats.entry(p.champion_id).or_insert_with(|| CRow { id: p.champion_id, name: d.champs.get(&p.champion_id).map(|c| c.name.clone()).unwrap_or_else(|| "?".into()), ..Default::default() });
        r.g += 1.0;
        r.w += if win(p) { 1.0 } else { 0.0 };
        r.k += kda(&p.stats);
        r.cs += cs(&p.stats) / (g.game_duration as f64 / 60.0).max(0.01);
        r.last = r.last.max(g.game_creation);
    }
    let show_played = st.played && !stats.is_empty();
    let q = fold(st.q.trim());
    let mut rows: Vec<CRow> = d
        .champs
        .iter()
        .map(|(id, c)| stats.get(id).cloned().unwrap_or(CRow { id: *id, name: c.name.clone(), ..Default::default() }))
        .filter(|r| (!show_played || r.g > 0.0) && (st.cls.is_empty() || d.champs[&r.id].tags.contains(&st.cls)) && fold(&r.name).contains(&q))
        .collect();
    if show_played {
        rows.sort_by(|x, y| {
            let val = |r: &CRow| match st.sort.as_str() {
                "wr" => r.w / r.g.max(1.0),
                "k" => r.k / r.g.max(1.0),
                "cs" => r.cs / r.g.max(1.0),
                "last" => r.last as f64,
                _ => r.g,
            };
            let c = if st.sort == "name" { fold(&x.name).cmp(&fold(&y.name)) } else { val(x).total_cmp(&val(y)) };
            if st.desc {
                c.reverse()
            } else {
                c
            }
        });
    } else {
        rows.sort_by_key(|r| fold(&r.name));
    }
    let th = |k: &str, l: &str, px: f32| sort_col(l, px, format!("csort:{k}"), st.sort == k, st.desc);
    let cols = vec![col("#", 44.0), th("name", "Champion", 0.0), col("Classe", 120.0), th("g", "Parties", 100.0), th("wr", "Winrate", 90.0), th("k", "KDA", 80.0), th("cs", "CS/min", 80.0), th("last", "Dernière", 130.0)];
    let maxg = rows.iter().map(|r| r.g).fold(1.0, f64::max);
    let trows = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let class = CLASSES.iter().find(|(k, _)| Some(*k) == d.champs[&r.id].tags.first().map(String::as_str)).map(|x| x.1).unwrap_or("");
            let mut g = txt((r.g as i64).to_string());
            g.bar = (r.g / maxg) as f32;
            row(
                vec![toned((i + 1).to_string(), 4), iconed(&r.name, img_champ(&d, r.id)), toned(class, 4), g, toned(pct0(r.w / r.g.max(1.0)), good(r.w / r.g.max(1.0) >= 0.5)), txt(format!("{:.2}", r.k / r.g.max(1.0))), txt(format!("{:.1}", r.cs / r.g.max(1.0))), toned(ago(r.last), 4)],
                format!("champion/{}", r.id),
            )
        })
        .collect();
    let grid = rows
        .iter()
        .map(|r| GridChamp { icon: ss(img_champ(&d, r.id)), name: ss(&r.name), sub: ss(if r.g > 0.0 { format!("{} sur {} parties", pct0(r.w / r.g), r.g) } else { String::new() }), route: ss(format!("champion/{}", r.id)) })
        .collect();
    let text = match &st.games {
        None => "Analyse de tes 100 dernières parties…".to_string(),
        Some(g) if !g.is_empty() => format!("Tes stats sur tes {} dernières parties. Clique sur un champion pour voir et importer son build.", g.len()),
        _ => "Clique sur un champion pour voir son build méta. Lance le client League pour ajouter tes stats personnelles.".to_string(),
    };
    a.data().set_champions(ChampionsData {
        loading: false,
        text: ss(text),
        classes: ms(CLASSES.iter().map(|(k, l)| opt(format!("ccls:{k}"), l, st.cls == *k)).collect()),
        played: ms(if stats.is_empty() { vec![] } else { vec![opt("cplayed:1", "Joués", st.played), opt("cplayed:0", "Tous", !st.played)] }),
        show_table: show_played,
        cols: ms(cols),
        rows: ms(trows),
        grid: ms(grid),
    });
}

#[derive(Default)]
pub struct BuildsState {
    pub champ: i64,
    pub role: String,
    pub src: String,
    pub mode: String,
    pub rank: String,
    pub b: Option<Build>,
    pub loaded: bool,
    pub msg: String,
    pub sb: usize,
    pub compact: bool,
    pub jteam: String,
    pub jstart: String,
    pub gen: u64,
}

pub fn open_builds(a: &mut App, champ: i64, role: String, compact: bool, mode: &str) {
    let role = opgg_pos(&role).map(String::from).unwrap_or_else(|| if pos_label(&role).is_empty() { String::new() } else { role.clone() });
    a.builds = BuildsState { champ, role, src: "opgg".into(), mode: mode.into(), rank: settings().rank_filter, compact, jteam: "blue".into(), jstart: if settings().jungle_route == "blue" { "blue".into() } else { "red".into() }, ..Default::default() };
    load_builds(a);
}

pub fn load_builds(a: &mut App) {
    a.builds.gen += 1;
    a.builds.loaded = false;
    a.builds.msg.clear();
    a.builds.sb = 0;
    render_builds(a);
    let gen = (a.gen, a.builds.gen);
    let (champ, src, mode, role, rank) = (a.builds.champ, a.builds.src.clone(), a.builds.mode.clone(), a.builds.role.clone(), a.builds.rank.clone());
    spawn(async move {
        let Ok(d) = ddragon().await else { return };
        let map = if mode == "aram" { 12 } else { 11 };
        let (b, picked_role): (Result<Option<Build>, String>, Option<String>) = match src.as_str() {
            "opgg" if mode == "aram" => (opgg_build(&d, champ, Some("mid"), "aram", "all").await, None),
            "opgg" => {
                let p = if role.is_empty() { main_position(&d, champ).await.ok() } else { Some(role.clone()) };
                (opgg_build(&d, champ, p.as_deref(), "ranked", &rank).await, p)
            }
            "toi" => (my_build(&d, champ, map).await, None),
            _ => {
                let pos = if mode == "aram" { "NONE" } else { riot_pos(&role) };
                (Ok(riot_build(champ, pos, map).await), None)
            }
        };
        on_ui(move |a| {
            if (a.gen, a.builds.gen) != gen {
                return;
            }
            a.d = Some(d);
            if let Some(p) = picked_role {
                if a.builds.role.is_empty() {
                    a.builds.role = p;
                }
            }
            match b {
                Ok(Some(x)) => {
                    a.builds.b = Some(x);
                    a.builds.loaded = true;
                    render_builds(a);
                }
                _ => match a.builds.src.as_str() {
                    "opgg" => {
                        a.builds.src = "toi".into();
                        load_builds(a);
                    }
                    "toi" => {
                        a.builds.src = "riot".into();
                        load_builds(a);
                    }
                    _ => {
                        a.builds.b = None;
                        a.builds.loaded = true;
                        render_builds(a);
                    }
                },
            }
        });
    });
}

fn riot_pos(r: &str) -> &'static str {
    match r {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "mid" => "MIDDLE",
        "adc" => "BOTTOM",
        "support" => "UTILITY",
        _ => "NONE",
    }
}

fn steps(d: &DD, ids: &[i64], arrows: bool) -> slint::ModelRc<ItemStep> {
    ms(ids.iter().enumerate().map(|(i, id)| ItemStep { icon: ss(img_item(d, *id)), arrow: arrows && i > 0, name: ss(d.items.get(id).map(|x| x.name.clone()).unwrap_or_default()) }).collect())
}

fn iopt_rows(d: &DD, label: &str, opts: &[&Opt2], arrows: bool) -> IOptRow {
    let list: Vec<IOpt> = opts.iter().enumerate().map(|(i, o)| IOpt { items: steps(d, &o.ids, arrows), win: ss(pct1(o.win)), tone: good(o.win >= 0.5), pick: ss(format!("{} des parties", pct1(o.pick))), best: i == 0 }).collect();
    let chunks: Vec<slint::ModelRc<IOpt>> = list.chunks(2).map(|c| ms(c.to_vec())).collect();
    IOptRow { label: ss(label), rows: ms(chunks) }
}

type Opt2 = crate::opgg::Opt;

fn chip(text: impl AsRef<str>, tone: i32) -> Chip {
    Chip { text: ss(text), tone, pill: false }
}

fn jungle_data(a: &App, d: &DD) -> JungleData {
    let st = &a.builds;
    let r = jungle::route(&st.jteam, &st.jstart);
    let last = r.steps.len() - 1;
    let mut path = String::new();
    for (i, s) in r.steps.iter().enumerate() {
        let (x, y) = jungle::pt(s.1);
        path += &format!("{}{x:.1} {y:.1} ", if i == 0 { "M" } else { "L" });
    }
    let leash = if (st.jteam == "blue") == (st.jstart == "red") { "bot" } else { "top" };
    JungleData {
        show: true,
        teams: ms(vec![opt("jteam:blue", "Équipe bleue", st.jteam == "blue"), opt("jteam:red", "Équipe rouge", st.jteam == "red")]),
        starts: ms(vec![opt("jstart:red", "Départ rouge", st.jstart == "red"), opt("jstart:blue", "Départ bleu", st.jstart == "blue")]),
        map: ss(format!("https://ddragon.leagueoflegends.com/cdn/{}/img/map/map11.png", d.v)),
        path: ss(path),
        steps: ms(r
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (x, y) = jungle::pt(s.1);
                JungleStep {
                    n: ss(if i == last { "C".to_string() } else { (i + 1).to_string() }),
                    name: ss(s.0),
                    note: ss(if i == 0 {
                        "apparition 1:30"
                    } else if i == last {
                        "apparition 3:30"
                    } else {
                        ""
                    }),
                    x: x as f32,
                    y: y as f32,
                    crab: i == last,
                }
            })
            .collect()),
        text: ss(format!(
            "Full clear classique qui finit côté {} : prépare ton premier gank {}, puis prends le crabe de cette rivière. Le départ {} demande en général un leash de ta lane {leash}.",
            r.side,
            if r.side == "top" { "sur le top ou le mid" } else { "sur le bot ou le mid" },
            if st.jstart == "red" { "rouge" } else { "bleu" },
        )),
    }
}

pub fn render_builds(a: &mut App) {
    let Some(d) = a.d.clone() else { return };
    let cfg = settings();
    let st = &a.builds;
    let c = d.champs.get(&st.champ);
    let aram = st.mode == "aram";
    let o = st.b.as_ref().and_then(|b| b.extra.as_deref());
    let mut roles = vec![];
    if st.src != "toi" && !aram {
        for (k, l) in POS_LABEL {
            let mut x = opt(format!("brole:{k}"), l, st.role == k);
            x.icon = ss(img_role(k));
            roles.push(x);
        }
    }
    let ranks = if st.src == "opgg" && !aram { RANK_FILTERS.iter().map(|(k, l)| opt(format!("brank:{k}"), l, st.rank == *k)).collect() } else { vec![] };
    let mut data = BuildsData {
        loading: !st.loaded,
        none: st.loaded && st.b.is_none(),
        compact: st.compact,
        champ: ss(img_champ(&d, st.champ)),
        name: ss(c.map(|x| x.name.clone()).unwrap_or_default()),
        tags: ss(c.map(|x| x.tags.join(", ")).unwrap_or_default()),
        splash: ss(c.map(|x| img_splash(&x.id)).unwrap_or_default()),
        route: ss(format!("champion/{}/{}", st.champ, st.role)),
        sources: ms(vec![opt("bsrc:opgg", "Méta OP.GG", st.src == "opgg"), opt("bsrc:toi", "Tes parties", st.src == "toi"), opt("bsrc:riot", "Reco Riot", st.src == "riot")]),
        modes: ms(vec![opt("bmode:ranked", "Faille", !aram), opt("bmode:aram", "ARAM", aram)]),
        roles: ms(roles),
        ranks: ms(ranks),
        msg: ss(&st.msg),
        can_apply: st.b.is_some(),
        auto_runes: cfg.auto_runes,
        auto_spells: cfg.auto_spells,
        auto_items: cfg.auto_items,
        ..Default::default()
    };
    if let Some(x) = o {
        data.has_stats = !st.compact;
        data.tier = ss(tier_label(x.tier));
        data.tier_tone = x.tier.min(3) as i32;
        data.stats = ms(vec![stat(pct1(x.win_rate), "Winrate", good(x.win_rate >= 0.5)), stat(pct1(x.pick_rate), "Pick", 0), stat(pct1(x.ban_rate), "Ban", 0), stat(fr(st.b.as_ref().map(|b| b.games).unwrap_or(0)), "Parties", 0)]);
    }
    if let Some(b) = &st.b {
        data.rune_chips = ms(if let Some(x) = o {
            vec![chip(format!("{} victoires", pct1(x.rune_win)), good(x.rune_win >= 0.5)), chip(format!("{} des joueurs", pct1(x.rune_pick)), 4), chip(format!("{} parties", fr(x.rune_play)), 4)]
        } else if b.games > 0 {
            let w = b.wins as f64 / b.games as f64;
            vec![chip(format!("{} victoires", pct0(w)), good(w >= 0.5)), chip(format!("{} de tes parties", b.games), 4)]
        } else {
            vec![chip("Recommandation Riot", 4)]
        });
        data.page_name = ss(page_name(&d, st.champ, b, &st.role, aram));
        let mut trees = vec![];
        for (i, sid) in [b.primary, b.sub].iter().enumerate() {
            let Some(style) = d.styles.iter().find(|s| s.rune.id == *sid) else { continue };
            let slots = style
                .slots
                .iter()
                .skip(if i > 0 { 1 } else { 0 })
                .enumerate()
                .map(|(k, sl)| RuneSlot { keys: k == 0 && i == 0, runes: ms(sl.iter().map(|r| RuneCell { icon: ss(if b.perks.contains(&r.id) { img_rune(&d, r.id) } else { format!("gray:{}", img_rune(&d, r.id)) }), on: b.perks.contains(&r.id), name: ss(&r.name) }).collect()) })
                .collect();
            trees.push(RuneTree { icon: ss(img_rune(&d, *sid)), name: ss(&style.rune.name), slots: ms(slots) });
        }
        data.trees = ms(trees);
        data.shards = ms(if b.perks.len() >= 9 { b.perks[6..9].iter().map(|id| ss(img_rune(&d, *id))).collect() } else { vec![] });
        data.spells = ms(b.spells.iter().map(|s| ss(img_spell(&d, *s))).collect());
        data.spell_chips = ms(match o {
            Some(x) if x.spell_play > 0 => vec![chip(format!("{} victoires", pct1(x.spell_win)), good(x.spell_win >= 0.5)), chip(format!("{} des joueurs", pct1(x.spell_pick)), 4)],
            _ => vec![],
        });
        if let Some(x) = o {
            let mut rows = vec![];
            if let Some(v) = &x.start_opt {
                rows.push(iopt_rows(&d, "Départ", &[v], false));
            }
            if let Some(v) = &x.core_opt {
                rows.push(iopt_rows(&d, "Build principal", &[v], true));
            }
            if let Some(v) = &x.boots_opt {
                rows.push(iopt_rows(&d, "Bottes", &[v], false));
            }
            for (l, list) in [("4e objet", &x.fourth), ("5e objet", &x.fifth), ("6e objet", &x.sixth)] {
                if !list.is_empty() {
                    rows.push(iopt_rows(&d, l, &list.iter().collect::<Vec<_>>(), false));
                }
            }
            data.opts = ms(rows);
        } else {
            let mut plain = vec![];
            for (t, ids, arrows) in [("Objets de départ", &b.start, false), ("Ordre d'achat", &b.order, true), ("Objets finis les plus joués", &b.core, false)] {
                if !ids.is_empty() {
                    plain.push(ItemsPlain { title: ss(t), items: steps(&d, ids, arrows) });
                }
            }
            data.plain = ms(plain);
        }
        data.riot_note = b.source == "riot";
        let pick = o.and_then(|x| x.skill_builds.get(st.sb));
        let slot = |k: &String| match k.as_str() {
            "Q" => 1,
            "W" => 2,
            "E" => 3,
            "R" => 4,
            _ => 0,
        };
        let order: Vec<i64> = pick.map(|p| p.order.iter().map(slot).collect()).unwrap_or_else(|| b.skills.clone());
        data.has_skills = b.skills.iter().any(|s| *s != 0);
        data.max_line = ss(match (pick, o) {
            (Some(p), _) => format!("Max {}", p.max.join(" › ")),
            (None, Some(x)) if !x.max_order.is_empty() => format!("Max {}", x.max_order.join(" › ")),
            _ => String::new(),
        });
        if let Some(x) = o {
            data.variants = ms(x
                .skill_builds
                .iter()
                .enumerate()
                .map(|(i, v)| SkillVariant { first: ss(v.order.iter().take(3).cloned().collect::<Vec<_>>().join(" ")), max: ss(format!("Max {}", v.max.join(" › "))), win: ss(pct1(v.win)), tone: good(v.win >= 0.5), pick: ss(pct1(v.pick)), on: i == st.sb, key: ss(format!("bsb:{i}")) })
                .collect());
        }
        if let Some(cc) = c {
            data.skills = ms(["Q", "W", "E", "R"]
                .iter()
                .enumerate()
                .map(|(sl, k)| SkillRow { icon: ss(img_ability(&cc.id, k)), key: ss(k), cells: ms(order.iter().map(|s| SkillCell { on: *s == sl as i64 + 1, key: ss(if *s == sl as i64 + 1 { *k } else { "" }) }).collect()) })
                .collect());
        }
        data.levels = ms((1..=order.len()).map(|i| ss(i.to_string())).collect());
        if let Some(x) = o {
            data.has_counters = !x.weak.is_empty() || !x.strong.is_empty();
            let cell = |k: &Counter, gd: bool| CounterCell { icon: ss(img_champ(&d, k.champ)), win: ss(pct1(k.my_win)), good: gd, name: ss(d.champs.get(&k.champ).map(|c| c.name.clone()).unwrap_or_else(|| k.name.clone())), route: ss(format!("champion/{}/{}", k.champ, st.role)) };
            data.weak = ms(x.weak.iter().map(|k| cell(k, false)).collect());
            data.strong = ms(x.strong.iter().map(|k| cell(k, true)).collect());
        }
    }
    if !aram && st.role == "jungle" {
        data.jungle = jungle_data(a, &d);
    }
    a.data().set_builds(data);
}

fn apply_click(a: &mut App) {
    let Some(b) = a.builds.b.clone() else { return };
    let Some(d) = a.d.clone() else { return };
    let (champ, role, aram, gen) = (a.builds.champ, a.builds.role.clone(), a.builds.mode == "aram", a.builds.gen);
    spawn(async move {
        let phase = net::get("/lol-gameflow/v1/gameflow-phase").await.ok();
        let in_cs = phase.as_ref().and_then(|p| p.as_str()) == Some("ChampSelect");
        let r = apply_build(&d, champ, &b, ApplyOpts { runes: true, spells: in_cs, items: true, role: &role, aram }).await;
        on_ui(move |a| {
            if a.builds.gen != gen {
                return;
            }
            a.builds.msg = match r {
                Ok(done) => format!("Importé : {}", done.join(", ")),
                Err(e) => format!("Erreur : {e}"),
            };
            render_builds(a);
        });
    });
}

#[derive(Clone, Default)]
pub struct Slot {
    pub puuid: String,
    pub champion: i64,
    pub position: String,
    pub name: String,
    pub me: bool,
}

#[derive(Clone, Default)]
pub struct PlayerInfo {
    pub name: String,
    pub q: Option<Queue>,
    pub games: Vec<Game>,
}

#[derive(Default)]
pub struct LiveState {
    pub teams: [Vec<Slot>; 2],
    pub info: HashMap<String, PlayerInfo>,
    pub asked: std::collections::HashSet<String>,
    pub aram: bool,
    pub mine: (i64, String),
    pub running: bool,
}

const LIVE: [&str; 4] = ["ChampSelect", "InProgress", "Reconnect", "GameStart"];

fn pos_fr(p: &str) -> String {
    match p {
        "top" => "Top",
        "jungle" => "Jungle",
        "middle" => "Mid",
        "bottom" => "ADC",
        "utility" => "Support",
        x => x,
    }
    .to_string()
}

pub fn open_live(a: &mut App) {
    a.live.teams = Default::default();
    a.live.mine = (0, "NONE".into());
    render_live(a);
    load_live(a);
}

fn load_live(a: &App) {
    let phase = a.phase.clone().unwrap_or_default();
    if !LIVE.contains(&phase.as_str()) {
        return;
    }
    let gen = a.gen;
    spawn(async move {
        let mut aram = false;
        let mut teams: [Vec<Slot>; 2] = Default::default();
        if phase == "ChampSelect" {
            let Ok(s) = net::get("/lol-champ-select/v1/session").await else { return };
            if let Ok(f) = net::get("/lol-gameflow/v1/session").await {
                aram = f["map"]["id"].as_i64().or(f["gameData"]["queue"]["mapId"].as_i64()) == Some(12);
            }
            let cell = s["localPlayerCellId"].as_i64();
            let slot = |m: &serde_json::Value| Slot {
                puuid: m["puuid"].as_str().unwrap_or_default().into(),
                champion: m["championId"].as_i64().filter(|x| *x != 0).or(m["championPickIntent"].as_i64()).unwrap_or(0),
                position: m["assignedPosition"].as_str().unwrap_or_default().into(),
                name: m["gameName"].as_str().unwrap_or_default().into(),
                me: m["cellId"].as_i64() == cell,
            };
            teams[0] = s["myTeam"].as_array().into_iter().flatten().map(slot).collect();
            teams[1] = s["theirTeam"].as_array().into_iter().flatten().map(slot).collect();
        } else {
            let (s, me_) = tokio::join!(net::get("/lol-gameflow/v1/session"), net::get("/lol-summoner/v1/current-summoner"));
            let Ok(s) = s else { return };
            let my = me_.ok().and_then(|m| m["puuid"].as_str().map(String::from)).unwrap_or_default();
            let slot = |m: &serde_json::Value| {
                let pu = m["puuid"].as_str().unwrap_or_default().to_string();
                Slot { me: pu == my && !pu.is_empty(), puuid: pu, champion: m["championId"].as_i64().unwrap_or(0), position: m["selectedPosition"].as_str().unwrap_or_default().to_lowercase(), name: String::new() }
            };
            let x: Vec<Slot> = s["gameData"]["teamOne"].as_array().into_iter().flatten().map(slot).collect();
            let y: Vec<Slot> = s["gameData"]["teamTwo"].as_array().into_iter().flatten().map(slot).collect();
            teams = if y.iter().any(|p| p.me) { [y, x] } else { [x, y] };
        }
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            let was_cs = a.live.mine.clone();
            a.live.aram = aram;
            if let Some(me_) = teams[0].iter().find(|x| x.me).filter(|x| x.champion != 0) {
                a.live.mine = (me_.champion, position_key(&me_.position).to_string());
            }
            a.live.teams = teams;
            let mut wanted = vec![];
            for s in a.live.teams.iter().flatten() {
                if !s.puuid.is_empty() && a.live.asked.insert(s.puuid.clone()) {
                    wanted.push(s.puuid.clone());
                }
            }
            for pu in wanted {
                fetch_player(gen, pu);
            }
            if a.phase.as_deref() == Some("ChampSelect") && a.live.mine.0 != 0 && (a.live.mine != was_cs || a.builds.champ != a.live.mine.0) {
                let (c, r) = a.live.mine.clone();
                let mode = if aram { "aram" } else { "ranked" };
                open_builds(a, c, r, true, mode);
            }
            render_live(a);
        });
    });
}

fn fetch_player(gen: u64, puuid: String) {
    spawn(async move {
        let (s, r, h) = tokio::join!(summoner(&puuid), ranked(&puuid), history(&puuid, 20));
        let Ok(s) = s else { return };
        let info = PlayerInfo { name: s.game_name, q: r.ok().and_then(|mut m| m.remove("RANKED_SOLO_5x5")), games: h.unwrap_or_default() };
        on_ui(move |a| {
            a.live.info.insert(puuid, info);
            if a.gen == gen {
                render_live(a);
            }
        });
    });
}

pub fn player_tags(info: &PlayerInfo, champ: i64) -> Vec<String> {
    let g = &info.games;
    let on_champ = g.iter().filter(|x| x.me().champion_id == champ).count();
    let first = g.first().map(|x| win(x.me()));
    let streak = first.map(|w| g.iter().take_while(|x| win(x.me()) == w).count()).unwrap_or(0);
    let mut tags = vec![];
    if on_champ >= 12 {
        tags.push("OTP".to_string());
    }
    if streak >= 3 {
        tags.push(if first == Some(true) { format!("{streak} wins") } else { format!("{streak} défaites") });
    }
    if champ != 0 && !g.is_empty() && on_champ == 0 {
        tags.push("1re sur ce champ".to_string());
    }
    tags
}

fn damage(team: &[Slot], d: &DD, label: &str) -> DmgBar {
    let (mut ad, mut ap) = (0.0, 0.0);
    for s in team {
        if let Some(i) = d.champs.get(&s.champion) {
            ad += i.attack / (i.attack + i.magic).max(1.0);
            ap += i.magic / (i.attack + i.magic).max(1.0);
        }
    }
    let tot: f64 = ad + ap;
    if tot == 0.0 {
        return DmgBar::default();
    }
    DmgBar { label: ss(label), ad: (ad / tot) as f32, ap: (ap / tot) as f32, ad_text: ss(format!("{}% AD", (ad / tot * 100.0).round())), ap_text: ss(format!("{}% AP", (ap / tot * 100.0).round())), show: true }
}

fn player_card(s: &Slot, a: &App, d: &DD) -> PlayerCard {
    let info = a.live.info.get(&s.puuid);
    let g: &[Game] = info.map(|i| i.games.as_slice()).unwrap_or(&[]);
    let c = d.champs.get(&s.champion);
    let wins = g.iter().filter(|x| win(x.me())).count();
    let on: Vec<&Game> = g.iter().filter(|x| x.me().champion_id == s.champion).collect();
    let cw = on.iter().filter(|x| win(x.me())).count();
    let q = info.and_then(|i| i.q.as_ref());
    let tier = q.filter(|q| TIERS.contains(&q.tier.as_str()));
    let plain = Color::from_rgb_u8(0xee, 0xf0, 0xf7);
    let wr = if g.is_empty() { 0.0 } else { wins as f64 / g.len() as f64 };
    PlayerCard {
        art: ss(c.map(|x| img_loading(&x.id)).unwrap_or_default()),
        tags: ms(info.map(|i| player_tags(i, s.champion)).unwrap_or_default().iter().map(ss).collect()),
        rank: ss(match tier {
            Some(q) => {
                let high = TIERS.iter().position(|t| *t == q.tier).unwrap_or(0) >= 7;
                format!("{} {}", title_tier(&q.tier), if high { "" } else { q.division.as_str() })
            }
            None if info.is_some() => "Non classé".to_string(),
            None => String::new(),
        }),
        rank_color: tier.map(|q| tier_color(&q.tier)).unwrap_or(plain),
        record: ss(q.filter(|q| q.wins + q.losses > 0).map(|q| format!("{}V - {}D", q.wins, q.losses)).unwrap_or_default()),
        name: ss(info.map(|i| i.name.clone()).filter(|x| !x.is_empty()).unwrap_or_else(|| {
            if !s.name.is_empty() {
                s.name.clone()
            } else if s.puuid.is_empty() {
                "Joueur masqué".to_string()
            } else {
                "…".to_string()
            }
        })),
        sub: ss(format!("{}{}", c.map(|x| x.name.as_str()).unwrap_or("Pas encore choisi"), if s.position.is_empty() { String::new() } else { format!(", {}", pos_fr(&s.position)) })),
        wr: ss(if g.is_empty() { String::new() } else { format!("{} récent", pct0(wr)) }),
        wr_tone: good(wr >= 0.5),
        champ_line: ss(if on.is_empty() { String::new() } else { format!("{} en {} parties, {:.1} KDA", pct0(cw as f64 / on.len() as f64), on.len(), on.iter().map(|x| kda(&x.me().stats)).sum::<f64>() / on.len() as f64) }),
        route: ss(if s.puuid.is_empty() { String::new() } else { format!("profil/{}", s.puuid) }),
        me: s.me,
    }
}

pub fn render_live(a: &mut App) {
    let phase = a.phase.clone().unwrap_or_default();
    if !LIVE.contains(&phase.as_str()) {
        a.data().set_live(LiveData { waiting: true, ..Default::default() });
        return;
    }
    let Some(d) = a.d.clone() else { return };
    let cs_phase = phase == "ChampSelect";
    let t = &a.live.teams;
    let data = LiveData {
        waiting: false,
        title: ss(if cs_phase { "Sélection des champions" } else { "Partie en cours" }),
        show_build: cs_phase && a.live.mine.0 != 0,
        build_hint: cs_phase && a.live.mine.0 == 0,
        dmg1: damage(&t[0], &d, "Dégâts de ton équipe (estimés)"),
        dmg2: if t[1].iter().any(|x| x.champion != 0) { damage(&t[1], &d, "Dégâts adverses (estimés)") } else { DmgBar::default() },
        team1: ms(t[0].iter().map(|s| player_card(s, a, &d)).collect()),
        team2: ms(t[1].iter().map(|s| player_card(s, a, &d)).collect()),
    };
    a.data().set_live(data);
}

pub fn on_phase(a: &mut App, p: Option<String>) {
    let old = a.phase.clone();
    a.phase = p.clone();
    let cfg = settings();
    if p.as_deref() == Some("ChampSelect") && old.as_deref() != Some("ChampSelect") && cfg.auto_open_live && a.game == "lol" {
        a.go("partie");
    }
    if p.as_deref() == Some("ReadyCheck") && cfg.auto_accept {
        spawn(async {
            let _ = net::lcu("POST", "/lol-matchmaking/v1/ready-check/accept", None).await;
        });
    }
    if p.as_deref() != Some("ChampSelect") {
        a.imported = 0;
    }
    if old != p {
        a.live.asked.clear();
        a.refresh_view();
        if a.page() == "partie" {
            render_live(a);
        }
    } else {
        a.render_tabs();
        a.render_shell();
    }
    if a.page() == "partie" && a.key.starts_with("partie|") {
        a.live.running = !a.live.running;
        if a.live.running {
            load_live(a);
        }
    }
    if p.as_deref() == Some("ChampSelect") && (cfg.auto_runes || cfg.auto_spells || cfg.auto_items) {
        auto_import(a, cfg);
    }
}

fn auto_import(a: &mut App, s: crate::store::Settings) {
    let done = a.imported;
    spawn(async move {
        let champ = net::get("/lol-champ-select/v1/current-champion").await.ok().and_then(|v| v.as_i64()).unwrap_or(0);
        if champ == 0 || champ == done {
            return;
        }
        on_ui(move |a| a.imported = champ);
        let r: Result<String, String> = async {
            let dd = ddragon().await?;
            let sess = net::get("/lol-champ-select/v1/session").await?;
            let cell = sess["localPlayerCellId"].as_i64();
            let pos = sess["myTeam"].as_array().into_iter().flatten().find(|m| m["cellId"].as_i64() == cell).and_then(|m| m["assignedPosition"].as_str()).unwrap_or_default().to_string();
            let flow = net::get("/lol-gameflow/v1/session").await.unwrap_or_default();
            let map = flow["map"]["id"].as_i64().or(flow["gameData"]["queue"]["mapId"].as_i64()).unwrap_or(11);
            let key = position_key(&pos);
            let name = dd.champs.get(&champ).map(|c| c.name.clone()).unwrap_or_default();
            let Some(b) = best_build(&dd, champ, key, map, &s.source).await else {
                return Ok(format!("Pas de build trouvé pour {name}"));
            };
            let list = apply_build(&dd, champ, &b, ApplyOpts { runes: s.auto_runes, spells: s.auto_spells, items: s.auto_items, role: if key == "NONE" { "" } else { key }, aram: map == 12 }).await?;
            let origin = match b.source {
                "toi" => format!("tes {} parties", b.games),
                "opgg" => "méta OP.GG".to_string(),
                _ => "reco Riot".to_string(),
            };
            Ok(format!("{name} : {} importés ({origin})", list.join(", ")))
        }
        .await;
        let msg = r.unwrap_or_else(|e| format!("Import impossible : {e}"));
        on_ui(move |a| a.toast(msg));
    });
}

pub fn action(a: &mut App, key: &str) -> bool {
    let cfg = settings;
    if let Some(k) = key.strip_prefix("tier-q:") {
        a.tier.q = k.into();
        render_tier(a);
    } else if key == "tfav" {
        a.tier.fav_only = !a.tier.fav_only;
        render_tier(a);
    } else if let Some(k) = key.strip_prefix("tpos:") {
        a.tier.pos = k.into();
        render_tier(a);
    } else if let Some(k) = key.strip_prefix("tsort:") {
        let same = a.tier.sort == k;
        a.tier.desc = if same { !a.tier.desc } else { k != "tier" };
        a.tier.sort = k.into();
        render_tier(a);
    } else if let Some(k) = key.strip_prefix("fav:") {
        toggle_fav(k.parse().unwrap_or(0));
        render_tier(a);
    } else if let Some(k) = key.strip_prefix("champ-q:") {
        a.champs.q = k.into();
        render_champs(a);
    } else if let Some(k) = key.strip_prefix("ccls:") {
        a.champs.cls = k.into();
        render_champs(a);
    } else if let Some(k) = key.strip_prefix("cplayed:") {
        a.champs.played = k == "1";
        render_champs(a);
    } else if let Some(k) = key.strip_prefix("csort:") {
        let same = a.champs.sort == k;
        a.champs.desc = if same { !a.champs.desc } else { k != "name" };
        a.champs.sort = k.into();
        render_champs(a);
    } else if let Some(k) = key.strip_prefix("bsrc:") {
        a.builds.src = k.into();
        load_builds(a);
    } else if let Some(k) = key.strip_prefix("bmode:") {
        a.builds.mode = k.into();
        load_builds(a);
    } else if let Some(k) = key.strip_prefix("brole:") {
        a.builds.role = k.into();
        load_builds(a);
    } else if let Some(k) = key.strip_prefix("brank:") {
        a.builds.rank = k.into();
        let mut s = cfg();
        s.rank_filter = k.into();
        save_settings(&s);
        load_builds(a);
    } else if let Some(k) = key.strip_prefix("bsb:") {
        a.builds.sb = k.parse().unwrap_or(0);
        render_builds(a);
    } else if let Some(k) = key.strip_prefix("jteam:") {
        a.builds.jteam = k.into();
        render_builds(a);
    } else if let Some(k) = key.strip_prefix("jstart:") {
        a.builds.jstart = k.into();
        let mut s = cfg();
        s.jungle_route = k.into();
        save_settings(&s);
        render_builds(a);
    } else if let Some(k) = key.strip_prefix("auto:") {
        let mut s = cfg();
        match k {
            "autoRunes" => s.auto_runes = !s.auto_runes,
            "autoSpells" => s.auto_spells = !s.auto_spells,
            _ => s.auto_items = !s.auto_items,
        }
        save_settings(&s);
        render_builds(a);
    } else if key == "apply-build" {
        apply_click(a);
    } else {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn g(champ: i64, w: bool) -> Game {
        serde_json::from_value(json!({"participants": [{"championId": champ, "stats": {"win": w}}]})).unwrap()
    }

    #[test]
    fn tags() {
        let mut games: Vec<Game> = (0..12).map(|_| g(7, true)).collect();
        games.push(g(8, false));
        let info = PlayerInfo { games, ..Default::default() };
        let t = player_tags(&info, 7);
        assert!(t.contains(&"OTP".to_string()));
        assert!(t.contains(&"12 wins".to_string()));
        assert_eq!(player_tags(&info, 99).iter().filter(|x| *x == "1re sur ce champ").count(), 1);
        assert_eq!(player_tags(&PlayerInfo::default(), 7), Vec::<String>::new());
    }

    #[test]
    fn damage_split() {
        let mut d = DD::default();
        d.champs.insert(1, Champ { attack: 8.0, magic: 2.0, ..Default::default() });
        let b = damage(&[Slot { champion: 1, ..Default::default() }], &d, "x");
        assert!(b.show);
        assert!((b.ad - 0.8).abs() < 0.01);
        assert!(!damage(&[], &d, "x").show);
    }
}
