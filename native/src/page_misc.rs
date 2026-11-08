use crate::history::{parse_tft_games, tft_queue, tft_summary, TftGame};
use crate::lol::*;
use crate::opgg::RANK_FILTERS;
use crate::overlay::{icon_baron, icon_dragon, icon_inhib, row as ov_row, widget};
use crate::page_builds::fav_champs;
use crate::page_profile::copy_text;
use crate::store::{self, save_settings, settings, Settings};
use crate::text::*;
use crate::tft_api::*;
use crate::update;
use crate::valo_api::seen_players;
use crate::{good, ms, on_ui, opt, spawn, ss, stat, with_app, App};
use crate::{AvatarItem, AvatarSection, GameCard, KeyLine, Opt, OvAlert, OvCard, OverlaysData, PickerData, RankRow, SearchOpt, SetRow, SettingsData, TftDeckV, TftDecksData, TftProfileData, TftRow, TftTraitV, TftUnitV};
use serde_json::Value;
use slint::{Color, ComponentHandle};
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct SearchState {
    pub q: String,
    pub sel: usize,
    pub focused: bool,
    pub keys: Vec<String>,
}

#[derive(Default)]
pub struct SettingsState {
    pub checked: bool,
    pub armed: String,
    pub migrated: bool,
}

#[derive(Default)]
pub struct TftState {
    pub data: Option<Arc<TftData>>,
    pub decks: Option<TftDecks>,
    pub err: String,
    pub sum: Summoner,
    pub rank: Option<Queue>,
    pub games: Option<Vec<TftGame>>,
    pub more: String,
    pub ready: bool,
}

#[derive(Default)]
pub struct PickerState {
    pub platform: String,
    pub q: String,
    pub choice: String,
}

pub fn avatar() -> String {
    let v = store::get("rw-avatar").unwrap_or_default();
    match v.strip_prefix("http://ethone.localhost/") {
        Some(p) => format!("https://ethone.dev/avatars/library/{p}"),
        None => v,
    }
}

pub fn avatar_or(fallback: &str) -> String {
    let a = avatar();
    if a.is_empty() {
        fallback.to_string()
    } else {
        a
    }
}

pub fn render_games(a: &App) {
    let list = [
        ("lol", "League of Legends", "Profil, builds, tier list, overlays", "https://ddragon.leagueoflegends.com/cdn/img/champion/loading/Jinx_0.jpg"),
        ("valo", "VALORANT", "Profil, matchs, rangs en direct", "https://media.valorant-api.com/agents/add6443a-41bd-e414-f6ad-e58d267f4e95/fullportrait.png"),
        ("tft", "Teamfight Tactics", "Decks méta du set en cours", "https://ddragon.leagueoflegends.com/cdn/img/champion/loading/Teemo_0.jpg"),
    ];
    let d = a.data();
    d.set_games(ms(list.iter().map(|(k, n, s, u)| GameCard { key: ss(k), name: ss(n), sub: ss(s), art: ss(u), on: a.game == *k }).collect::<Vec<_>>()));
    d.set_lol_mark(ss(if crate::net::game_icon("lol").is_some() { "local:lol" } else { "" }));
    d.set_tft_mark(ss(if crate::net::game_icon("tft").is_some() { "local:tft" } else { "" }));
}

pub fn rerender_current(a: &mut App) {
    if a.page() == "profil" && a.profile.ready {
        crate::page_profile::render(a, true);
    }
}

fn run_search(a: &mut App) {
    let t = fold(a.search.q.trim());
    let q = a.search.q.trim().to_string();
    let mut opts: Vec<SearchOpt> = vec![];
    let mut keys: Vec<String> = vec![];
    if a.game == "valo" {
        let all = if a.search.focused { seen_players() } else { vec![] };
        let score = |n: &str| if fold(n).starts_with(&t) { 0 } else if fold(n).contains(&t) { 1 } else { 2 };
        let mut list: Vec<_> = all.into_iter().filter(|p| t.is_empty() || fold(&format!("{}#{}", p.name, p.tag)).contains(&t)).collect();
        if !t.is_empty() {
            list.sort_by_key(|p| score(&p.name));
        }
        for p in list.iter().take(8) {
            keys.push(format!("go:v-profil/{}", p.puuid));
            opts.push(SearchOpt { label: ss(&p.name), tag: ss(&p.tag), sub: ss(format!("Croisé {}", ago(p.t))), letter: ss(p.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()), ..Default::default() });
        }
        a.data().set_search_empty(ss(if opts.is_empty() && a.search.focused {
            if t.is_empty() {
                "Les joueurs de tes parties apparaîtront ici. Riot ne permet pas de chercher un joueur que tu n'as jamais croisé."
            } else {
                "Aucun joueur croisé avec ce nom. Riot ne permet pas de chercher un joueur que tu n'as jamais croisé."
            }
        } else {
            ""
        }));
    } else {
        if q.contains('#') {
            keys.push(format!("lookup:{q}"));
            opts.push(SearchOpt { label: ss(&q), sub: ss("Chercher ce joueur"), letter: ss(q.chars().next().map(String::from).unwrap_or_default()), ..Default::default() });
        }
        if let Some(d) = a.d.clone() {
            if !t.is_empty() {
                for (id, c) in d.champs.iter().filter(|(_, c)| fold(&c.name).contains(&t)).take(6) {
                    keys.push(format!("go:champion/{id}"));
                    opts.push(SearchOpt { label: ss(&c.name), sub: ss("Build méta du champion"), icon: ss(img_champ(&d, *id)), ..Default::default() });
                }
            } else {
                for id in fav_champs().into_iter().filter(|id| d.champs.contains_key(id)).take(5) {
                    keys.push(format!("go:champion/{id}"));
                    opts.push(SearchOpt { label: ss(&d.champs[&id].name), sub: ss("Favori"), icon: ss(img_champ(&d, id)), ..Default::default() });
                }
            }
        }
        let recent: Vec<Value> = store::load("rw-recent").unwrap_or_default();
        for p in recent.iter().filter(|p| t.is_empty() || fold(p["name"].as_str().unwrap_or_default()).contains(&t)).take(4) {
            let name = p["name"].as_str().unwrap_or_default();
            keys.push(format!("go:profil/{}", p["puuid"].as_str().unwrap_or_default()));
            opts.push(SearchOpt { label: ss(name), sub: ss("Joueur récent"), letter: ss(name.chars().next().map(String::from).unwrap_or_default()), ..Default::default() });
        }
        a.data().set_search_empty(ss(if a.search.focused && !t.is_empty() && opts.is_empty() { "Aucun champion trouvé. Pour un joueur, écris Nom#TAG." } else { "" }));
    }
    let sel = a.search.sel.min(opts.len().saturating_sub(1));
    a.search.sel = sel;
    for (i, o) in opts.iter_mut().enumerate() {
        o.on = i == sel;
        o.key = ss(format!("search-pick:{i}"));
    }
    a.search.keys = keys;
    let d = a.data();
    let open = a.search.focused && (!opts.is_empty() || !d.get_search_empty().is_empty());
    d.set_search_opts(ms(opts));
    d.set_search_open(open);
}

fn search_pick(a: &mut App, i: usize) {
    let Some(k) = a.search.keys.get(i).cloned() else { return };
    a.search.q.clear();
    a.search.sel = 0;
    a.search.focused = false;
    a.data().set_search_text(ss(""));
    a.data().set_search_open(false);
    if let Some(name) = k.strip_prefix("lookup:") {
        let name = name.to_string();
        spawn(async move {
            let r = find_player(&name).await;
            on_ui(move |a| match r {
                Ok(puuid) => {
                    let mut recent: Vec<Value> = store::load("rw-recent").unwrap_or_default();
                    recent.retain(|p| p["puuid"] != puuid.as_str());
                    recent.insert(0, serde_json::json!({"puuid": puuid, "name": name}));
                    recent.truncate(8);
                    store::save("rw-recent", &recent);
                    a.go(&format!("profil/{puuid}"));
                }
                Err(_) => a.toast(format!("Joueur introuvable : {name}")),
            });
        });
    } else if let Some(r) = k.strip_prefix("go:") {
        a.go(r);
    }
}

fn card(key: &str, title: &str, desc: &str, on: bool, kind: i32) -> OvCard {
    OvCard { key: ss(key), title: ss(title), desc: ss(desc), on, kind, widget: Default::default(), alert: Default::default(), skill_icon: ss("") }
}

pub fn render_overlays(a: &mut App) {
    let s = settings();
    let ch = |id: &str| format!("https://ddragon.leagueoflegends.com/cdn/15.1.1/img/champion/{id}.png");
    let it = |id: i64| format!("https://ddragon.leagueoflegends.com/cdn/15.1.1/img/item/{id}.png");
    let mut c = card("ovTimers", "Timers d'objectifs", "Dragon, Nashor et réapparition des inhibiteurs, calculés depuis les événements de la partie.", s.ov_timers, 0);
    c.widget = widget("Objectifs", vec![ov_row(icon_baron(), "Nashor", "0:48", 0, 0.85), ov_row(icon_dragon(), "Dragon", "UP", 3, -1.0), ov_row(icon_inhib(), "Inhib mid rouge", "3:12", 0, 0.35)]);
    let mut al = card("ovAlerts", "Alertes d'objectifs", "Bandeau en haut de l'écran 30 secondes avant l'apparition d'un objectif.", s.ov_alerts, 1);
    al.alert = OvAlert { icon: ss(icon_dragon()), name: ss("Dragon"), secs: ss("30s") };
    let mut sk = card("ovSkill", "Ordre des compétences", "Indique la compétence à monter à chaque niveau, selon le build méta OP.GG.", s.ov_skill, 2);
    sk.skill_icon = ss("https://cdn.communitydragon.org/latest/champion/Ahri/ability-icon/q");
    let mut be = card("ovBench", "Benchmark de lane", "Ton écart de CS, de valeur d'objets et de niveau avec ton adversaire direct.", s.ov_bench, 0);
    be.widget = widget("vs Yasuo", vec![ov_row("", "CS", "+12", 1, -1.0), ov_row("", "Objets", "-0.4k", 2, -1.0), ov_row("", "Niveau", "+1", 1, -1.0)]);
    let mut de = card("ovDeaths", "Ennemis morts", "Les adversaires morts et leur temps de réapparition, pour savoir quand forcer un objectif.", s.ov_deaths, 0);
    de.widget = widget("Ennemis morts", vec![ov_row(ch("Yasuo"), "Yasuo", "24s", 1, -1.0), ov_row(ch("Thresh"), "Thresh", "11s", 1, -1.0)]);
    let mut ni = card("ovItem", "Prochain objet", "Le prochain objet de ton build méta et l'or qu'il te manque pour l'acheter.", s.ov_item, 0);
    ni.widget = widget("Prochain objet", vec![ov_row(it(3089), "Coiffe de Rabadon", "-620", 0, 0.82)]);
    let mut go = card("ovGold", "Valeur des objets", "Différence de valeur totale des objets entre ton équipe et l'équipe adverse.", s.ov_gold, 0);
    go.widget = widget("Valeur des objets", vec![ov_row("", "Équipes", "+3.8k", 1, -1.0)]);
    let mut ju = card("ovJungle", "Jungle path", "Ton full clear étape par étape pendant les premières minutes quand tu joues jungle, avec le crabe à 3:30. Le départ se règle sur la page du champion.", s.ov_jungle, 0);
    ju.widget = widget("Jungle path, fin côté top", vec![ov_row("", "1. Buff rouge", "", 0, -1.0), ov_row("", "2. Krugs", "", 0, -1.0), ov_row("", "3. Raptors", "", 0, -1.0), ov_row("", "C. Crabe", "1:12", 0, -1.0)]);
    let mut st = card("ovStats", "Tes stats", "KDA, CS par minute et or disponible, toujours visibles.", s.ov_stats, 0);
    st.widget = widget("Toi", vec![ov_row("", "KDA", "4 / 1 / 6", 0, -1.0), ov_row("", "CS/min", "7.8", 0, -1.0), ov_row("", "Or", "1 240", 0, -1.0)]);
    a.data().set_overlays(OverlaysData { enabled: s.ov_enabled, cards: ms(vec![c, al, sk, be, de, ni, go, ju, st]) });
}

fn toggle_setting(s: &mut Settings, key: &str) {
    let f: &mut bool = match key {
        "ovTimers" => &mut s.ov_timers,
        "ovAlerts" => &mut s.ov_alerts,
        "ovSkill" => &mut s.ov_skill,
        "ovBench" => &mut s.ov_bench,
        "ovDeaths" => &mut s.ov_deaths,
        "ovItem" => &mut s.ov_item,
        "ovGold" => &mut s.ov_gold,
        "ovJungle" => &mut s.ov_jungle,
        "ovStats" => &mut s.ov_stats,
        "ovEnabled" => &mut s.ov_enabled,
        "autoOpenLive" => &mut s.auto_open_live,
        "autoAccept" => &mut s.auto_accept,
        "autoRunes" => &mut s.auto_runes,
        "autoSpells" => &mut s.auto_spells,
        "autoItems" => &mut s.auto_items,
        _ => return,
    };
    *f = !*f;
}

pub fn open_settings(a: &mut App) {
    a.settings.armed.clear();
    render_settings(a);
}

fn kept_games() -> usize {
    store::keys_with("rw-games-").iter().map(|k| store::load::<Vec<Value>>(k).map(|l| l.len()).unwrap_or(0)).sum()
}

fn human(b: u64) -> String {
    format!("{:.1} Mo", b as f64 / 1_048_576.0)
}

pub fn render_settings(a: &mut App) {
    let s = settings();
    let gopt = |key: &str, label: &str, help: &str, on: bool| Opt { key: ss(format!("set:{key}")), label: ss(label), sub: ss(help), on, ..Default::default() };
    let source_label = match s.source.as_str() {
        "opgg" => "Toujours la méta OP.GG",
        "toi" => "Toujours tes parties",
        "riot" => "Toujours la recommandation Riot",
        _ => "Auto",
    };
    let rank_label = RANK_FILTERS.iter().find(|x| x.0 == s.rank_filter).map(|x| x.1).unwrap_or("");
    let games = kept_games();
    let lp = store::keys_with("rw-lp-").len();
    let recent = store::load::<Vec<Value>>("rw-recent").map(|l| l.len()).unwrap_or(0);
    let seen = seen_players().len();
    let armed = a.settings.armed.clone();
    let setrow = |title: String, help: String, btn: &str, key: &str, enabled: bool| SetRow { title: ss(title), help: ss(help), btn: ss(if armed == key { "Confirmer" } else { btn }), key: ss(key), enabled, armed: armed == key };
    let up = update::latest_info(a);
    let d = SettingsData {
        general: ms(vec![
            gopt("autoOpenLive", "Ouvrir la partie en cours automatiquement", "Kairo passe sur la page de la partie dès la sélection des champions ou des agents.", s.auto_open_live),
            gopt("autoAccept", "Accepter la partie automatiquement", "Clique sur « Accepter » à ta place quand une partie League est trouvée.", s.auto_accept),
        ]),
        lol: ms(vec![
            gopt("autoRunes", "Importer les runes", "Crée une page « Kairo : Champion Rôle » dès que tu verrouilles ton champion.", s.auto_runes),
            gopt("autoSpells", "Importer les sorts d'invocateur", "Applique les sorts les plus joués avec ce champion.", s.auto_spells),
            gopt("autoItems", "Créer l'item set", "Ajoute ton build dans la boutique en jeu.", s.auto_items),
        ]),
        source_label: ss(source_label),
        source_opts: ms([("auto", "Auto"), ("opgg", "Toujours la méta OP.GG"), ("toi", "Toujours tes parties"), ("riot", "Toujours la recommandation Riot")].iter().map(|(k, l)| opt(format!("setsrc:{k}"), l, s.source == *k)).collect()),
        rank_label: ss(rank_label),
        rank_opts: ms(RANK_FILTERS.iter().map(|(k, l)| opt(format!("setrank:{k}"), l, s.rank_filter == *k)).collect()),
        seen: setrow("Joueurs croisés".into(), format!("{seen} joueurs mémorisés pour la recherche. Ils restent sur ce PC."), "Oublier", "clear:seen", seen > 0),
        keys: ms([("Ctrl K", "Rechercher un joueur ou un champion"), ("Ctrl 1, 2, 3", "Passer sur League, VALORANT ou TFT"), ("Alt ← et Alt →", "Page précédente et suivante"), ("Boutons latéraux", "Page précédente et suivante")].iter().map(|(k, t)| KeyLine { key: ss(k), text: ss(t) }).collect()),
        data_rows: ms(vec![
            setrow("Cache TFT".into(), "Noms et icônes du set en cours, rechargés une fois par jour.".into(), "Vider", "clear:tft", true),
            setrow("Images".into(), format!("{} d'images gardées sur ce PC pour aller plus vite.", human(crate::images::disk_cache_size())), "Vider", "clear:images", true),
            setrow("Parties conservées".into(), format!("{games} parties League gardées sur ce PC, au-delà des 20 du client."), "Effacer", "clear:games", games > 0),
            setrow("Historique LP".into(), if lp > 0 { "La courbe de LP de tes comptes repart de zéro si tu l'effaces.".into() } else { "Aucune courbe enregistrée.".to_string() }, "Effacer", "clear:lp", lp > 0),
            setrow("Recherches récentes".into(), format!("{recent} joueur{} proposé{} dans la recherche.", if recent > 1 { "s" } else { "" }, if recent > 1 { "s" } else { "" }), "Oublier", "clear:recent", recent > 0),
        ]),
        version: ss(update::VERSION),
        update_note: ss(match &up {
            Some(u) => format!("{} disponible", u.0),
            None if a.settings.checked => "à jour".to_string(),
            None => "vérification…".to_string(),
        }),
        update_tone: if up.is_some() { 1 } else { 4 },
        update_url: ss(up.as_ref().map(|u| u.1.clone()).unwrap_or_default()),
        update_label: ss(up.as_ref().map(|u| format!("Télécharger la version {}", u.0)).unwrap_or_default()),
        migrated: ss(""),
    };
    a.data().set_settings(d);
}

fn clear_data(a: &mut App, what: &str) {
    match what {
        "seen" => {
            store::remove(&["rw-vseen".to_string()]);
            a.toast("Joueurs croisés oubliés");
        }
        "tft" => {
            store::remove(&["rw-tft".to_string()]);
            a.toast("Cache TFT vidé, il sera rechargé");
        }
        "images" => {
            let n = crate::images::clear_disk_cache();
            a.toast(format!("{} d'images effacés", human(n)));
        }
        "games" => {
            store::remove(&store::keys_with("rw-games-"));
            a.toast("Parties conservées effacées");
        }
        "lp" => {
            store::remove(&store::keys_with("rw-lp-"));
            a.toast("Historique LP effacé");
        }
        _ => {
            store::remove(&["rw-recent".to_string()]);
            a.toast("Recherches récentes oubliées");
        }
    }
    a.settings.armed.clear();
    render_settings(a);
}

fn tft_color(cost: i64) -> Color {
    Color::from_argb_encoded(
        0xff000000
            | match cost {
                1 => 0x8f97a8,
                2 => 0x2fbf71,
                3 => 0x3d8bff,
                4 => 0xc158ff,
                5 => 0xf6c867,
                _ => 0x262d44,
            },
    )
}

pub fn open_tft_decks(a: &mut App) {
    a.tft.err.clear();
    a.data().set_tft_decks(TftDecksData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async move {
        let (t, r) = tokio::join!(tft_data(), tft_decks());
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match (t, r) {
                (Ok(t), Ok(r)) => {
                    a.tft.data = Some(t);
                    a.tft.decks = Some(r);
                    render_decks(a);
                }
                (Err(e), _) | (_, Err(e)) => a.data().set_tft_decks(TftDecksData { error: ss(e), ..Default::default() }),
            }
        });
    });
}

fn parse_iso_date(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis())
}

pub fn deck_view(k: &Value, t: &TftData) -> TftDeckV {
    let s = &k["stat"]["deck"];
    let f = |x: &str| s[x].as_f64().unwrap_or(0.0);
    let mut units: Vec<&Value> = k["units"].as_array().map(|l| l.iter().collect()).unwrap_or_default();
    units.sort_by_key(|u| t.units.get(u["key"].as_str().unwrap_or_default()).map(|x| x.cost).unwrap_or(0));
    let uv: Vec<TftUnitV> = units
        .iter()
        .map(|u| {
            let key = u["key"].as_str().unwrap_or_default();
            let info = t.units.get(key);
            TftUnitV {
                icon: ss(info.map(|x| x.icon.clone()).unwrap_or_default()),
                name: ss(info.map(|x| x.name.clone()).unwrap_or_else(|| key.replace("DA_", "").replace("18", ""))),
                stars: ss("★".repeat(u["tier"].as_i64().unwrap_or(1).max(1) as usize)),
                color: tft_color(info.map(|x| x.cost).unwrap_or(0)),
                core: u["isCore"] == true,
                items: ms(u["items"].as_array().into_iter().flatten().filter_map(|i| t.items.get(i.as_str().unwrap_or_default())).filter(|x| !x.icon.is_empty()).map(|x| ss(&x.icon)).collect()),
            }
        })
        .collect();
    let mut traits: Vec<&Value> = k["traits"].as_array().map(|l| l.iter().filter(|x| x["style"].as_i64().unwrap_or(0) > 0).collect()).unwrap_or_default();
    traits.sort_by_key(|x| (std::cmp::Reverse(x["style"].as_i64().unwrap_or(0)), std::cmp::Reverse(x["numUnits"].as_i64().unwrap_or(0))));
    let tv: Vec<TftTraitV> = traits
        .iter()
        .map(|x| {
            let key = x["key"].as_str().unwrap_or_default();
            let info = t.traits.get(key);
            TftTraitV { icon: ss(info.map(|i| i.icon.clone()).unwrap_or_default()), n: ss(x["numUnits"].as_i64().unwrap_or(0).to_string()), name: ss(info.map(|i| i.name.clone()).unwrap_or_else(|| key.to_string())), style: x["style"].as_i64().unwrap_or(0) as i32 }
        })
        .collect();
    let badge = |key: &str| k["badge"].as_array().into_iter().flatten().find(|b| b["key"] == key).and_then(|b| b["value"].as_i64()).unwrap_or(0);
    let mut tags = vec![];
    if badge("difficulty") > 0 {
        tags.push(ss(format!("Difficulté {}/5", badge("difficulty"))));
    }
    if badge("reroll") > 0 {
        tags.push(ss(format!("Reroll niveau {}", badge("reroll"))));
    }
    let tier = k["stat"]["opTier"].as_str().unwrap_or_default();
    TftDeckV {
        tier: ss(tier),
        tier_tone: match tier.to_lowercase().as_str() {
            "op" => 1,
            "s" => 2,
            "a" => 3,
            _ => 0,
        },
        name: ss(k["name"]["fr_FR"].as_str().or(k["name"]["en_US"].as_str()).unwrap_or_default()),
        tags: ms(tags),
        stats: ms(vec![stat(format!("{:.2}", f("avgPlacement")), "Place moyenne", 0), stat(pct1(f("top4Rate")), "Top 4", good(f("top4Rate") >= 0.5)), stat(pct1(f("winRate")), "1re place", 0), stat(pct1(f("pickRate")), "Popularité", 0)]),
        units: ms(uv.chunks(9).map(|c| ms(c.to_vec())).collect()),
        traits: ms(tv.chunks(4).map(|c| ms(c.to_vec())).collect()),
    }
}

fn render_decks(a: &mut App) {
    let (Some(t), Some(r)) = (a.tft.data.clone(), a.tft.decks.as_ref()) else { return };
    let date = parse_iso_date(&r.at).map(|x| format!(", mises à jour le {}", date_long(x))).unwrap_or_default();
    a.data().set_tft_decks(TftDecksData {
        loading: false,
        error: ss(""),
        sub: ss(format!("Set {}", t.set)),
        text: ss(format!("Les meilleures compositions du moment d'après OP.GG, sur {} parties classées{date}. Les unités encadrées sont les carrys à équiper en priorité.", fr(r.games))),
        decks: ms(r.decks.iter().map(|k| deck_view(k, &t)).collect()),
    });
}

pub fn open_tft_profile(a: &mut App) {
    a.tft.games = None;
    a.tft.more = "idle".into();
    a.data().set_tft_profile(TftProfileData { loading: true, ..Default::default() });
    let gen = a.gen;
    spawn(async {
        if let Ok(t) = tft_data().await {
            on_ui(move |a| {
                a.tft.data = Some(t);
                if a.page() == "t-profil" {
                    render_tft_profile(a);
                }
            });
        }
    });
    spawn(async move {
        let r: Result<_, String> = async {
            let s = me().await?;
            let (q, raw, dd) = tokio::join!(ranked(&s.puuid), tft_history(&s.puuid, 0, 20), ddragon());
            Ok((s, q.unwrap_or_default(), raw?, dd?))
        }
        .await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match r {
                Ok((s, mut q, raw, dd)) => {
                    let list = parse_tft_games(&raw, &s.puuid);
                    if list.len() < 20 {
                        a.tft.more = "end".into();
                    }
                    a.tft.rank = q.remove("RANKED_TFT");
                    a.tft.sum = s;
                    a.tft.games = Some(list);
                    a.d = Some(dd);
                    render_tft_profile(a);
                }
                Err(e) => a.data().set_tft_profile(TftProfileData { error: ss(e), ..Default::default() }),
            }
        });
    });
}

fn trim_prefix(s: &str) -> String {
    let t = s.strip_prefix("TFT").unwrap_or(s);
    let t = t.strip_prefix("Set").unwrap_or(t);
    let t = t.trim_start_matches(|c: char| c.is_ascii_digit());
    t.strip_prefix('_').unwrap_or(t).to_string()
}

fn render_tft_profile(a: &mut App) {
    let (Some(d), Some(games)) = (a.d.clone(), a.tft.games.as_ref()) else { return };
    let t = a.tft.data.clone();
    let s = tft_summary(games);
    let mut quick = vec![];
    if s.games > 0 {
        quick = vec![stat(format!("{:.2}", s.average), "Place moyenne", good(s.average <= 4.5)), stat(pct0(s.top4), "Top 4", good(s.top4 >= 0.5)), stat(s.wins.to_string(), "1res places", 0), stat(s.games.to_string(), "Parties analysées", 0)];
    }
    let q = a.tft.rank.as_ref();
    let placed = has_rank(q);
    let rows: Vec<TftRow> = games
        .iter()
        .map(|g| {
            let mut units = g.units.clone();
            units.sort_by_key(|u| (std::cmp::Reverse(u.rarity), std::cmp::Reverse(u.tier)));
            let mut traits = g.traits.clone();
            traits.sort_by_key(|x| (std::cmp::Reverse(x.style), std::cmp::Reverse(x.num_units)));
            traits.truncate(6);
            TftRow {
                place: ss(g.placement.to_string()),
                top: g.placement <= 4,
                queue: ss(tft_queue(g.queue_id)),
                when: ss(format!("{}, {}", mmss(g.seconds), ago(g.at))),
                level: ss(format!("Niveau {}", g.level)),
                units: ms(units
                    .iter()
                    .map(|u| {
                        let info = t.as_ref().and_then(|x| x.units.get(&u.character_id));
                        TftUnitV { icon: ss(info.map(|i| i.icon.clone()).unwrap_or_default()), name: ss(info.map(|i| i.name.clone()).unwrap_or_else(|| trim_prefix(&u.character_id))), stars: ss("★".repeat(u.tier.max(1) as usize)), color: tft_color(info.map(|i| i.cost).unwrap_or(0)), core: false, items: ms(vec![]) }
                    })
                    .collect()),
                traits: ms(traits
                    .iter()
                    .map(|x| {
                        let info = t.as_ref().and_then(|y| y.traits.get(&x.name));
                        TftTraitV { icon: ss(info.map(|i| i.icon.clone()).unwrap_or_default()), n: ss(x.num_units.to_string()), name: ss(info.map(|i| i.name.clone()).unwrap_or_else(|| trim_prefix(&x.name))), style: x.style as i32 }
                    })
                    .collect()),
            }
        })
        .collect();
    let more = a.tft.more.clone();
    a.data().set_tft_profile(TftProfileData {
        loading: false,
        error: ss(""),
        avatar: ss(img_icon(&d, a.tft.sum.profile_icon_id)),
        level: ss(a.tft.sum.summoner_level.to_string()),
        name: ss(&a.tft.sum.game_name),
        tag: ss(&a.tft.sum.tag_line),
        quick: ms(quick),
        rank: RankRow {
            emblem: ss(if placed { img_emblem(&q.unwrap().tier) } else { String::new() }),
            has: placed,
            label: ss("Classé TFT"),
            value: ss(rank_label(q)),
            color: if placed { tier_color(&q.unwrap().tier) } else { Color::from_rgb_u8(0xee, 0xf0, 0xf7) },
            sub: ss(q.filter(|x| x.wins + x.losses > 0).map(|x| format!("{} top 1, {} parties", x.wins, x.wins + x.losses)).unwrap_or_default()),
        },
        empty: games.is_empty(),
        games: ms(rows),
        more: ss(if games.is_empty() || more == "end" {
            ""
        } else if more == "loading" {
            "Chargement…"
        } else {
            "Voir plus de parties"
        }),
        more_enabled: more == "idle",
    });
}

fn tft_more(a: &mut App) {
    if a.tft.more != "idle" {
        return;
    }
    let Some(len) = a.tft.games.as_ref().map(|g| g.len()) else { return };
    a.tft.more = "loading".into();
    render_tft_profile(a);
    let (puuid, gen) = (a.tft.sum.puuid.clone(), a.gen);
    spawn(async move {
        let r = tft_history(&puuid, len, 20).await;
        on_ui(move |a| {
            if a.gen != gen {
                return;
            }
            match r {
                Ok(raw) => {
                    let games = a.tft.games.get_or_insert_with(Vec::new);
                    let next: Vec<TftGame> = parse_tft_games(&raw, &puuid).into_iter().filter(|g| !games.iter().any(|x| x.id == g.id)).collect();
                    a.tft.more = if next.is_empty() { "end".into() } else { "idle".into() };
                    games.extend(next);
                }
                Err(_) => a.tft.more = "idle".into(),
            }
            render_tft_profile(a);
        });
    });
}

fn catalog() -> &'static Vec<Value> {
    static C: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../assets/avatars.json")).unwrap_or_default())
}

const AV_BASE: &str = "https://ethone.dev/avatars/library/";

fn render_picker(a: &mut App, open: bool) {
    let all = catalog();
    if a.picker.platform.is_empty() {
        a.picker.platform = all.first().and_then(|c| c["id"].as_str()).unwrap_or_default().to_string();
    }
    let st = &a.picker;
    let fallback = a.self_.as_ref().zip(a.d.as_ref()).map(|(s, d)| img_icon(d, s.profile_icon_id)).unwrap_or_default();
    let total: usize = all.iter().map(|c| c["items"].as_array().map(|l| l.len()).unwrap_or(0)).sum();
    let words: Vec<String> = fold(&st.q).split_whitespace().map(String::from).collect();
    let mut groups: Vec<(String, Vec<(String, String)>)> = vec![];
    if let Some(col) = all.iter().find(|c| c["id"] == st.platform.as_str()) {
        for it in col["items"].as_array().into_iter().flatten() {
            let (name, series, path) = (it[0].as_str().unwrap_or_default(), it[1].as_str().unwrap_or_default(), it[2].as_str().unwrap_or_default());
            let hay = fold(&format!("{name} {series}"));
            if words.iter().all(|w| hay.contains(w)) {
                let url = format!("{AV_BASE}{path}");
                match groups.iter_mut().find(|g| g.0 == series) {
                    Some(g) => g.1.push((name.to_string(), url)),
                    None => groups.push((series.to_string(), vec![(name.to_string(), url)])),
                }
            }
        }
    }
    let per_row = 9;
    let sections: Vec<AvatarSection> = groups
        .iter()
        .take(40)
        .map(|(title, items)| {
            let cells: Vec<AvatarItem> = items.iter().map(|(n, u)| AvatarItem { name: ss(n), url: ss(u), on: *u == st.choice }).collect();
            AvatarSection { title: ss(title), count: ss(items.len().to_string()), rows: ms(cells.chunks(per_row).map(|c| ms(c.to_vec())).collect()) }
        })
        .collect();
    let tabs: Vec<Opt> = all.iter().map(|c| opt(format!("picker-tab:{}", c["id"].as_str().unwrap_or_default()), c["label"].as_str().unwrap_or_default(), c["id"] == st.platform.as_str())).collect();
    a.data().set_picker(PickerData { open, choice: ss(if st.choice.is_empty() { fallback } else { st.choice.clone() }), count: ss(format!("Bibliothèque ETHONE, {total} avatars")), tabs: ms(tabs), sections: ms(sections) });
}

fn picker_file(a: &mut App) {
    let picked = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"]).pick_file();
    let Some(path) = picked else { return };
    match avatar_from_file(&path) {
        Ok(url) => {
            a.picker.choice = url;
            render_picker(a, true);
        }
        Err(e) => a.toast(format!("Image illisible : {e}")),
    }
}

pub fn avatar_from_file(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    avatar_from_bytes(&bytes)
}

pub fn avatar_from_bytes(bytes: &[u8]) -> Result<String, String> {
    use base64::Engine;
    let im = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let s = im.width().min(im.height());
    let out = im.crop_imm((im.width() - s) / 2, (im.height() - s) / 2, s, s).resize_exact(256, 256, image::imageops::FilterType::Triangle).to_rgba8();
    let mut png = std::io::Cursor::new(vec![]);
    out.write_to(&mut png, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png.get_ref())))
}

pub fn action(a: &mut App, key: &str) -> bool {
    if let Some(k) = key.strip_prefix("search:") {
        a.search.q = k.into();
        a.search.sel = 0;
        run_search(a);
    } else if key == "search-focus" {
        a.search.focused = true;
        run_search(a);
    } else if key == "search-blur" {
        a.search.focused = false;
        a.data().set_search_open(false);
    } else if key == "search-down" {
        a.search.sel = (a.search.sel + 1).min(a.search.keys.len().saturating_sub(1));
        run_search(a);
    } else if key == "search-up" {
        a.search.sel = a.search.sel.saturating_sub(1);
        run_search(a);
    } else if key == "search-enter" {
        search_pick(a, a.search.sel);
    } else if let Some(k) = key.strip_prefix("search-pick:") {
        search_pick(a, k.parse().unwrap_or(0));
    } else if let Some(k) = key.strip_prefix("ov:") {
        let mut s = settings();
        toggle_setting(&mut s, k);
        save_settings(&s);
        render_overlays(a);
    } else if let Some(k) = key.strip_prefix("set:") {
        let mut s = settings();
        toggle_setting(&mut s, k);
        save_settings(&s);
        render_settings(a);
    } else if let Some(k) = key.strip_prefix("setsrc:") {
        let mut s = settings();
        s.source = k.into();
        save_settings(&s);
        render_settings(a);
    } else if let Some(k) = key.strip_prefix("setrank:") {
        let mut s = settings();
        s.rank_filter = k.into();
        save_settings(&s);
        render_settings(a);
    } else if let Some(k) = key.strip_prefix("clear:") {
        let needs_confirm = k == "games" || k == "lp";
        if needs_confirm && a.settings.armed != key {
            a.settings.armed = key.to_string();
            render_settings(a);
            let armed = key.to_string();
            slint::Timer::single_shot(Duration::from_secs(3), move || {
                with_app(|a| {
                    if a.settings.armed == armed {
                        a.settings.armed.clear();
                        render_settings(a);
                    }
                });
            });
        } else {
            clear_data(a, k);
        }
    } else if let Some(u) = key.strip_prefix("open:") {
        if crate::net::open_url(u).is_err() {
            a.toast("Impossible d'ouvrir le lien");
        }
    } else if let Some(xy) = key.strip_prefix("click:").filter(|_| std::env::var("KAIRO_SNAP").is_ok()) {
        use slint::platform::{PointerEventButton, WindowEvent};
        let (x, y) = xy.split_once(',').unwrap_or(("0", "0"));
        let position = slint::LogicalPosition::new(x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0));
        let win = a.ui.as_weak();
        slint::Timer::single_shot(Duration::from_millis(1), move || {
            if let Some(ui) = win.upgrade() {
                ui.window().dispatch_event(WindowEvent::PointerMoved { position });
                ui.window().dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
                ui.window().dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
            }
        });
    } else if key == "tmore" {
        tft_more(a);
    } else if key == "picker-open" {
        a.picker.choice = avatar();
        a.picker.q.clear();
        a.data().set_avatar_q(ss(""));
        render_picker(a, true);
    } else if key == "picker-close" {
        render_picker(a, false);
    } else if let Some(k) = key.strip_prefix("picker-tab:") {
        a.picker.platform = k.into();
        render_picker(a, true);
    } else if let Some(k) = key.strip_prefix("picker-q:") {
        a.picker.q = k.into();
        render_picker(a, true);
    } else if let Some(k) = key.strip_prefix("picker-pick:") {
        a.picker.choice = k.into();
        render_picker(a, true);
    } else if key == "picker-reset" {
        a.picker.choice.clear();
        render_picker(a, true);
    } else if key == "picker-use" {
        if a.picker.choice.is_empty() {
            store::remove(&["rw-avatar".to_string()]);
        } else {
            store::set("rw-avatar", a.picker.choice.clone());
        }
        render_picker(a, false);
        a.render_shell();
        if a.page() == "profil" && a.profile.ready {
            crate::page_profile::render(a, false);
        }
    } else if key == "picker-file" {
        picker_file(a);
    } else {
        let _ = copy_text;
        return false;
    }
    true
}

pub fn start_script() {
    let (Ok(dir), steps) = (std::env::var("KAIRO_SNAP"), with_app(|a| std::mem::take(&mut a.script)).unwrap_or_default()) else { return };
    run_script(dir, steps, 0);
}

fn run_script(dir: String, steps: Vec<String>, i: usize) {
    let wait = std::env::var("KAIRO_SNAP_WAIT").ok().and_then(|x| x.parse().ok()).unwrap_or(5000);
    slint::Timer::single_shot(Duration::from_millis(wait), move || {
        with_app(|a| {
            if let Some(s) = steps.get(i) {
                let name = s.replace(['/', ':'], "_");
                if let Ok(b) = a.ui.window().take_snapshot() {
                    let _ = image::save_buffer(format!("{dir}/{i:02}_{name}.png"), b.as_bytes(), b.width(), b.height(), image::ColorType::Rgba8);
                }
            }
            if let Some(next) = steps.get(i + 1) {
                crate::dispatch_pub(a, next);
            }
        });
        if i + 1 < steps.len() {
            run_script(dir, steps, i + 1);
        } else {
            crate::overlay::snapshot_overlay(&format!("{dir}/overlay.png"));
            if let Some(ex) = crate::overlay::overlay_styles() {
                let _ = std::fs::write(format!("{dir}/overlay_exstyle.txt"), format!("{ex:#x}"));
            }
            let _ = slint::quit_event_loop();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn avatars_catalog_loads() {
        assert!(catalog().len() >= 2);
        assert!(catalog()[0]["items"].as_array().unwrap().len() > 10);
    }

    #[test]
    fn photo_is_cropped() {
        let mut out = std::io::Cursor::new(vec![]);
        image::RgbaImage::new(400, 200).write_to(&mut out, image::ImageFormat::Png).unwrap();
        let url = avatar_from_bytes(out.get_ref()).unwrap();
        let back = crate::images::parse_data_url(&url).unwrap();
        let im = image::load_from_memory(&back).unwrap();
        assert_eq!((im.width(), im.height()), (256, 256));
    }

    #[test]
    fn deck() {
        let t = TftData::default();
        let k = json!({"name": {"fr_FR": "Deck"}, "stat": {"opTier": "S", "deck": {"avgPlacement": 3.5, "top4Rate": 0.6, "winRate": 0.2, "pickRate": 0.1}}, "units": [{"key": "A", "tier": 2, "items": [], "isCore": true}], "traits": [{"key": "T", "style": 1, "numUnits": 3}, {"key": "Z", "style": 0, "numUnits": 1}], "badge": [{"key": "difficulty", "value": 3}]});
        let v = deck_view(&k, &t);
        assert_eq!(v.tier_tone, 2);
        assert_eq!(v.name.as_str(), "Deck");
    }

    #[test]
    fn prefixes() {
        assert_eq!(trim_prefix("TFT15_Ahri"), "Ahri");
        assert_eq!(trim_prefix("Set15_Mage"), "Mage");
    }
}
