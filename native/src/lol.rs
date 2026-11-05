use crate::history::Game;
use crate::net::{get, json_url};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::OnceCell;

const CDN: &str = "https://ddragon.leagueoflegends.com/cdn";
const CD: &str = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/";

#[derive(Clone, Debug, Default)]
pub struct Champ {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub attack: f64,
    pub magic: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Item {
    pub name: String,
    pub total: i64,
    pub into: bool,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Rune {
    pub id: i64,
    pub name: String,
    pub icon: String,
}

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub rune: Rune,
    pub slots: Vec<Vec<Rune>>,
}

#[derive(Clone, Debug, Default)]
pub struct DD {
    pub v: String,
    pub champs: BTreeMap<i64, Champ>,
    pub items: HashMap<i64, Item>,
    pub styles: Vec<Style>,
    pub runes: HashMap<i64, Rune>,
    pub spells: HashMap<i64, (String, String)>,
    pub perk_icons: HashMap<i64, String>,
}

fn to_i(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}

fn rune_of(x: &Value) -> Rune {
    Rune { id: x["id"].as_i64().unwrap_or(0), name: x["name"].as_str().unwrap_or_default().into(), icon: x["icon"].as_str().unwrap_or_default().into() }
}

fn strs(x: &Value) -> Vec<String> {
    x.as_array().into_iter().flatten().filter_map(|t| t.as_str().map(String::from)).collect()
}

pub fn parse_dd(v: String, c: &Value, i: &Value, st: &Value, s: &Value, perks: &Value) -> DD {
    let mut d = DD { v, ..Default::default() };
    for p in perks.as_array().into_iter().flatten() {
        if let (Some(id), Some(path)) = (p["id"].as_i64(), p["iconPath"].as_str()) {
            d.perk_icons.insert(id, format!("{CD}{}", path.replace("/lol-game-data/assets/", "").to_lowercase()));
        }
    }
    for x in c["data"].as_object().into_iter().flatten().map(|(_, x)| x) {
        d.champs.insert(
            to_i(x["key"].as_str().unwrap_or_default()),
            Champ {
                id: x["id"].as_str().unwrap_or_default().into(),
                name: x["name"].as_str().unwrap_or_default().into(),
                tags: strs(&x["tags"]),
                attack: x["info"]["attack"].as_f64().unwrap_or(0.0),
                magic: x["info"]["magic"].as_f64().unwrap_or(0.0),
            },
        );
    }
    for (k, x) in i["data"].as_object().into_iter().flatten() {
        d.items.insert(
            to_i(k),
            Item {
                name: x["name"].as_str().unwrap_or_default().into(),
                total: x["gold"]["total"].as_i64().unwrap_or(0),
                into: x["into"].as_array().is_some_and(|a| !a.is_empty()),
                tags: strs(&x["tags"]),
            },
        );
    }
    for x in s["data"].as_object().into_iter().flatten().map(|(_, x)| x) {
        d.spells.insert(to_i(x["key"].as_str().unwrap_or_default()), (x["id"].as_str().unwrap_or_default().into(), x["name"].as_str().unwrap_or_default().into()));
    }
    for x in st.as_array().into_iter().flatten() {
        let rune = rune_of(x);
        d.runes.insert(rune.id, rune.clone());
        let mut slots = vec![];
        for sl in x["slots"].as_array().into_iter().flatten() {
            let list: Vec<Rune> = sl["runes"].as_array().into_iter().flatten().map(rune_of).collect();
            for r in &list {
                d.runes.insert(r.id, r.clone());
            }
            slots.push(list);
        }
        d.styles.push(Style { rune, slots });
    }
    d
}

pub async fn ddragon() -> Result<Arc<DD>, String> {
    static DDC: OnceCell<Arc<DD>> = OnceCell::const_new();
    DDC.get_or_try_init(|| async {
        let vs = json_url("https://ddragon.leagueoflegends.com/api/versions.json").await?;
        let v = vs[0].as_str().ok_or("version Data Dragon absente")?.to_string();
        let file = |f: &str| format!("{CDN}/{v}/data/fr_FR/{f}.json");
        let (c, i, st, s, perks) = tokio::join!(json_url(&file("champion")), json_url(&file("item")), json_url(&file("runesReforged")), json_url(&file("summoner")), json_url(&format!("{CD}v1/perks.json")));
        Ok::<_, String>(Arc::new(parse_dd(v.clone(), &c?, &i?, &st?, &s?, &perks.unwrap_or(Value::Null))))
    })
    .await
    .cloned()
}

pub fn img_champ(d: &DD, id: i64) -> String {
    match d.champs.get(&id) {
        Some(c) => format!("{CDN}/{}/img/champion/{}.png", d.v, c.id),
        None => String::new(),
    }
}

pub fn img_item(d: &DD, id: i64) -> String {
    format!("{CDN}/{}/img/item/{id}.png", d.v)
}

pub fn img_spell(d: &DD, id: i64) -> String {
    match d.spells.get(&id) {
        Some(s) => format!("{CDN}/{}/img/spell/{}.png", d.v, s.0),
        None => String::new(),
    }
}

pub fn img_icon(d: &DD, id: i64) -> String {
    format!("{CDN}/{}/img/profileicon/{id}.png", d.v)
}

pub fn img_rune(d: &DD, id: i64) -> String {
    if let Some(p) = d.perk_icons.get(&id) {
        return p.clone();
    }
    match d.runes.get(&id) {
        Some(r) => format!("https://ddragon.leagueoflegends.com/cdn/img/{}", r.icon),
        None => String::new(),
    }
}

pub fn img_emblem(tier: &str) -> String {
    format!("https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/{}.svg", tier.to_lowercase())
}

pub fn img_role(pos: &str) -> String {
    let p = match pos {
        "adc" => "bottom",
        "mid" => "middle",
        "support" => "utility",
        x => x,
    };
    format!("https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-{p}.png")
}

pub fn img_ability(champ: &str, key: &str) -> String {
    format!("https://cdn.communitydragon.org/latest/champion/{champ}/ability-icon/{}", key.to_lowercase())
}

pub fn img_splash(champ: &str) -> String {
    format!("https://ddragon.leagueoflegends.com/cdn/img/champion/splash/{champ}_0.jpg")
}

pub fn img_loading(champ: &str) -> String {
    format!("https://ddragon.leagueoflegends.com/cdn/img/champion/loading/{champ}_0.jpg")
}

pub fn is_completed(d: &DD, id: i64) -> bool {
    d.items.get(&id).is_some_and(|it| !it.into && it.total >= 900 && !it.tags.iter().any(|t| t == "Consumable" || t == "Trinket"))
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Summoner {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub summoner_level: i64,
    pub profile_icon_id: i64,
    pub summoner_id: i64,
}

pub async fn me() -> Result<Summoner, String> {
    serde_json::from_value(get("/lol-summoner/v1/current-summoner").await?).map_err(|e| e.to_string())
}

pub async fn summoner(puuid: &str) -> Result<Summoner, String> {
    serde_json::from_value(get(&format!("/lol-summoner/v2/summoners/puuid/{puuid}")).await?).map_err(|e| e.to_string())
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub async fn find_player(riot_id: &str) -> Result<String, String> {
    let (name, tag) = riot_id.split_once('#').unwrap_or((riot_id, ""));
    let r = get(&format!("/lol-summoner/v1/alias/lookup?gameName={}&tagLine={}", encode(name.trim()), encode(tag.trim()))).await.unwrap_or(Value::Null);
    r["puuid"].as_str().filter(|p| !p.is_empty()).map(String::from).ok_or_else(|| "Joueur introuvable".to_string())
}

pub async fn history(puuid: &str, count: i64) -> Result<Vec<Game>, String> {
    let r = get(&format!("/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex={}", count - 1)).await?;
    serde_json::from_value(r["games"]["games"].clone()).map_err(|e| e.to_string())
}

pub async fn tft_history(puuid: &str, begin: usize, count: usize) -> Result<Value, String> {
    get(&format!("/lol-match-history/v1/products/tft/{puuid}/matches?begin={begin}&count={count}")).await
}

pub async fn game_details(id: i64) -> Result<Arc<Game>, String> {
    static CACHE: OnceLock<Mutex<HashMap<i64, Arc<Game>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(g) = cache.lock().unwrap().get(&id) {
        return Ok(g.clone());
    }
    let g: Game = serde_json::from_value(get(&format!("/lol-match-history/v1/games/{id}")).await?).map_err(|e| e.to_string())?;
    let g = Arc::new(g);
    cache.lock().unwrap().insert(id, g.clone());
    Ok(g)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Queue {
    pub queue_type: String,
    pub tier: String,
    pub division: String,
    pub league_points: i64,
    pub wins: i64,
    pub losses: i64,
}

pub async fn ranked(puuid: &str) -> Result<HashMap<String, Queue>, String> {
    serde_json::from_value(get(&format!("/lol-ranked/v1/ranked-stats/{puuid}")).await?["queueMap"].clone()).map_err(|e| e.to_string())
}

pub async fn queue_names() -> Result<Arc<HashMap<i64, String>>, String> {
    static CACHE: OnceCell<Arc<HashMap<i64, String>>> = OnceCell::const_new();
    CACHE
        .get_or_try_init(|| async {
            let qs = get("/lol-game-queues/v1/queues").await?;
            let mut out = HashMap::new();
            for q in qs.as_array().into_iter().flatten() {
                let d = q["description"].as_str().filter(|s| !s.is_empty()).or(q["name"].as_str()).unwrap_or_default();
                out.insert(q["id"].as_i64().unwrap_or(0), d.to_string());
            }
            Ok::<_, String>(Arc::new(out))
        })
        .await
        .cloned()
}

pub const TIERS: [&str; 10] = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND", "MASTER", "GRANDMASTER", "CHALLENGER"];
const DIVS: [&str; 4] = ["IV", "III", "II", "I"];

fn tier_index(t: &str) -> i64 {
    TIERS.iter().position(|x| *x == t).map(|x| x as i64).unwrap_or(-1)
}

pub fn rank_score(q: &Queue) -> i64 {
    let t = tier_index(&q.tier);
    let div = DIVS.iter().position(|x| *x == q.division).map(|x| x as i64).unwrap_or(-1);
    t * 400 + if t >= 7 { 0 } else { div * 100 } + q.league_points
}

pub fn title_tier(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_string() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

pub fn has_rank(q: Option<&Queue>) -> bool {
    q.is_some_and(|q| !q.tier.is_empty() && q.tier != "NONE" && q.tier != "UNRANKED")
}

pub fn rank_label(q: Option<&Queue>) -> String {
    match q {
        Some(q) if has_rank(Some(q)) => {
            let div = if tier_index(&q.tier) >= 7 { "" } else { q.division.as_str() };
            format!("{} {} {} LP", title_tier(&q.tier), div, q.league_points)
        }
        _ => "Non classé".into(),
    }
}

pub fn tier_color(t: &str) -> slint::Color {
    let hex = match t.to_lowercase().as_str() {
        "iron" | "bronze" => 0xc98b62,
        "silver" => 0xb9c4d3,
        "gold" => 0xe3b34c,
        "platinum" => 0x4fd1c5,
        "emerald" => 0x3ddc84,
        "diamond" => 0x7aa7ff,
        "master" | "grandmaster" | "challenger" => 0xc77dff,
        _ => 0xeef0f7,
    };
    slint::Color::from_argb_encoded(0xff000000 | hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks() {
        let q = Queue { tier: "GOLD".into(), division: "II".into(), league_points: 40, ..Default::default() };
        assert_eq!(rank_score(&q), 3 * 400 + 200 + 40);
        assert_eq!(rank_label(Some(&q)), "Gold II 40 LP");
        let m = Queue { tier: "MASTER".into(), division: "NA".into(), league_points: 12, ..Default::default() };
        assert_eq!(rank_score(&m), 7 * 400 + 12);
        assert_eq!(rank_label(Some(&m)), "Master  12 LP");
        assert_eq!(rank_label(None), "Non classé");
    }

    #[test]
    fn urls() {
        assert!(img_role("adc").ends_with("icon-position-bottom.png"));
        assert_eq!(encode("a b#é"), "a%20b%23%C3%A9");
    }
}
