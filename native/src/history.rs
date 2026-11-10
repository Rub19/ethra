use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const REMAKE_SECONDS: i64 = 300;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Participant {
    pub participant_id: i64,
    pub team_id: i64,
    pub champion_id: i64,
    pub spell1_id: i64,
    pub spell2_id: i64,
    pub stats: Map<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Player {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub summoner_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Identity {
    pub participant_id: i64,
    pub player: Player,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Game {
    pub game_id: i64,
    pub game_creation: i64,
    pub game_duration: i64,
    pub queue_id: i64,
    pub map_id: i64,
    pub participants: Vec<Participant>,
    pub participant_identities: Vec<Identity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub teams: Vec<Value>,
}

impl Game {
    pub fn me(&self) -> &Participant {
        static NONE: std::sync::OnceLock<Participant> = std::sync::OnceLock::new();
        self.participants.first().unwrap_or_else(|| NONE.get_or_init(Participant::default))
    }

    pub fn find(&self, puuid: &str) -> Option<&Participant> {
        let id = self.participant_identities.iter().find(|x| x.player.puuid == puuid)?.participant_id;
        self.participants.iter().find(|x| x.participant_id == id)
    }

    pub fn who(&self, participant: i64) -> Option<&Player> {
        self.participant_identities.iter().find(|x| x.participant_id == participant).map(|x| &x.player)
    }
}

pub fn n(s: &Map<String, Value>, k: &str) -> f64 {
    match s.get(k) {
        Some(Value::Number(v)) => v.as_f64().unwrap_or(0.0),
        Some(Value::Bool(true)) => 1.0,
        _ => 0.0,
    }
}

pub fn win(p: &Participant) -> bool {
    p.stats.get("win").and_then(Value::as_bool).unwrap_or(false)
}

pub fn kda(s: &Map<String, Value>) -> f64 {
    (n(s, "kills") + n(s, "assists")) / n(s, "deaths").max(1.0)
}

pub fn cs(s: &Map<String, Value>) -> f64 {
    n(s, "totalMinionsKilled") + n(s, "neutralMinionsKilled")
}

pub fn is_remake(g: &Game) -> bool {
    g.game_duration < REMAKE_SECONDS
}

pub fn counted(games: &[Game]) -> Vec<&Game> {
    games.iter().filter(|g| !is_remake(g)).collect()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryFilter {
    pub result: String,
    pub queue: i64,
    pub champ: i64,
}

pub fn filter_games(games: &[Game], f: &HistoryFilter) -> Vec<Game> {
    games
        .iter()
        .filter(|g| {
            let p = g.me();
            if f.queue != 0 && g.queue_id != f.queue {
                return false;
            }
            if f.champ != 0 && p.champion_id != f.champ {
                return false;
            }
            if !f.result.is_empty() && (is_remake(g) || (f.result == "win") != win(p)) {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

pub fn merge_games(fresh: Vec<Game>, kept: Vec<Game>, limit: usize) -> Vec<Game> {
    let known: HashSet<i64> = fresh.iter().map(|g| g.game_id).collect();
    let mut all = fresh;
    all.extend(kept.into_iter().filter(|g| !known.contains(&g.game_id)));
    all.sort_by_key(|g| std::cmp::Reverse(g.game_creation));
    all.truncate(limit);
    all
}

fn distinct(values: impl Iterator<Item = i64>) -> Vec<i64> {
    let mut seen = HashSet::new();
    values.filter(|v| seen.insert(*v)).collect()
}

pub fn queues_in(games: &[Game]) -> Vec<i64> {
    distinct(games.iter().map(|g| g.queue_id))
}

pub fn champs_in(games: &[Game]) -> Vec<i64> {
    distinct(games.iter().map(|g| g.me().champion_id))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Form {
    pub streak: Option<(bool, usize)>,
    pub strip: Vec<bool>,
    pub kda_now: Option<f64>,
    pub kda_before: Option<f64>,
}

pub fn form_of(games: &[Game], kda_of: impl Fn(&Game) -> f64) -> Form {
    let list = counted(games);
    let avg = |part: &[&Game]| part.iter().map(|g| kda_of(g)).sum::<f64>() / part.len() as f64;
    let first = list.first().map(|g| win(g.me()));
    let streak = first.map(|w| (w, list.iter().take_while(|g| win(g.me()) == w).count()));
    Form {
        streak,
        strip: list.iter().take(20).rev().map(|g| win(g.me())).collect(),
        kda_now: (list.len() >= 10).then(|| avg(&list[..5])),
        kda_before: (list.len() >= 10).then(|| avg(&list[5..10])),
    }
}

pub fn games_csv(games: &[Game], champ: impl Fn(i64) -> String, queue: impl Fn(i64) -> String, date: impl Fn(i64) -> String) -> String {
    let esc = |s: String| if s.contains([';', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s };
    let mut out = String::from("\u{feff}Date;Mode;Champion;Résultat;Kills;Morts;Assists;CS;Dégâts;Or;Vision;Durée (s)\r\n");
    for g in games {
        let p = g.me();
        let s = &p.stats;
        let result = if is_remake(g) { "Remake" } else if win(p) { "Victoire" } else { "Défaite" };
        let line = [esc(date(g.game_creation)), esc(queue(g.queue_id)), esc(champ(p.champion_id)), result.to_string(), n(s, "kills").to_string(), n(s, "deaths").to_string(), n(s, "assists").to_string(), cs(s).to_string(), n(s, "totalDamageDealtToChampions").to_string(), n(s, "goldEarned").to_string(), n(s, "visionScore").to_string(), g.game_duration.to_string()].join(";");
        out += &line;
        out += "\r\n";
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub games: usize,
    pub wins: usize,
    pub kda: f64,
}

pub fn by_queue(games: &[Game]) -> Vec<(i64, Tally)> {
    let mut out: Vec<(i64, Tally)> = vec![];
    for g in counted(games) {
        let p = g.me();
        let i = out.iter().position(|x| x.0 == g.queue_id).unwrap_or_else(|| {
            out.push((g.queue_id, Tally::default()));
            out.len() - 1
        });
        let t = &mut out[i].1;
        t.games += 1;
        t.wins += win(p) as usize;
        t.kda += kda(&p.stats);
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.1.games));
    out
}

pub fn tft_top(games: &[TftGame], keys: impl Fn(&TftGame) -> Vec<String>) -> Vec<(String, usize, f64)> {
    let mut out: Vec<(String, usize, f64)> = vec![];
    for g in games {
        let mut seen: Vec<String> = keys(g);
        seen.sort();
        seen.dedup();
        for k in seen {
            match out.iter_mut().find(|x| x.0 == k) {
                Some(x) => {
                    x.1 += 1;
                    x.2 += g.placement as f64;
                }
                None => out.push((k, 1, g.placement as f64)),
            }
        }
    }
    for x in &mut out {
        x.2 /= x.1 as f64;
    }
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.total_cmp(&b.2)));
    out
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TftUnit {
    #[serde(rename = "characterId")]
    pub character_id: String,
    pub tier: i64,
    pub rarity: i64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct TftTrait {
    pub name: String,
    pub num_units: i64,
    pub style: i64,
}

#[derive(Clone, Debug, Default)]
pub struct TftGame {
    pub id: String,
    pub at: i64,
    pub seconds: f64,
    pub queue_id: i64,
    pub placement: i64,
    pub level: i64,
    pub units: Vec<TftUnit>,
    pub traits: Vec<TftTrait>,
}

pub fn parse_tft_games(raw: &Value, puuid: &str) -> Vec<TftGame> {
    let mut out = vec![];
    for g in raw["games"].as_array().into_iter().flatten() {
        let info = &g["json"];
        let Some(me) = info["participants"].as_array().and_then(|l| l.iter().find(|p| p["puuid"] == puuid)) else { continue };
        let at = info["game_datetime"].as_i64().unwrap_or(0);
        let units: Vec<TftUnit> = serde_json::from_value(me["units"].clone()).unwrap_or_default();
        let traits: Vec<TftTrait> = serde_json::from_value(me["traits"].clone()).unwrap_or_default();
        out.push(TftGame {
            id: g["metadata"]["match_id"].as_str().map(String::from).unwrap_or_else(|| at.to_string()),
            at,
            seconds: info["game_length"].as_f64().unwrap_or(0.0),
            queue_id: info["queue_id"].as_i64().unwrap_or(0),
            placement: me["placement"].as_i64().unwrap_or(0),
            level: me["level"].as_i64().unwrap_or(0),
            units,
            traits: traits.into_iter().filter(|t| t.style > 0).collect(),
        });
    }
    out.sort_by_key(|g| std::cmp::Reverse(g.at));
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TftSummary {
    pub games: usize,
    pub average: f64,
    pub top4: f64,
    pub wins: usize,
}

pub fn tft_summary(games: &[TftGame]) -> TftSummary {
    if games.is_empty() {
        return TftSummary::default();
    }
    let len = games.len() as f64;
    TftSummary {
        games: games.len(),
        average: games.iter().map(|g| g.placement as f64).sum::<f64>() / len,
        top4: games.iter().filter(|g| g.placement <= 4).count() as f64 / len,
        wins: games.iter().filter(|g| g.placement == 1).count(),
    }
}

pub fn tft_queue(id: i64) -> &'static str {
    match id {
        1090 => "Normale",
        1100 => "Classée",
        1130 => "Hyper Roll",
        1160 => "Double Up",
        1170 => "Fortune's Favor",
        1180 => "Soul Brawl",
        _ => "TFT",
    }
}

pub fn streak<I: IntoIterator<Item = bool>>(results: I) -> Option<(bool, usize)> {
    let mut list = results.into_iter();
    let first = list.next()?;
    Some((first, 1 + list.take_while(|result| *result == first).count()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streak_counts_the_latest_run() {
        assert_eq!(streak([true, true, false, true]), Some((true, 2)));
        assert_eq!(streak([false, false, false]), Some((false, 3)));
        assert_eq!(streak([false, true]), Some((false, 1)));
        assert_eq!(streak(std::iter::empty()), None);
    }
    use serde_json::json;

    fn game(id: i64, queue: i64, champ: i64, won: bool, duration: i64) -> Game {
        serde_json::from_value(json!({"gameId": id, "queueId": queue, "gameDuration": duration, "participants": [{"championId": champ, "stats": {"win": won}}]})).unwrap()
    }

    fn games() -> Vec<Game> {
        vec![game(1, 420, 7, true, 1500), game(2, 420, 8, false, 1500), game(3, 450, 7, false, 1500), game(4, 420, 7, false, 200)]
    }

    fn ids(list: &[Game]) -> Vec<i64> {
        list.iter().map(|g| g.game_id).collect()
    }

    #[test]
    fn form() {
        let series: Vec<Game> = [true, true, false, false, false, false, false, false, false, false, true].iter().enumerate().map(|(i, w)| game(100 + i as i64, 420, 7, *w, 1500)).collect();
        let mut all = vec![game(99, 420, 7, false, 100)];
        all.extend(series.clone());
        let f = form_of(&all, |g| g.game_id as f64);
        assert_eq!(f.streak, Some((true, 2)));
        assert_eq!(f.strip.len(), 11);
        assert!(f.strip[10]);
        assert_eq!(f.kda_now, Some(102.0));
        assert_eq!(f.kda_before, Some(107.0));
        assert_eq!(form_of(&[], |_| 0.0).streak, None);
        assert_eq!(form_of(&series[..5], |_| 1.0).kda_now, None);
    }

    #[test]
    fn filters() {
        let g = games();
        assert_eq!(counted(&g).iter().map(|x| x.game_id).collect::<Vec<_>>(), vec![1, 2, 3]);
        let f = |result: &str, queue, champ| ids(&filter_games(&g, &HistoryFilter { result: result.into(), queue, champ }));
        assert_eq!(f("", 0, 0), vec![1, 2, 3, 4]);
        assert_eq!(f("win", 0, 0), vec![1]);
        assert_eq!(f("loss", 0, 0), vec![2, 3]);
        assert_eq!(f("", 450, 0), vec![3]);
        assert_eq!(f("loss", 420, 8), vec![2]);
        assert_eq!(queues_in(&g), vec![420, 450]);
        assert_eq!(champs_in(&g), vec![7, 8]);
    }

    #[test]
    fn merge() {
        let old = |id: i64, at: i64, map: i64| Game { game_id: id, game_creation: at, map_id: map, ..Default::default() };
        let m = merge_games(vec![old(3, 30, 1), old(2, 20, 1)], vec![old(2, 20, 2), old(1, 10, 2)], 10);
        assert_eq!(m.iter().map(|g| (g.game_id, g.map_id)).collect::<Vec<_>>(), vec![(3, 1), (2, 1), (1, 2)]);
        assert_eq!(ids(&merge_games(vec![old(3, 30, 0)], vec![old(2, 20, 0), old(1, 10, 0)], 2)), vec![3, 2]);
        assert!(merge_games(vec![], vec![], 5).is_empty());
    }

    #[test]
    fn extras() {
        let g = games();
        let q = by_queue(&g);
        assert_eq!(q[0].0, 420);
        assert_eq!((q[0].1.games, q[0].1.wins), (2, 1));
        let csv = games_csv(&g[..1], |_| "Kai;Sa".into(), |_| "Solo".into(), |_| "1 nov".into());
        assert!(csv.contains("\"Kai;Sa\""));
        assert!(csv.contains("Victoire"));
        let t = vec![TftGame { placement: 1, traits: vec![TftTrait { name: "A".into(), num_units: 2, style: 1 }], ..Default::default() }, TftGame { placement: 5, traits: vec![TftTrait { name: "A".into(), num_units: 2, style: 1 }], ..Default::default() }];
        let top = tft_top(&t, |g| g.traits.iter().map(|x| x.name.clone()).collect());
        assert_eq!(top, vec![("A".to_string(), 2, 3.0)]);
    }

    #[test]
    fn tft() {
        let raw = json!({"games": [
            {"metadata": {"match_id": "A"}, "json": {"game_datetime": 100, "game_length": 1800, "queue_id": 1100, "participants": [{"puuid": "me", "placement": 3, "level": 8, "units": [{"characterId": "TFT_X", "tier": 2, "rarity": 1}], "traits": [{"name": "T1", "num_units": 3, "style": 1}, {"name": "T2", "num_units": 1, "style": 0}]}, {"puuid": "other", "placement": 1}]}},
            {"metadata": {"match_id": "B"}, "json": {"game_datetime": 200, "game_length": 1500, "queue_id": 1090, "participants": [{"puuid": "me", "placement": 1, "level": 9}]}},
            {"metadata": {"match_id": "C"}, "json": {"game_datetime": 300, "participants": [{"puuid": "other", "placement": 2}]}},
            {"broken": true}
        ]});
        let t = parse_tft_games(&raw, "me");
        assert_eq!(t.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(), vec!["B", "A"]);
        assert_eq!(t[1].traits.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), vec!["T1"]);
        assert!(t[0].units.is_empty());
        assert!(parse_tft_games(&Value::Null, "me").is_empty());
        let s = tft_summary(&t);
        assert_eq!((s.games, s.average, s.top4, s.wins), (2, 2.0, 1.0, 1));
        assert_eq!(tft_summary(&[]).average, 0.0);
        assert_eq!(tft_queue(1100), "Classée");
        assert_eq!(tft_queue(9), "TFT");
    }
}
