use crate::net::{json_url, valo};
use crate::store;
use crate::text::now_ms;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::OnceCell;

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub name: String,
    pub icon: String,
    pub portrait: String,
    pub role: String,
    pub role_icon: String,
}

#[derive(Clone, Debug, Default)]
pub struct Tier {
    pub name: String,
    pub icon: String,
    pub color: String,
}

#[derive(Clone, Debug, Default)]
pub struct VD {
    pub agents: HashMap<String, Agent>,
    pub maps: HashMap<String, (String, String, String)>,
    pub tiers: HashMap<i64, Tier>,
    pub weapons: HashMap<String, (String, String)>,
}

fn s(x: &Value) -> String {
    x.as_str().unwrap_or_default().to_string()
}

fn title(t: &str) -> String {
    let mut out = String::new();
    let mut start = true;
    for c in t.to_lowercase().chars() {
        if start {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
        start = c.is_whitespace();
    }
    out
}

pub fn parse_vd(a: &Value, m: &Value, t: &Value, w: &Value) -> VD {
    let mut d = VD::default();
    for x in a.as_array().into_iter().flatten() {
        d.agents.insert(s(&x["uuid"]).to_lowercase(), Agent { name: s(&x["displayName"]), icon: s(&x["displayIcon"]), portrait: s(&x["fullPortrait"]), role: s(&x["role"]["displayName"]), role_icon: s(&x["role"]["displayIcon"]) });
    }
    for x in m.as_array().into_iter().flatten() {
        d.maps.insert(s(&x["mapUrl"]), (s(&x["displayName"]), s(&x["listViewIcon"]), s(&x["splash"])));
    }
    let last = t.as_array().and_then(|l| l.last()).cloned().unwrap_or(Value::Null);
    for x in last["tiers"].as_array().into_iter().flatten() {
        let color = s(&x["color"]);
        let icon = x["smallIcon"].as_str().or(x["largeIcon"].as_str()).unwrap_or_default().to_string();
        d.tiers.insert(x["tier"].as_i64().unwrap_or(0), Tier { name: title(&s(&x["tierName"])), icon, color: color.chars().take(6).collect() });
    }
    for x in w.as_array().into_iter().flatten() {
        let icon = x["killStreamIcon"].as_str().or(x["displayIcon"].as_str()).unwrap_or_default().to_string();
        d.weapons.insert(s(&x["uuid"]).to_lowercase(), (s(&x["displayName"]), icon));
    }
    d
}

pub async fn vdata() -> Result<Arc<VD>, String> {
    static CELL: OnceCell<Arc<VD>> = OnceCell::const_new();
    CELL.get_or_try_init(|| async {
        let get = |p: &'static str| async move { json_url(&format!("https://valorant-api.com/v1/{p}")).await.map(|j| j["data"].clone()) };
        let (a, m, t, w) = tokio::join!(get("agents?isPlayableCharacter=true&language=fr-FR"), get("maps?language=fr-FR"), get("competitivetiers?language=fr-FR"), get("weapons?language=fr-FR"));
        Ok::<_, String>(Arc::new(parse_vd(&a?, &m?, &t?, &w.unwrap_or(Value::Null))))
    })
    .await
    .cloned()
}

pub fn queue_label(q: &str) -> String {
    match q {
        "competitive" => "Compétition",
        "unrated" => "Non classé",
        "swiftplay" => "Vélocité",
        "spikerush" => "Spike Rush",
        "deathmatch" => "Combat à mort",
        "hurm" => "Combat à mort en équipe",
        "ggteam" => "Escalade",
        "onefa" => "Réplication",
        "premier" => "Premier",
        "newmap" => "Nouvelle carte",
        "" => "Personnalisée",
        x => return x.to_string(),
    }
    .to_string()
}

async fn current_act() -> Result<String, String> {
    static CELL: OnceCell<String> = OnceCell::const_new();
    CELL.get_or_try_init(|| async {
        let c = valo("shared", "GET", "/content-service/v3/content", None).await?;
        Ok::<_, String>(c["Seasons"].as_array().into_iter().flatten().find(|x| x["Type"] == "act" && x["IsActive"] == true).map(|x| s(&x["ID"])).unwrap_or_default())
    })
    .await
    .cloned()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VRank {
    pub tier: i64,
    pub rr: i64,
    pub wins: i64,
    pub games: i64,
    pub peak: i64,
}

pub fn parse_rank(r: &Value, act: &str) -> VRank {
    let seasons = &r["QueueSkills"]["competitive"]["SeasonalInfoBySeasonID"];
    let cur = &seasons[act];
    let mut peak = 0;
    for x in seasons.as_object().into_iter().flatten().map(|(_, x)| x) {
        for k in x["WinsByTier"].as_object().into_iter().flatten().map(|(k, _)| k) {
            peak = peak.max(k.parse::<i64>().unwrap_or(0));
        }
    }
    let int = |k: &str| cur[k].as_i64().unwrap_or(0);
    VRank { tier: int("CompetitiveTier"), rr: int("RankedRating"), wins: int("NumberOfWinsWithPlacements"), games: int("NumberOfGames"), peak }
}

static RANKS: OnceLock<Mutex<HashMap<String, VRank>>> = OnceLock::new();

pub async fn v_rank(puuid: &str) -> VRank {
    let cache = RANKS.get_or_init(Default::default);
    if let Some(r) = cache.lock().unwrap().get(puuid) {
        return *r;
    }
    let (r, a) = tokio::join!(valo("pd", "GET", &format!("/mmr/v1/players/{puuid}"), None), current_act());
    let out = match (r, a) {
        (Ok(r), Ok(a)) => parse_rank(&r, &a),
        _ => VRank::default(),
    };
    cache.lock().unwrap().insert(puuid.to_string(), out);
    out
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VName {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "GameName")]
    pub game_name: String,
    #[serde(rename = "TagLine")]
    pub tag_line: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Seen {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub t: i64,
}

pub fn seen_players() -> Vec<Seen> {
    store::load("rw-vseen").unwrap_or_default()
}

fn remember(list: &[VName]) {
    let fresh: Vec<Seen> = list.iter().filter(|n| !n.game_name.is_empty()).map(|n| Seen { puuid: n.subject.clone(), name: n.game_name.clone(), tag: n.tag_line.clone(), t: now_ms() }).collect();
    if fresh.is_empty() {
        return;
    }
    let ids: HashSet<&str> = fresh.iter().map(|x| x.puuid.as_str()).collect();
    let mut all: Vec<Seen> = fresh.clone();
    all.extend(seen_players().into_iter().filter(|x| !ids.contains(x.puuid.as_str())));
    all.truncate(400);
    store::save("rw-vseen", &all);
}

pub async fn v_names(puuids: &[String]) -> Result<Vec<VName>, String> {
    let r = valo("pd", "PUT", "/name-service/v2/players", Some(json!(puuids))).await?;
    let list: Vec<VName> = serde_json::from_value(r).map_err(|e| e.to_string())?;
    remember(&list);
    Ok(list)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VStatsRaw {
    pub score: i64,
    #[serde(rename = "roundsPlayed")]
    pub rounds_played: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VPlayer {
    pub subject: String,
    pub game_name: String,
    pub tag_line: String,
    pub team_id: String,
    pub character_id: String,
    pub competitive_tier: i64,
    pub account_level: i64,
    pub stats: Option<VStatsRaw>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VInfo {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "mapId")]
    pub map_id: String,
    #[serde(rename = "gameLengthMillis")]
    pub game_length_millis: Option<i64>,
    #[serde(rename = "gameStartMillis")]
    pub game_start_millis: i64,
    #[serde(rename = "queueID")]
    pub queue_id: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VTeam {
    pub team_id: String,
    pub won: bool,
    pub rounds_won: i64,
    pub rounds_played: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VFinish {
    pub damage_type: String,
    pub damage_item: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VKill {
    pub killer: String,
    pub victim: String,
    pub round_time: i64,
    pub finishing_damage: Option<VFinish>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VDamage {
    pub damage: i64,
    pub headshots: i64,
    pub bodyshots: i64,
    pub legshots: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VEcon {
    pub subject: String,
    pub loadout_value: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VPlayerRound {
    pub subject: String,
    pub kills: Option<Vec<VKill>>,
    pub damage: Option<Vec<VDamage>>,
    pub economy: Option<VEcon>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VRound {
    pub round_num: i64,
    pub winning_team: String,
    pub round_result: String,
    pub plant_round_time: Option<i64>,
    pub bomb_planter: Option<String>,
    pub plant_site: Option<String>,
    pub defuse_round_time: Option<i64>,
    pub bomb_defuser: Option<String>,
    pub player_stats: Vec<VPlayerRound>,
    pub player_economies: Option<Vec<VEcon>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VMatch {
    pub match_info: VInfo,
    pub players: Vec<VPlayer>,
    pub teams: Option<Vec<VTeam>>,
    pub round_results: Option<Vec<VRound>>,
}

pub async fn v_history(puuid: &str, count: usize, queue: &str, start: usize) -> Result<Vec<String>, String> {
    let q = if queue.is_empty() { String::new() } else { format!("&queue={queue}") };
    let r = valo("pd", "GET", &format!("/match-history/v1/history/{puuid}?startIndex={start}&endIndex={}{q}", start + count), None).await?;
    Ok(r["History"].as_array().into_iter().flatten().map(|x| s(&x["MatchID"])).collect())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VRR {
    pub earned: i64,
    pub tier_before: i64,
    pub tier_after: i64,
    pub placement: bool,
}

pub async fn v_rr_updates(puuid: &str) -> Vec<(String, VRR)> {
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<(String, VRR)>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(r) = cache.lock().unwrap().get(puuid) {
        return r.clone();
    }
    let r = valo("pd", "GET", &format!("/mmr/v1/players/{puuid}/competitiveupdates?startIndex=0&endIndex=20&queue=competitive"), None).await.unwrap_or(Value::Null);
    let out: Vec<(String, VRR)> = r["Matches"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|x| {
            let i = |k: &str| x[k].as_i64().unwrap_or(0);
            (s(&x["MatchID"]), VRR { earned: i("RankedRatingEarned"), tier_before: i("TierBeforeUpdate"), tier_after: i("TierAfterUpdate"), placement: x["IsPlacementMatch"] == true })
        })
        .collect();
    cache.lock().unwrap().insert(puuid.to_string(), out.clone());
    out
}

pub async fn v_match(id: &str) -> Result<Arc<VMatch>, String> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<VMatch>>>> = OnceLock::new();
    static CHAIN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().unwrap().get(id) {
        return Ok(m.clone());
    }
    let _turn = CHAIN.lock().await;
    if let Some(m) = cache.lock().unwrap().get(id) {
        return Ok(m.clone());
    }
    let mut tries = 3;
    loop {
        match valo("pd", "GET", &format!("/match-details/v1/matches/{id}"), None).await {
            Ok(v) => {
                let m = Arc::new(serde_json::from_value::<VMatch>(v).map_err(|e| e.to_string())?);
                cache.lock().unwrap().insert(id.to_string(), m.clone());
                return Ok(m);
            }
            Err(e) => {
                let limited = e.contains("429") || e.contains("Too Many") || e.to_lowercase().contains("rate");
                if tries > 0 && limited {
                    tokio::time::sleep(std::time::Duration::from_millis((4 - tries) * 2500)).await;
                    tries -= 1;
                } else {
                    return Err(e);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VStat {
    pub acs: f64,
    pub adr: f64,
    pub hs: f64,
    pub fk: i64,
    pub fd: i64,
    pub kd: f64,
    pub mk: [i64; 6],
}

#[derive(Default)]
struct Acc {
    dmg: i64,
    head: i64,
    shots: i64,
    fk: i64,
    fd: i64,
    mk: [i64; 6],
}

pub fn v_stats(m: &VMatch) -> HashMap<String, VStat> {
    let mut acc: HashMap<String, Acc> = HashMap::new();
    for r in m.round_results.iter().flatten() {
        let kills: Vec<&VKill> = r.player_stats.iter().flat_map(|p| p.kills.iter().flatten()).collect();
        if let Some(first) = kills.iter().min_by_key(|k| k.round_time) {
            acc.entry(first.killer.clone()).or_default().fk += 1;
            acc.entry(first.victim.clone()).or_default().fd += 1;
        }
        let mut per: HashMap<&str, i64> = HashMap::new();
        for k in &kills {
            *per.entry(k.killer.as_str()).or_default() += 1;
        }
        for (who, c) in per {
            if c >= 3 {
                acc.entry(who.to_string()).or_default().mk[c.min(5) as usize] += 1;
            }
        }
        for p in &r.player_stats {
            for d in p.damage.iter().flatten() {
                let a = acc.entry(p.subject.clone()).or_default();
                a.dmg += d.damage;
                a.head += d.headshots;
                a.shots += d.headshots + d.bodyshots + d.legshots;
            }
        }
    }
    let mut out = HashMap::new();
    for p in &m.players {
        let a = acc.remove(&p.subject).unwrap_or_default();
        let st = p.stats.clone().unwrap_or_default();
        let rounds = st.rounds_played.max(1) as f64;
        out.insert(
            p.subject.clone(),
            VStat {
                acs: st.score as f64 / rounds,
                adr: a.dmg as f64 / rounds,
                hs: if a.shots > 0 { a.head as f64 / a.shots as f64 } else { 0.0 },
                fk: a.fk,
                fd: a.fd,
                mk: a.mk,
                kd: st.kills as f64 / st.deaths.max(1) as f64,
            },
        );
    }
    out
}

pub fn v_team_won(m: &VMatch, team: &str) -> bool {
    m.teams.iter().flatten().find(|t| t.team_id == team).is_some_and(|t| t.won)
}

pub fn v_score(m: &VMatch, team: &str) -> String {
    let list = m.teams.as_deref().unwrap_or(&[]);
    match (list.iter().find(|t| t.team_id == team), list.iter().find(|t| t.team_id != team)) {
        (Some(a), Some(b)) => format!("{} - {}", a.rounds_won, b.rounds_won),
        _ => String::new(),
    }
}

fn decode(b64: &str) -> Option<Value> {
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    serde_json::from_slice(&raw).ok()
}

pub async fn v_parties(puuids: &[String], self_id: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let in_match: HashSet<&str> = puuids.iter().map(String::as_str).collect();
    let pres = valo("local", "GET", "/chat/v4/presences", None).await.unwrap_or(Value::Null);
    for p in pres["presences"].as_array().into_iter().flatten() {
        let (puuid, private) = (s(&p["puuid"]), s(&p["private"]));
        if p["product"] != "valorant" || !in_match.contains(puuid.as_str()) || private.is_empty() {
            continue;
        }
        if let Some(j) = decode(&private) {
            let id = j["partyPresenceData"]["partyId"].as_str().or(j["partyId"].as_str());
            if let Some(id) = id.filter(|x| !x.is_empty()) {
                out.insert(puuid, id.to_string());
            }
        }
    }
    if let Ok(mine) = valo("glz", "GET", &format!("/parties/v1/players/{self_id}"), None).await {
        let party_id = s(&mine["CurrentPartyID"]);
        if !party_id.is_empty() {
            if let Ok(party) = valo("glz", "GET", &format!("/parties/v1/parties/{party_id}"), None).await {
                for m in party["Members"].as_array().into_iter().flatten() {
                    let sub = s(&m["Subject"]);
                    if in_match.contains(sub.as_str()) {
                        out.insert(sub, party_id.clone());
                    }
                }
            }
        }
    }
    out
}

pub const PARTY_LABEL: [&str; 6] = ["", "", "Duo", "Trio", "Quatuor", "Five stack"];

pub fn phase_of(presences: &Value, puuid: &str) -> (bool, String) {
    let p = presences["presences"].as_array().into_iter().flatten().find(|x| x["puuid"] == puuid && x["product"] == "valorant");
    let Some(p) = p else { return (false, "MENUS".into()) };
    let private = s(&p["private"]);
    if private.is_empty() {
        return (true, "MENUS".into());
    }
    let phase = decode(&private).map(|j| j["matchPresenceData"]["sessionLoopState"].as_str().or(j["sessionLoopState"].as_str()).unwrap_or("MENUS").to_string());
    (true, phase.unwrap_or_else(|| "MENUS".into()))
}

pub async fn v_presence(puuid: &str) -> Result<(bool, String), String> {
    Ok(phase_of(&valo("local", "GET", "/chat/v4/presences", None).await?, puuid))
}

pub fn mk_label(mk: &[i64; 6]) -> Vec<String> {
    [(5, "ACE"), (4, "4K"), (3, "3K")]
        .iter()
        .filter(|(n, _)| mk[*n] > 0)
        .map(|(n, l)| if mk[*n] > 1 { format!("{l} ×{}", mk[*n]) } else { l.to_string() })
        .collect()
}

pub fn v_weapon_kills(m: &VMatch, puuid: &str) -> HashMap<String, i64> {
    let mut out = HashMap::new();
    for r in m.round_results.iter().flatten() {
        for p in &r.player_stats {
            for k in p.kills.iter().flatten() {
                if k.killer != puuid {
                    continue;
                }
                let key = match &k.finishing_damage {
                    Some(f) if f.damage_type == "Weapon" => f.damage_item.to_lowercase(),
                    Some(f) if f.damage_type == "Melee" => "melee".to_string(),
                    _ => "ability".to_string(),
                };
                *out.entry(key).or_default() += 1;
            }
        }
    }
    out
}

pub fn v_economy(m: &VMatch, team: &str) -> Vec<(i64, &'static str)> {
    let members: HashSet<&str> = m.players.iter().filter(|p| p.team_id == team).map(|p| p.subject.as_str()).collect();
    m.round_results
        .iter()
        .flatten()
        .map(|r| {
            let v: i64 = match &r.player_economies {
                Some(e) => e.iter().filter(|x| members.contains(x.subject.as_str())).map(|x| x.loadout_value).sum(),
                None => r.player_stats.iter().filter(|p| members.contains(p.subject.as_str())).map(|p| p.economy.as_ref().map(|e| e.loadout_value).unwrap_or(0)).sum(),
            };
            let kind = if r.round_num == 0 || r.round_num == 12 {
                "Pistolet"
            } else if v < 5000 {
                "Éco"
            } else if v < 10000 {
                "Semi-éco"
            } else if v < 20000 {
                "Semi"
            } else {
                "Full"
            };
            (v, kind)
        })
        .collect()
}

pub fn hex_color(h: &str) -> slint::Color {
    slint::Color::from_argb_encoded(0xff000000 | u32::from_str_radix(h, 16).unwrap_or(0xeef0f7))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VMatch {
        serde_json::from_value(json!({
            "matchInfo": {"matchId": "m", "mapId": "/m", "queueID": "competitive", "gameStartMillis": 5},
            "players": [
                {"subject": "a", "teamId": "Blue", "characterId": "X", "stats": {"score": 400, "roundsPlayed": 2, "kills": 4, "deaths": 1, "assists": 0}},
                {"subject": "b", "teamId": "Red", "characterId": "Y", "stats": {"score": 100, "roundsPlayed": 2, "kills": 1, "deaths": 4, "assists": 0}}
            ],
            "teams": [{"teamId": "Blue", "won": true, "roundsWon": 2, "roundsPlayed": 2}, {"teamId": "Red", "won": false, "roundsWon": 0, "roundsPlayed": 2}],
            "roundResults": [
                {"roundNum": 0, "winningTeam": "Blue", "roundResult": "Eliminated", "playerStats": [
                    {"subject": "a", "kills": [
                        {"killer": "a", "victim": "b", "roundTime": 5, "finishingDamage": {"damageType": "Weapon", "damageItem": "ABC"}},
                        {"killer": "a", "victim": "c", "roundTime": 6, "finishingDamage": {"damageType": "Melee", "damageItem": ""}},
                        {"killer": "a", "victim": "d", "roundTime": 7, "finishingDamage": {"damageType": "Ability", "damageItem": "x"}}
                    ], "damage": [{"damage": 200, "headshots": 1, "bodyshots": 3, "legshots": 0}], "economy": {"loadoutValue": 800}},
                    {"subject": "b", "kills": [], "damage": [], "economy": {"loadoutValue": 900}}
                ]},
                {"roundNum": 1, "winningTeam": "Blue", "roundResult": "Eliminated", "playerEconomies": [{"subject": "a", "loadoutValue": 25000}], "playerStats": []}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn stats() {
        let m = sample();
        let st = v_stats(&m);
        assert_eq!(st["a"].acs, 200.0);
        assert_eq!(st["a"].adr, 100.0);
        assert_eq!(st["a"].hs, 0.25);
        assert_eq!(st["a"].fk, 1);
        assert_eq!(st["b"].fd, 1);
        assert_eq!(st["a"].mk[3], 1);
        assert_eq!(st["a"].kd, 4.0);
        assert_eq!(mk_label(&st["a"].mk), vec!["3K"]);
        assert!(v_team_won(&m, "Blue"));
        assert_eq!(v_score(&m, "Red"), "0 - 2");
    }

    #[test]
    fn economy_weapons() {
        let m = sample();
        let e = v_economy(&m, "Blue");
        assert_eq!(e, vec![(800, "Pistolet"), (25000, "Full")]);
        let w = v_weapon_kills(&m, "a");
        assert_eq!((w["abc"], w["melee"], w["ability"]), (1, 1, 1));
    }

    #[test]
    fn presence() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(r#"{"matchPresenceData":{"sessionLoopState":"PREGAME"}}"#);
        let v = json!({"presences": [{"puuid": "x", "product": "valorant", "private": b64}, {"puuid": "y", "product": "league_of_legends", "private": ""}]});
        assert_eq!(phase_of(&v, "x"), (true, "PREGAME".to_string()));
        assert_eq!(phase_of(&v, "y"), (false, "MENUS".to_string()));
    }

    #[test]
    fn rank() {
        let r = json!({"QueueSkills": {"competitive": {"SeasonalInfoBySeasonID": {"act": {"CompetitiveTier": 15, "RankedRating": 40, "NumberOfWinsWithPlacements": 3, "NumberOfGames": 6, "WinsByTier": {"12": 1, "15": 2}}, "old": {"WinsByTier": {"18": 1}}}}}});
        let x = parse_rank(&r, "act");
        assert_eq!((x.tier, x.rr, x.wins, x.games, x.peak), (15, 40, 3, 6, 18));
        assert_eq!(title("GOLD 1"), "Gold 1");
    }
}
