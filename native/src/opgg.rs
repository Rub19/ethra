use crate::lol::DD;
use crate::net;
use crate::opgg_parse::parse_opgg;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

pub fn opgg_pos(p: &str) -> Option<&'static str> {
    match p {
        "TOP" => Some("top"),
        "JUNGLE" => Some("jungle"),
        "MIDDLE" => Some("mid"),
        "BOTTOM" => Some("adc"),
        "UTILITY" => Some("support"),
        _ => None,
    }
}

pub const POS_LABEL: [(&str, &str); 5] = [("top", "Top"), ("jungle", "Jungle"), ("mid", "Mid"), ("adc", "ADC"), ("support", "Support")];
pub const RANK_FILTERS: [(&str, &str); 4] = [("all", "Tous les rangs"), ("emerald_plus", "Émeraude+"), ("diamond_plus", "Diamant+"), ("master_plus", "Master+")];

pub fn pos_label(k: &str) -> &'static str {
    POS_LABEL.iter().find(|(a, _)| *a == k).map(|x| x.1).unwrap_or("")
}

pub fn tier_label(t: i64) -> String {
    match t {
        1 => "S".into(),
        2 => "A".into(),
        3 => "B".into(),
        4 => "C".into(),
        5 => "D".into(),
        x => x.to_string(),
    }
}

pub async fn call(tool: &str, args: Value) -> Result<Value, String> {
    static CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = format!("{tool}{args}");
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return Ok(v.clone());
    }
    let t = net::opgg(tool, &args).await?;
    let first = t.trim_start().chars().next().unwrap_or(' ');
    let v = if first == '[' || first == '{' { serde_json::from_str(&t).map_err(|e| e.to_string())? } else { parse_opgg(&t)? };
    cache.lock().unwrap().insert(key, v.clone());
    Ok(v)
}

#[derive(Clone, Debug, Default)]
pub struct MetaRow {
    pub champ: i64,
    pub name: String,
    pub position: String,
    pub tier: i64,
    pub rank: i64,
    pub rank_prev: i64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    pub play: i64,
    pub kda: f64,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

pub fn champ_id(d: &DD, name: &str) -> i64 {
    let k = norm(&name.to_lowercase());
    let exact = d.champs.iter().find(|(_, c)| norm(&c.id) == k || norm(&c.name) == k);
    let hit = exact.or_else(|| d.champs.iter().find(|(_, c)| k.starts_with(&norm(&c.id))));
    hit.map(|(id, _)| *id).unwrap_or(0)
}

fn f(x: &Value) -> f64 {
    x.as_f64().unwrap_or(0.0)
}

fn i(x: &Value) -> i64 {
    x.as_i64().or_else(|| x.as_f64().map(|v| v as i64)).unwrap_or(0)
}

fn ints(x: &Value) -> Vec<i64> {
    x.as_array().into_iter().flatten().map(i).collect()
}

pub async fn meta(d: &DD) -> Result<Vec<MetaRow>, String> {
    let fields: Vec<String> = ["top", "jungle", "mid", "adc", "support"].iter().map(|p| format!("data.positions.{p}[].{{ban_rate,champion,kda,pick_rate,play,rank,rank_prev,tier,win_rate}}")).collect();
    let r = call("lol_list_lane_meta_champions", json!({"position": "all", "desired_output_fields": fields})).await?;
    let mut out = vec![];
    for (position, rows) in r["data"]["positions"].as_object().into_iter().flatten() {
        for x in rows.as_array().into_iter().flatten() {
            let name = x["champion"].as_str().unwrap_or_default().to_string();
            out.push(MetaRow {
                champ: champ_id(d, &name),
                name,
                position: position.clone(),
                tier: i(&x["tier"]),
                rank: i(&x["rank"]),
                rank_prev: i(&x["rank_prev"]),
                win_rate: f(&x["win_rate"]),
                pick_rate: f(&x["pick_rate"]),
                ban_rate: f(&x["ban_rate"]),
                play: i(&x["play"]),
                kda: f(&x["kda"]),
            });
        }
    }
    Ok(out)
}

pub async fn main_position(d: &DD, champ: i64) -> Result<String, String> {
    let rows = meta(d).await?;
    Ok(rows.iter().filter(|r| r.champ == champ).max_by_key(|r| r.play).map(|r| r.position.clone()).unwrap_or_else(|| "mid".into()))
}

#[derive(Clone, Debug, Default)]
pub struct Counter {
    pub champ: i64,
    pub name: String,
    pub play: i64,
    pub my_win: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Opt {
    pub ids: Vec<i64>,
    pub win: f64,
    pub pick: f64,
}

#[derive(Clone, Debug, Default)]
pub struct SkillBuild {
    pub order: Vec<String>,
    pub max: Vec<String>,
    pub win: f64,
    pub pick: f64,
}

#[derive(Clone, Debug, Default)]
pub struct OpggExtra {
    pub tier: i64,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub ban_rate: f64,
    pub boots: Vec<i64>,
    pub max_order: Vec<String>,
    pub rune_win: f64,
    pub rune_play: i64,
    pub rune_pick: f64,
    pub spell_win: f64,
    pub spell_play: i64,
    pub spell_pick: f64,
    pub weak: Vec<Counter>,
    pub strong: Vec<Counter>,
    pub start_opt: Option<Opt>,
    pub core_opt: Option<Opt>,
    pub boots_opt: Option<Opt>,
    pub fourth: Vec<Opt>,
    pub fifth: Vec<Opt>,
    pub sixth: Vec<Opt>,
    pub skill_builds: Vec<SkillBuild>,
}

#[derive(Clone, Debug, Default)]
pub struct Build {
    pub source: &'static str,
    pub games: i64,
    pub wins: i64,
    pub primary: i64,
    pub sub: i64,
    pub perks: Vec<i64>,
    pub spells: Vec<i64>,
    pub start: Vec<i64>,
    pub order: Vec<i64>,
    pub core: Vec<i64>,
    pub skills: Vec<i64>,
    pub position: String,
    pub extra: Option<Box<OpggExtra>>,
}

fn opt(x: &Value) -> Option<Opt> {
    let ids = ints(&x["ids"]);
    if ids.is_empty() {
        return None;
    }
    Some(Opt { ids, win: f(&x["win"]) / f(&x["play"]).max(1.0), pick: f(&x["pick_rate"]) })
}

pub fn max_of(order: &[String]) -> Vec<String> {
    let mut n: HashMap<&str, i32> = HashMap::from([("Q", 0), ("W", 0), ("E", 0)]);
    let mut done: Vec<String> = vec![];
    for k in order {
        if let Some(c) = n.get_mut(k.as_str()) {
            *c += 1;
            if *c == 5 {
                done.push(k.clone());
            }
        }
    }
    let mut rest: Vec<&str> = ["Q", "W", "E"].into_iter().filter(|k| !done.iter().any(|d| d == k)).collect();
    rest.sort_by_key(|k| -n[k]);
    done.extend(rest.into_iter().map(String::from));
    done
}

fn slot(k: &str) -> i64 {
    match k {
        "Q" => 1,
        "W" => 2,
        "E" => 3,
        "R" => 4,
        _ => 0,
    }
}

fn opts(x: &Value) -> Vec<Opt> {
    x.as_array().into_iter().flatten().filter_map(opt).take(4).collect()
}

pub fn parse_build(x: &Value, pos: &str) -> Option<Build> {
    if x["runes"].is_null() {
        return None;
    }
    let avg = &x["summary"]["average_stats"];
    let list = |k: &str| x[k].as_array().cloned().unwrap_or_default();
    let mut seen = HashSet::new();
    let mut core: Vec<i64> = vec![];
    for b in list("fourth_items").iter().chain(list("fifth_items").iter()).chain(list("sixth_items").iter()) {
        for id in ints(&b["ids"]) {
            if seen.insert(id) {
                core.push(id);
            }
        }
    }
    core.truncate(8);
    let counter = |k: &Value| Counter { champ: i(&k["champion_id"]), name: k["champion_name"].as_str().unwrap_or_default().into(), play: i(&k["play"]), my_win: f(&k["my_win_rate"]) };
    let counters = |key: &str| list(key).iter().map(counter).take(6).collect::<Vec<_>>();
    let strs = |v: &Value| v.as_array().into_iter().flatten().filter_map(|s| s.as_str().map(String::from)).collect::<Vec<_>>();
    let r = &x["runes"];
    let mut perks = ints(&r["primary_rune_ids"]);
    perks.extend(ints(&r["secondary_rune_ids"]));
    perks.extend(ints(&r["stat_mod_ids"]));
    let sp = &x["summoner_spells"];
    let play = f(&avg["play"]);
    Some(Build {
        source: "opgg",
        games: play as i64,
        wins: (play * f(&avg["win_rate"])).round() as i64,
        primary: i(&r["primary_page_id"]),
        sub: i(&r["secondary_page_id"]),
        perks,
        spells: ints(&sp["ids"]),
        start: ints(&x["starter_items"]["ids"]),
        order: ints(&x["core_items"]["ids"]),
        core,
        skills: strs(&x["skills"]["order"]).iter().map(|k| slot(k)).collect(),
        position: pos.to_string(),
        extra: Some(Box::new(OpggExtra {
            tier: i(&avg["tier"]),
            win_rate: f(&avg["win_rate"]),
            pick_rate: f(&avg["pick_rate"]),
            ban_rate: f(&avg["ban_rate"]),
            boots: ints(&x["boots"]["ids"]),
            max_order: strs(&x["skill_masteries"]["ids"]),
            rune_win: f(&r["win"]) / f(&r["play"]).max(1.0),
            rune_play: i(&r["play"]),
            rune_pick: f(&r["pick_rate"]),
            spell_win: f(&sp["win"]) / f(&sp["play"]).max(1.0),
            spell_play: i(&sp["play"]),
            spell_pick: f(&sp["pick_rate"]),
            weak: counters("weak_counters"),
            strong: counters("strong_counters"),
            start_opt: opt(&x["starter_items"]),
            core_opt: opt(&x["core_items"]),
            boots_opt: opt(&x["boots"]),
            fourth: opts(&x["fourth_items"]),
            fifth: opts(&x["fifth_items"]),
            sixth: opts(&x["sixth_items"]),
            skill_builds: x["skill_masteries"]["builds"]
                .as_array()
                .into_iter()
                .flatten()
                .take(4)
                .map(|k| {
                    let order = strs(&k["order"]);
                    SkillBuild { max: max_of(&order), order, win: f(&k["win"]) / f(&k["play"]).max(1.0), pick: f(&k["pick_rate"]) }
                })
                .collect(),
        })),
    })
}

pub async fn opgg_build(d: &DD, champ: i64, position: Option<&str>, mode: &str, rank_filter: &str) -> Result<Option<Build>, String> {
    let Some(c) = d.champs.get(&champ) else { return Ok(None) };
    let pos = if mode == "aram" {
        "mid".to_string()
    } else {
        match position {
            Some(p) if p != "NONE" && !p.is_empty() => opgg_pos(p).unwrap_or(p).to_string(),
            _ => main_position(d, champ).await?,
        }
    };
    let mut args = json!({
        "champion": c.id.to_uppercase(), "position": pos, "game_mode": mode,
        "desired_output_fields": [
            "data.summary.average_stats", "data.runes", "data.summoner_spells", "data.starter_items", "data.core_items", "data.boots",
            "data.fourth_items", "data.fifth_items", "data.sixth_items", "data.skills", "data.skill_masteries.{ids[]}", "data.skill_masteries.builds[].{order[],pick_rate,play,win}",
            "data.weak_counters", "data.strong_counters"
        ]
    });
    if mode == "ranked" && rank_filter != "all" {
        args["tier"] = json!(rank_filter);
    }
    let r = call("lol_get_champion_analysis", args).await?;
    Ok(parse_build(&r["data"], &pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_order() {
        let o: Vec<String> = "Q E W Q Q R Q E Q E R E E W W R W W".split(' ').map(String::from).collect();
        assert_eq!(max_of(&o), vec!["Q", "E", "W"]);
        let few: Vec<String> = "E E W".split(' ').map(String::from).collect();
        assert_eq!(max_of(&few), vec!["E", "W", "Q"]);
    }

    #[test]
    fn champ_names() {
        let mut d = DD::default();
        d.champs.insert(145, crate::lol::Champ { id: "Kaisa".into(), name: "Kai'Sa".into(), ..Default::default() });
        d.champs.insert(62, crate::lol::Champ { id: "MonkeyKing".into(), name: "Wukong".into(), ..Default::default() });
        assert_eq!(champ_id(&d, "Kai'Sa"), 145);
        assert_eq!(champ_id(&d, "wukong"), 62);
        assert_eq!(champ_id(&d, "Inconnu"), 0);
    }

    #[test]
    fn parse_minimal_build() {
        assert!(parse_build(&Value::Null, "mid").is_none());
        let b = parse_build(&json!({"runes": {"primary_page_id": 8100, "secondary_page_id": 8300, "primary_rune_ids": [1, 2, 3, 4], "secondary_rune_ids": [5, 6], "stat_mod_ids": [7, 8, 9], "win": 5, "play": 10}, "skills": {"order": ["Q", "W", "R"]}}), "mid").unwrap();
        assert_eq!(b.perks.len(), 9);
        assert_eq!(b.skills, vec![1, 2, 4]);
        assert_eq!(b.extra.unwrap().rune_win, 0.5);
    }
}
