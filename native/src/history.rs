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
    pub since: i64,
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
            if f.since != 0 && g.game_creation < f.since {
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

pub struct Slot {
    pub hour: u32,
    pub weekday: u32,
}

pub fn versus(mine: &[&Game], theirs: &[&Game], champ: &dyn Fn(i64) -> String) -> Vec<(String, String, String)> {
    if mine.is_empty() || theirs.is_empty() {
        return Vec::new();
    }
    let rate = |l: &[&Game]| format!("{} %", (l.iter().filter(|g| win(g.me())).count() as f64 / l.len() as f64 * 100.0).round());
    let kda_of = |l: &[&Game]| format!("{:.2}", l.iter().map(|g| kda(&g.me().stats)).sum::<f64>() / l.len() as f64);
    let cs_of = |l: &[&Game]| format!("{:.1}", l.iter().map(|g| cs(&g.me().stats) / (g.game_duration as f64 / 60.0).max(1.0)).sum::<f64>() / l.len() as f64);
    let fav = |l: &[&Game]| {
        let mut counts: Vec<(i64, usize)> = Vec::new();
        for g in l {
            match counts.iter_mut().find(|c| c.0 == g.me().champion_id) {
                Some(c) => c.1 += 1,
                None => counts.push((g.me().champion_id, 1)),
            }
        }
        counts.into_iter().max_by_key(|c| c.1).map(|c| champ(c.0)).unwrap_or_default()
    };
    vec![
        ("Parties".to_string(), mine.len().to_string(), theirs.len().to_string()),
        ("Taux de victoire".to_string(), rate(mine), rate(theirs)),
        ("KDA moyen".to_string(), kda_of(mine), kda_of(theirs)),
        ("CS par minute".to_string(), cs_of(mine), cs_of(theirs)),
        ("Champion favori".to_string(), fav(mine), fav(theirs)),
    ]
}

pub const HIGHLIGHTS: [(&str, &str); 8] = [
    ("totalDamageDealtToChampions", "dégâts aux champions"),
    ("totalHeal", "soins"),
    ("timeCCingOthers", "secondes de contrôle"),
    ("damageSelfMitigated", "dégâts réduits"),
    ("totalDamageTaken", "dégâts subis"),
    ("kills", "éliminations"),
    ("goldEarned", "or gagné"),
    ("visionScore", "score de vision"),
];

pub fn highlight(g: &Game, p: &Participant) -> (&'static str, &'static str, f64) {
    let mut best = (HIGHLIGHTS[0].0, HIGHLIGHTS[0].1, n(&p.stats, HIGHLIGHTS[0].0));
    let mut best_rank = 0;
    for (key, label) in HIGHLIGHTS {
        let value = n(&p.stats, key);
        if value <= 0.0 || (key == "visionScore" && g.map_id != 11) {
            continue;
        }
        let rank = g.participants.iter().filter(|x| n(&x.stats, key) < value).count();
        if rank > best_rank {
            best = (key, label, value);
            best_rank = rank;
        }
    }
    best
}

pub fn parties(team: &[(String, Vec<i64>)], skip: i64) -> std::collections::HashMap<String, (usize, usize)> {
    let mut root: Vec<usize> = (0..team.len()).collect();
    fn find(root: &mut Vec<usize>, i: usize) -> usize {
        if root[i] != i {
            let r = find(root, root[i]);
            root[i] = r;
        }
        root[i]
    }
    for i in 0..team.len() {
        for j in i + 1..team.len() {
            if team[i].0.is_empty() || team[j].0.is_empty() {
                continue;
            }
            let shared = team[i].1.iter().filter(|id| **id != skip && team[j].1.contains(id)).count();
            if shared >= 2 {
                let (a, b) = (find(&mut root, i), find(&mut root, j));
                root[a] = b;
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..team.len() {
        let r = find(&mut root, i);
        match groups.iter_mut().find(|g| g.0 == r) {
            Some(g) => g.1.push(i),
            None => groups.push((r, vec![i])),
        }
    }
    let mut out = std::collections::HashMap::new();
    for (n, (_, members)) in groups.into_iter().filter(|g| g.1.len() >= 2).enumerate() {
        for i in &members {
            out.insert(team[*i].0.clone(), (n, members.len()));
        }
    }
    out
}

pub fn party_label(size: usize) -> String {
    match size {
        2 => "Duo",
        3 => "Trio",
        4 => "Quatuor",
        _ => "Five stack",
    }
    .to_string()
}

pub fn party_rgb(index: usize) -> u32 {
    [0xf6c867, 0x6fa8ff, 0xc58cff, 0xff9a62, 0x5fe0c8][index % 5]
}

pub const PINS: usize = 10;

pub fn toggle_pin(list: &mut Vec<Value>, puuid: &str, name: &str) -> bool {
    if let Some(i) = list.iter().position(|p| p["puuid"] == puuid) {
        list.remove(i);
        return false;
    }
    list.insert(0, serde_json::json!({"puuid": puuid, "name": name}));
    list.truncate(PINS);
    true
}

pub const GOALS: [i64; 4] = [0, 2, 3, 5];

pub fn next_goal(goal: i64) -> i64 {
    GOALS[(GOALS.iter().position(|g| *g == goal).unwrap_or(0) + 1) % GOALS.len()]
}

pub fn goal_text(goal: i64, wins: usize) -> String {
    match goal {
        0 => "Fixer un objectif".to_string(),
        g if wins as i64 >= g => format!("Objectif atteint : {wins}/{g} victoires"),
        g => format!("Objectif : {wins}/{g} victoires"),
    }
}

pub fn minutes_text(seconds: i64) -> String {
    let minutes = (seconds.max(0) + 30) / 60;
    if minutes >= 60 { format!("{} h {:02}", minutes / 60, minutes % 60) } else { format!("{minutes} min") }
}

pub fn insights(games: &[&Game], now: i64, day_start: i64, slot: &dyn Fn(i64) -> Slot, champ: &dyn Fn(i64) -> String) -> Vec<String> {
    let mut lines = Vec::new();
    let rate = |list: &[&&Game]| list.iter().filter(|g| win(g.me())).count() as f64 / list.len().max(1) as f64;
    const PARTS: [(&str, u32, u32); 4] = [("la nuit", 0, 6), ("le matin", 6, 12), ("l'après-midi", 12, 18), ("le soir", 18, 24)];
    let best_part = PARTS
        .iter()
        .filter_map(|(label, from, to)| {
            let list: Vec<&&Game> = games.iter().filter(|g| (*from..*to).contains(&slot(g.game_creation).hour)).collect();
            (list.len() >= 3).then(|| (label, rate(&list), list.len()))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((label, wr, n)) = best_part {
        lines.push(format!("Tu gagnes le plus {label} : {} % sur {n} parties.", (wr * 100.0).round()));
    }
    const DAYS: [&str; 7] = ["le lundi", "le mardi", "le mercredi", "le jeudi", "le vendredi", "le samedi", "le dimanche"];
    let best_day = (0..7u32)
        .filter_map(|d| {
            let list: Vec<&&Game> = games.iter().filter(|g| slot(g.game_creation).weekday == d).collect();
            (list.len() >= 3).then(|| (d, rate(&list), list.len()))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((d, wr, n)) = best_day {
        lines.push(format!("Meilleur jour : {} ({} % sur {n} parties).", DAYS[d as usize], (wr * 100.0).round()));
    }
    let today: i64 = games.iter().filter(|g| g.game_creation >= day_start).map(|g| g.game_duration).sum();
    let week: i64 = games.iter().filter(|g| g.game_creation >= now - 7 * 86_400_000).map(|g| g.game_duration).sum();
    if week > 0 {
        lines.push(format!("Temps de jeu : {} aujourd'hui, {} sur 7 jours.", minutes_text(today), minutes_text(week)));
    }
    let mut champs: Vec<(i64, usize, usize)> = Vec::new();
    for g in games {
        let id = g.me().champion_id;
        match champs.iter_mut().find(|c| c.0 == id) {
            Some(c) => {
                c.1 += 1;
                c.2 += usize::from(win(g.me()));
            }
            None => champs.push((id, 1, usize::from(win(g.me())))),
        }
    }
    let mut weak: Vec<&(i64, usize, usize)> = champs.iter().filter(|c| c.1 >= 3 && (c.2 as f64) < c.1 as f64 * 0.45).collect();
    weak.sort_by(|a, b| (a.2 as f64 / a.1 as f64).total_cmp(&(b.2 as f64 / b.1 as f64)));
    if !weak.is_empty() {
        let names: Vec<String> = weak.iter().take(3).map(|c| format!("{} {}V {}D", champ(c.0), c.2, c.1 - c.2)).collect();
        lines.push(format!("À éviter en ce moment : {}.", names.join(", ")));
    }
    let mut ordered: Vec<&&Game> = games.iter().collect();
    ordered.sort_by_key(|g| g.game_creation);
    let (mut best_win, mut best_loss, mut run, mut last) = (0usize, 0usize, 0usize, None);
    for g in &ordered {
        let w = win(g.me());
        run = if last == Some(w) { run + 1 } else { 1 };
        last = Some(w);
        if w { best_win = best_win.max(run) } else { best_loss = best_loss.max(run) }
    }
    if best_win >= 2 || best_loss >= 2 {
        lines.push(format!("Plus longues séries : {best_win} victoire{}, {best_loss} défaite{}.", if best_win > 1 { "s" } else { "" }, if best_loss > 1 { "s" } else { "" }));
    }
    let average = |w: bool| {
        let list: Vec<i64> = games.iter().filter(|g| win(g.me()) == w).map(|g| g.game_duration).collect();
        (!list.is_empty()).then(|| list.iter().sum::<i64>() / list.len() as i64)
    };
    if let (Some(w), Some(l)) = (average(true), average(false)) {
        lines.push(format!("Durée moyenne : {} pour les victoires, {} pour les défaites.", minutes_text(w), minutes_text(l)));
    }
    lines
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
    fn players_sharing_recent_games_form_parties() {
        let p = |id: &str, games: &[i64]| (id.to_string(), games.to_vec());
        let team = vec![p("a", &[1, 2, 3]), p("b", &[2, 3, 9]), p("c", &[3, 4, 5]), p("d", &[4, 5]), p("e", &[7])];
        let found = parties(&team, 0);
        assert_eq!(found["a"].1, 2);
        assert_eq!(found["a"].0, found["b"].0);
        assert_eq!(found["c"].1, 2);
        assert_ne!(found["a"].0, found["c"].0);
        assert!(!found.contains_key("e"));
        let five = vec![p("a", &[1, 2]), p("b", &[1, 2]), p("c", &[1, 2]), p("d", &[1, 2]), p("e", &[1, 2])];
        assert_eq!(party_label(parties(&five, 0)["e"].1), "Five stack");
        assert!(parties(&five, 1).is_empty());
        assert_eq!(party_label(3), "Trio");
        assert_ne!(party_rgb(0), party_rgb(1));
    }

    #[test]
    fn highlight_picks_the_stat_where_i_rank_best() {
        let player = |id: i64, dmg: f64, heal: f64| {
            let mut stats = Map::new();
            stats.insert("totalDamageDealtToChampions".into(), dmg.into());
            stats.insert("totalHeal".into(), heal.into());
            Participant { participant_id: id, stats, ..Default::default() }
        };
        let g = Game { map_id: 12, participants: vec![player(1, 10000.0, 5000.0), player(2, 20000.0, 100.0), player(3, 30000.0, 200.0)], ..Default::default() };
        assert_eq!(highlight(&g, &g.participants[0]), ("totalHeal", "soins", 5000.0));
        assert_eq!(highlight(&g, &g.participants[2]).0, "totalDamageDealtToChampions");
        let quiet = Game { participants: vec![player(1, 0.0, 0.0)], ..Default::default() };
        assert_eq!(highlight(&quiet, &quiet.participants[0]).1, "dégâts aux champions");
    }

    #[test]
    fn two_profiles_compare_line_by_line() {
        let game = |won: bool, champ: i64, kills: f64| {
            let mut stats = Map::new();
            stats.insert("win".into(), Value::Bool(won));
            stats.insert("kills".into(), kills.into());
            stats.insert("deaths".into(), 1.0.into());
            Game { game_duration: 1800, participants: vec![Participant { champion_id: champ, stats, ..Default::default() }], ..Default::default() }
        };
        let a = [game(true, 1, 4.0), game(false, 1, 2.0)];
        let b = [game(true, 2, 6.0)];
        let mine: Vec<&Game> = a.iter().collect();
        let theirs: Vec<&Game> = b.iter().collect();
        let lines = versus(&mine, &theirs, &|c| format!("C{c}"));
        assert_eq!(lines[0], ("Parties".into(), "2".into(), "1".into()));
        assert_eq!(lines[1], ("Taux de victoire".into(), "50 %".into(), "100 %".into()));
        assert_eq!(lines[2], ("KDA moyen".into(), "3.00".into(), "6.00".into()));
        assert_eq!(lines[4], ("Champion favori".into(), "C1".into(), "C2".into()));
        assert!(versus(&[], &theirs, &|_| String::new()).is_empty());
    }

    #[test]
    fn pins_toggle_and_stay_short() {
        let mut list = Vec::new();
        assert!(toggle_pin(&mut list, "a", "A#1"));
        assert!(!toggle_pin(&mut list, "a", "A#1"));
        assert!(list.is_empty());
        for i in 0..12 {
            toggle_pin(&mut list, &i.to_string(), "x");
        }
        assert_eq!(list.len(), PINS);
        assert_eq!(list[0]["puuid"], "11");
    }

    #[test]
    fn the_period_filter_keeps_recent_games() {
        let list = vec![Game { game_creation: 100, game_duration: 1800, ..Default::default() }, Game { game_creation: 900, game_duration: 1800, ..Default::default() }];
        let kept = filter_games(&list, &HistoryFilter { since: 500, ..Default::default() });
        assert_eq!(kept.iter().map(|g| g.game_creation).collect::<Vec<_>>(), vec![900]);
        assert_eq!(filter_games(&list, &HistoryFilter::default()).len(), 2);
    }

    #[test]
    fn session_goals_cycle_and_report() {
        assert_eq!(next_goal(0), 2);
        assert_eq!(next_goal(5), 0);
        assert_eq!(next_goal(7), 2);
        assert_eq!(goal_text(0, 4), "Fixer un objectif");
        assert_eq!(goal_text(3, 1), "Objectif : 1/3 victoires");
        assert_eq!(goal_text(3, 3), "Objectif atteint : 3/3 victoires");
    }

    #[test]
    fn insights_come_from_kept_games() {
        let game = |at: i64, won: bool, champ: i64, minutes: i64| {
            let mut stats = Map::new();
            stats.insert("win".into(), Value::Bool(won));
            Game { game_creation: at, game_duration: minutes * 60, participants: vec![Participant { champion_id: champ, stats, ..Default::default() }], ..Default::default() }
        };
        let hour = 3_600_000;
        let list = vec![
            game(20 * hour, true, 1, 30), game(21 * hour, true, 1, 28), game(22 * hour, true, 2, 26),
            game(8 * hour, false, 3, 31), game(9 * hour, false, 3, 33), game(10 * hour, false, 3, 35), game(11 * hour, true, 1, 30),
        ];
        let refs: Vec<&Game> = list.iter().collect();
        let slot = |t: i64| Slot { hour: ((t / hour) % 24) as u32, weekday: if t < 15 * hour { 5 } else { 1 } };
        let lines = insights(&refs, 23 * hour, 0, &slot, &|c| format!("C{c}"));
        assert!(lines.iter().any(|l| l == "Tu gagnes le plus le soir : 100 % sur 3 parties."), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("Meilleur jour : le mardi")), "{lines:?}");
        assert!(lines.iter().any(|l| l == "Temps de jeu : 3 h 33 aujourd'hui, 3 h 33 sur 7 jours."), "{lines:?}");
        assert!(lines.iter().any(|l| l == "À éviter en ce moment : C3 0V 3D."), "{lines:?}");
        assert!(lines.iter().any(|l| l == "Plus longues séries : 4 victoires, 3 défaites."), "{lines:?}");
        assert!(lines.iter().any(|l| l.starts_with("Durée moyenne : 29 min pour les victoires")), "{lines:?}");
        assert_eq!(minutes_text(3_600), "1 h 00");
        assert!(insights(&[], 0, 0, &slot, &|_| String::new()).is_empty());
    }

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
        let f = |result: &str, queue, champ| ids(&filter_games(&g, &HistoryFilter { result: result.into(), queue, champ, since: 0 }));
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
