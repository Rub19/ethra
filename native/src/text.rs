use chrono::{Datelike, Local, TimeZone, Timelike};

pub fn friendly_error(raw: &str) -> String {
    let raw = raw.trim();
    let lower = raw.to_lowercase();
    let code = raw.split_whitespace().next().and_then(|word| word.parse::<u16>().ok());
    let message = if raw.starts_with("VALO_CLOSED") {
        "VALORANT n'est pas lancé : ouvre le jeu puis réessaie."
    } else if raw == "LECTURE_SEULE" {
        "Mode lecture seule : cette action est désactivée."
    } else if raw.starts_with("OPGG") {
        "OP.GG ne répond pas pour l'instant."
    } else {
        match code {
            Some(401 | 403) => "Session Riot expirée : relance le client ou le jeu.",
            Some(404) => "Riot n'a pas ces données pour ce compte ou cette file.",
            Some(429) => "Riot limite les requêtes : attends quelques secondes.",
            Some(500..=599) => "Les serveurs Riot ont un souci, réessaie plus tard.",
            _ if ["error sending request", "connect", "timed out", "dns", "connexion"].iter().any(|word| lower.contains(word)) => "Pas de réseau ou serveur injoignable.",
            _ => return raw.to_string(),
        }
    };
    match code {
        Some(code) => format!("{message} (erreur {code})"),
        None => message.to_string(),
    }
}

pub fn fr(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    if v < 0 {
        out.insert(0, '-');
    }
    out
}

pub fn pct1(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

pub fn pct0(x: f64) -> String {
    format!("{}%", (x * 100.0).round() as i64)
}

pub fn signed(v: i64) -> String {
    if v >= 0 {
        format!("+{v}")
    } else {
        v.to_string()
    }
}

pub fn mmss(sec: f64) -> String {
    let s = sec.max(0.0);
    format!("{}:{:02}", (s / 60.0).floor() as i64, (s % 60.0).floor() as i64)
}

pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

pub fn ago(t: i64) -> String {
    let m = ((now_ms() - t) as f64 / 60000.0).round() as i64;
    if m < 1 {
        "à l'instant".into()
    } else if m < 60 {
        format!("il y a {m} min")
    } else if m < 1440 {
        format!("il y a {} h", (m as f64 / 60.0).round() as i64)
    } else {
        format!("il y a {} j", (m as f64 / 1440.0).round() as i64)
    }
}

pub fn midnight_ms() -> i64 {
    Local::now().date_naive().and_hms_opt(0, 0, 0).and_then(|d| Local.from_local_datetime(&d).earliest()).map(|d| d.timestamp_millis()).unwrap_or(0)
}

const MONTHS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
const SHORT: [&str; 12] = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];

fn local(t: i64) -> Option<chrono::DateTime<Local>> {
    Local.timestamp_millis_opt(t).single()
}

pub fn date_long_time(t: i64) -> String {
    local(t).map(|d| format!("{} {} à {:02}:{:02}", d.day(), MONTHS[d.month0() as usize], d.hour(), d.minute())).unwrap_or_default()
}

pub fn date_long(t: i64) -> String {
    local(t).map(|d| format!("{} {}", d.day(), MONTHS[d.month0() as usize])).unwrap_or_default()
}

pub fn date_short(t: i64) -> String {
    local(t).map(|d| format!("{} {}", d.day(), SHORT[d.month0() as usize])).unwrap_or_default()
}

pub fn date_short_time(t: i64) -> String {
    local(t).map(|d| format!("{} {}, {:02}:{:02}", d.day(), SHORT[d.month0() as usize], d.hour(), d.minute())).unwrap_or_default()
}

pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            x => x,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(fr(1234567), "1\u{202f}234\u{202f}567");
        assert_eq!(fr(-950), "-950");
        assert_eq!(pct1(0.5234), "52.3%");
        assert_eq!(pct0(0.5), "50%");
        assert_eq!(mmss(125.9), "2:05");
        assert_eq!(signed(3), "+3");
        assert_eq!(fold("Élise Çà"), "elise ca");
    }
}

#[cfg(test)]
mod friendly_error_tests {
    use super::friendly_error;

    #[test]
    fn riot_errors_become_sentences() {
        assert_eq!(friendly_error("404 valo:pd:/match-history/v1/history/vme"), "Riot n'a pas ces données pour ce compte ou cette file. (erreur 404)");
        assert_eq!(friendly_error("401 {\"error\":\"expired\"}"), "Session Riot expirée : relance le client ou le jeu. (erreur 401)");
        assert_eq!(friendly_error("VALO_CLOSED lockfile"), "VALORANT n'est pas lancé : ouvre le jeu puis réessaie.");
        assert_eq!(friendly_error("error sending request for url (https://pd.eu.a.pvp.net)"), "Pas de réseau ou serveur injoignable.");
        assert_eq!(friendly_error("OPGG {}"), "OP.GG ne répond pas pour l'instant.");
        assert_eq!(friendly_error("Joueur absent de cette partie."), "Joueur absent de cette partie.");
    }
}
