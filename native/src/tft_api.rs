use crate::net::json_url;
use crate::opgg::call;
use crate::store;
use crate::text::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const CD: &str = "https://raw.communitydragon.org/latest/";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Asset {
    pub name: String,
    pub icon: String,
    pub cost: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TftData {
    pub set: String,
    pub units: HashMap<String, Asset>,
    pub items: HashMap<String, Asset>,
    pub traits: HashMap<String, Asset>,
}

fn png(p: &Value) -> String {
    match p.as_str() {
        Some(x) if !x.is_empty() => {
            let low = x.to_lowercase();
            let base = low.strip_suffix(".tex").or(low.strip_suffix(".dds")).map(|b| format!("{b}.png")).unwrap_or_else(|| low.clone());
            format!("{CD}game/{base}")
        }
        _ => String::new(),
    }
}

pub fn parse_tft(j: &Value) -> Option<TftData> {
    let sets = j["sets"].as_object()?;
    let set = sets.keys().max_by_key(|k| k.parse::<i64>().unwrap_or(0))?.clone();
    let s = &sets[&set];
    let mut d = TftData { set, ..Default::default() };
    for c in s["champions"].as_array().into_iter().flatten() {
        let icon = if !c["tileIcon"].is_null() { &c["tileIcon"] } else if !c["squareIcon"].is_null() { &c["squareIcon"] } else { &c["icon"] };
        d.units.insert(c["apiName"].as_str().unwrap_or_default().into(), Asset { name: c["name"].as_str().unwrap_or_default().into(), icon: png(icon), cost: c["cost"].as_i64().unwrap_or(0) });
    }
    for t in s["traits"].as_array().into_iter().flatten() {
        d.traits.insert(t["apiName"].as_str().unwrap_or_default().into(), Asset { name: t["name"].as_str().unwrap_or_default().into(), icon: png(&t["icon"]), cost: 0 });
    }
    for i in j["items"].as_array().into_iter().flatten() {
        let id = i["apiName"].as_str().unwrap_or_default();
        if id.starts_with("DA_") {
            d.items.insert(id.into(), Asset { name: i["name"].as_str().unwrap_or_default().into(), icon: png(&i["icon"]), cost: 0 });
        }
    }
    Some(d)
}

#[derive(Serialize, Deserialize)]
struct Cached {
    t: i64,
    data: TftData,
}

pub async fn tft_data() -> Result<Arc<TftData>, String> {
    static CELL: Mutex<Option<Arc<TftData>>> = Mutex::const_new(None);
    let mut slot = CELL.lock().await;
    if let Some(d) = slot.as_ref() {
        return Ok(d.clone());
    }
    if let Some(c) = store::load::<Cached>("rw-tft") {
        if now_ms() - c.t < 86_400_000 {
            let d = Arc::new(c.data);
            *slot = Some(d.clone());
            return Ok(d);
        }
    }
    let j = json_url(&format!("{CD}cdragon/tft/fr_fr.json")).await?;
    let d = parse_tft(&j).ok_or("données TFT illisibles")?;
    store::save("rw-tft", &Cached { t: now_ms(), data: d.clone() });
    let d = Arc::new(d);
    *slot = Some(d.clone());
    Ok(d)
}

pub struct TftDecks {
    pub decks: Vec<Value>,
    pub games: i64,
    pub at: String,
}

pub async fn tft_decks() -> Result<TftDecks, String> {
    let r = call("tft_list_meta_decks", json!({})).await?;
    Ok(TftDecks { decks: r["data"].as_array().cloned().unwrap_or_default(), games: r["metadata"]["gameStatCounts"].as_i64().unwrap_or(0), at: r["metadata"]["gameStatDateTime"].as_str().unwrap_or_default().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let j = json!({"sets": {"9": {}, "15": {"champions": [{"apiName": "A", "name": "Ahri", "cost": 3, "tileIcon": "ASSETS/X.TEX"}], "traits": [{"apiName": "T", "name": "Mage", "icon": "a/b.dds"}]}}, "items": [{"apiName": "DA_1", "name": "Épée", "icon": "i.tex"}, {"apiName": "TFT_2", "name": "Non"}]});
        let d = parse_tft(&j).unwrap();
        assert_eq!(d.set, "15");
        assert_eq!(d.units["A"].icon, format!("{CD}game/assets/x.png"));
        assert_eq!(d.traits["T"].icon, format!("{CD}game/a/b.png"));
        assert_eq!(d.items.len(), 1);
    }
}
