use crate::history::{n, win, Participant};
use crate::lol::{history, is_completed, me, DD};
use crate::net::{get, lcu};
use crate::opgg::{opgg_build, Build};
use crate::store::settings;
use crate::text::fr;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn position_key(p: &str) -> &'static str {
    match p {
        "top" => "TOP",
        "jungle" => "JUNGLE",
        "middle" => "MIDDLE",
        "bottom" => "BOTTOM",
        "utility" => "UTILITY",
        _ => "NONE",
    }
}

pub async fn riot_build(champ: i64, position: &str, map: i64) -> Option<Build> {
    let pages = get(&format!("/lol-perks/v1/recommended-pages/champion/{champ}/position/{position}/map/{map}")).await.ok()?;
    let p = pages.get(0)?;
    Some(Build {
        source: "riot",
        primary: p["primaryPerkStyleId"].as_i64().unwrap_or(0),
        sub: p["secondaryPerkStyleId"].as_i64().unwrap_or(0),
        perks: p["perks"].as_array().into_iter().flatten().filter_map(|x| x["id"].as_i64()).collect(),
        spells: p["summonerSpellIds"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect(),
        ..Default::default()
    })
}

pub fn build_from_games(d: &DD, ps: &[Participant]) -> Option<Build> {
    if ps.is_empty() {
        return None;
    }
    let wins = ps.iter().filter(|p| win(p)).count() as i64;
    let page_key = |p: &Participant| ["perk0", "perk1", "perk2", "perk3", "perk4", "perk5", "perkPrimaryStyle", "perkSubStyle"].iter().map(|k| n(&p.stats, k).to_string()).collect::<Vec<_>>().join(",");
    let mut scores: HashMap<String, i64> = HashMap::new();
    for p in ps {
        *scores.entry(page_key(p)).or_default() += if win(p) { 2 } else { 1 };
    }
    let mut best = &ps[0];
    for p in ps {
        if scores[&page_key(p)] > scores[&page_key(best)] {
            best = p;
        }
    }
    let s = &best.stats;
    let mut perks: Vec<i64> = (0..6).map(|k| n(s, &format!("perk{k}")) as i64).collect();
    let shards: Vec<i64> = (0..3).map(|k| n(s, &format!("statPerk{k}")) as i64).filter(|x| *x != 0).collect();
    if shards.len() == 3 {
        perks.extend(shards);
    }
    let mut spell_count: Vec<((i64, i64), i64, (i64, i64))> = vec![];
    for p in ps {
        let key = (p.spell1_id.min(p.spell2_id), p.spell1_id.max(p.spell2_id));
        match spell_count.iter_mut().find(|x| x.0 == key) {
            Some(x) => {
                x.1 += 1;
                x.2 = (p.spell1_id, p.spell2_id);
            }
            None => spell_count.push((key, 1, (p.spell1_id, p.spell2_id))),
        }
    }
    let spells = spell_count.iter().rev().max_by_key(|x| x.1).map(|x| vec![x.2 .0, x.2 .1]).unwrap_or_default();
    let orders: Vec<Vec<i64>> = ps.iter().map(|p| (0..6).map(|k| n(&p.stats, &format!("item{k}")) as i64).filter(|id| is_completed(d, *id)).collect()).collect();
    let mut rank: Vec<(i64, Vec<usize>)> = vec![];
    for o in &orders {
        for (i, id) in o.iter().enumerate() {
            match rank.iter_mut().find(|x| x.0 == *id) {
                Some(x) => x.1.push(i),
                None => rank.push((*id, vec![i])),
            }
        }
    }
    let need = (orders.len() as f64 * 0.3).max(1.0);
    let mut kept: Vec<&(i64, Vec<usize>)> = rank.iter().filter(|x| x.1.len() as f64 >= need).collect();
    let mean = |r: &[usize]| r.iter().sum::<usize>() as f64 / r.len() as f64;
    kept.sort_by(|a, b| mean(&a.1).total_cmp(&mean(&b.1)));
    let mut by_count: Vec<&(i64, Vec<usize>)> = rank.iter().collect();
    by_count.sort_by_key(|x| std::cmp::Reverse(x.1.len()));
    Some(Build {
        source: "toi",
        games: ps.len() as i64,
        wins,
        primary: n(s, "perkPrimaryStyle") as i64,
        sub: n(s, "perkSubStyle") as i64,
        perks,
        spells,
        order: kept.iter().take(6).map(|x| x.0).collect(),
        core: by_count.iter().take(9).map(|x| x.0).collect(),
        ..Default::default()
    })
}

pub async fn my_build(d: &DD, champ: i64, map: i64) -> Result<Option<Build>, String> {
    let self_ = me().await?;
    let games = history(&self_.puuid, 100).await?;
    let ps: Vec<Participant> = games.iter().filter(|g| g.me().champion_id == champ && (g.map_id == 0 || g.map_id == map)).map(|g| g.me().clone()).collect();
    Ok(build_from_games(d, &ps))
}

pub async fn best_build(d: &DD, champ: i64, position: &str, map: i64, source: &str) -> Option<Build> {
    let riot = riot_build(champ, position, map).await;
    let opgg = || async {
        if map != 11 && map != 12 {
            return None;
        }
        let mode = if map == 12 { "aram" } else { "ranked" };
        opgg_build(d, champ, Some(position), mode, &settings().rank_filter).await.ok().flatten()
    };
    let mine = || async {
        let mut b = my_build(d, champ, map).await.ok().flatten()?;
        if b.perks.len() < 9 {
            if let Some(r) = &riot {
                b.perks.extend(r.perks.iter().skip(6));
            }
        }
        Some(b)
    };
    match source {
        "riot" => riot.clone(),
        "toi" => mine().await.or(riot.clone()),
        "opgg" => opgg().await.or(riot.clone()),
        _ => match opgg().await {
            Some(b) => Some(b),
            None => mine().await.or(riot.clone()),
        },
    }
}

fn role_label(r: &str) -> &'static str {
    match r {
        "top" | "TOP" => "Top",
        "jungle" | "JUNGLE" => "Jungle",
        "mid" | "MIDDLE" => "Mid",
        "adc" | "BOTTOM" => "ADC",
        "support" | "UTILITY" => "Support",
        _ => "",
    }
}

pub fn page_name(d: &DD, champ: i64, b: &Build, role: &str, aram: bool) -> String {
    let pos = if aram {
        "ARAM"
    } else {
        let p = role_label(&b.position);
        if p.is_empty() {
            role_label(role)
        } else {
            p
        }
    };
    let name = d.champs.get(&champ).map(|c| c.name.clone()).unwrap_or_else(|| champ.to_string());
    if pos.is_empty() {
        format!("Kairo : {name}")
    } else {
        format!("Kairo : {name} {pos}")
    }
}

pub struct ApplyOpts<'a> {
    pub runes: bool,
    pub spells: bool,
    pub items: bool,
    pub role: &'a str,
    pub aram: bool,
}

fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|x| x.as_nanos()).unwrap_or(0);
    let h = format!("{t:032x}");
    format!("{}-{}-4{}-a{}-{}", &h[0..8], &h[8..12], &h[13..16], &h[17..20], &h[20..32])
}

pub fn item_blocks(b: &Build) -> Value {
    let block = |t: String, ids: &[i64]| json!({"type": t, "items": ids.iter().map(|id| json!({"id": id.to_string(), "count": 1})).collect::<Vec<_>>()});
    let mut items = b.order.clone();
    items.extend(b.core.iter().filter(|x| !b.order.contains(x)));
    let mut blocks = vec![];
    if !b.start.is_empty() {
        blocks.push(block("Départ".into(), &b.start));
    }
    let title = if b.source == "opgg" { format!("Build méta OP.GG ({} parties)", fr(b.games)) } else { format!("Ton build ({} parties)", b.games) };
    blocks.push(block(title, &items));
    if let Some(x) = b.extra.as_ref().filter(|x| !x.boots.is_empty()) {
        blocks.push(block("Bottes".into(), &x.boots));
    }
    Value::Array(blocks)
}

pub async fn apply_build(d: &DD, champ: i64, b: &Build, o: ApplyOpts<'_>) -> Result<Vec<&'static str>, String> {
    let name = page_name(d, champ, b, o.role, o.aram);
    let mut done = vec![];
    if o.runes && b.perks.len() >= 9 {
        let pages = get("/lol-perks/v1/pages").await?;
        let old = pages.as_array().into_iter().flatten().find(|p| {
            let n = p["name"].as_str().unwrap_or_default();
            n.starts_with("Kairo") || n.starts_with("Ethra")
        });
        if let Some(id) = old.and_then(|p| p["id"].as_i64()) {
            lcu("DELETE", &format!("/lol-perks/v1/pages/{id}"), None).await?;
        }
        lcu("POST", "/lol-perks/v1/pages", Some(json!({"name": name, "primaryStyleId": b.primary, "subStyleId": b.sub, "selectedPerkIds": b.perks, "current": true}))).await?;
        done.push("runes");
    }
    if o.spells && b.spells.len() == 2 {
        lcu("PATCH", "/lol-champ-select/v1/session/my-selection", Some(json!({"spell1Id": b.spells[0], "spell2Id": b.spells[1]}))).await?;
        done.push("sorts");
    }
    if o.items && !(b.order.is_empty() && b.core.is_empty()) {
        let self_ = me().await?;
        let path = format!("/lol-item-sets/v1/item-sets/{}/sets", self_.summoner_id);
        let mut sets = get(&path).await?;
        let mut list: Vec<Value> = sets["itemSets"].as_array().cloned().unwrap_or_default();
        list.retain(|s| {
            let t = s["title"].as_str().unwrap_or_default();
            !t.starts_with("Kairo") && !t.starts_with("Ethra")
        });
        list.push(json!({"title": name, "associatedChampions": [champ], "associatedMaps": [], "map": "any", "mode": "any", "type": "custom", "sortrank": 0, "uid": uuid(), "blocks": item_blocks(b)}));
        sets["itemSets"] = Value::Array(list);
        lcu("PUT", &path, Some(sets)).await?;
        done.push("items");
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lol::{Champ, Item};

    fn dd() -> DD {
        let mut d = DD::default();
        d.champs.insert(1, Champ { id: "Annie".into(), name: "Annie".into(), ..Default::default() });
        for id in [10, 20, 30] {
            d.items.insert(id, Item { name: id.to_string(), total: 3000, into: false, tags: vec![] });
        }
        d.items.insert(99, Item { name: "potion".into(), total: 50, into: false, tags: vec!["Consumable".into()] });
        d
    }

    fn part(won: bool, perk: i64, items: [i64; 3]) -> Participant {
        serde_json::from_value(json!({"spell1Id": 4, "spell2Id": 14, "stats": {"win": won, "perk0": perk, "perk1": 2, "perk2": 3, "perk3": 4, "perk4": 5, "perk5": 6, "perkPrimaryStyle": 8100, "perkSubStyle": 8300, "statPerk0": 7, "statPerk1": 8, "statPerk2": 9, "item0": items[0], "item1": items[1], "item2": items[2]}})).unwrap()
    }

    #[test]
    fn from_games() {
        let d = dd();
        let b = build_from_games(&d, &[part(true, 100, [10, 20, 99]), part(true, 100, [10, 30, 0]), part(false, 200, [20, 10, 0])]).unwrap();
        assert_eq!((b.games, b.wins), (3, 2));
        assert_eq!(b.perks[0], 100);
        assert_eq!(b.perks.len(), 9);
        assert_eq!(b.spells, vec![4, 14]);
        assert_eq!(b.core[0], 10);
        assert!(!b.core.contains(&99));
        assert!(build_from_games(&d, &[]).is_none());
    }

    #[test]
    fn names() {
        let d = dd();
        let b = Build { position: "mid".into(), ..Default::default() };
        assert_eq!(page_name(&d, 1, &b, "", false), "Kairo : Annie Mid");
        assert_eq!(page_name(&d, 1, &b, "", true), "Kairo : Annie ARAM");
        assert_eq!(page_name(&d, 1, &Build::default(), "UTILITY", false), "Kairo : Annie Support");
        assert_eq!(page_name(&d, 1, &Build::default(), "", false), "Kairo : Annie");
    }

    #[test]
    fn blocks() {
        let b = Build { source: "opgg", games: 12000, order: vec![1, 2], core: vec![2, 3], start: vec![9], ..Default::default() };
        let v = item_blocks(&b);
        assert_eq!(v[0]["type"], "Départ");
        assert_eq!(v[1]["items"].as_array().unwrap().len(), 3);
    }
}
