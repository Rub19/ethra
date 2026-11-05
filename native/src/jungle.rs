const SIZE: f64 = 14870.0;
const SCUTTLES: [(f64, f64); 2] = [(4400.0, 9600.0), (10500.0, 5200.0)];

fn camp(k: &str) -> (&'static str, (f64, f64)) {
    match k {
        "blue" => ("Buff bleu", (3821.0, 8101.0)),
        "gromp" => ("Gromp", (2288.0, 8448.0)),
        "wolves" => ("Loups", (3783.0, 6495.0)),
        "raptors" => ("Raptors", (6823.0, 5508.0)),
        "red" => ("Buff rouge", (7765.0, 4020.0)),
        _ => ("Krugs", (8394.0, 2641.0)),
    }
}

pub struct Route {
    pub steps: Vec<(&'static str, (f64, f64))>,
    pub side: &'static str,
}

pub fn route(team: &str, start: &str) -> Route {
    let order = if start == "red" { ["red", "krugs", "raptors", "wolves", "blue", "gromp"] } else { ["blue", "gromp", "wolves", "raptors", "red", "krugs"] };
    let mirror = |(x, y): (f64, f64)| if team == "blue" { (x, y) } else { (SIZE - x, SIZE - y) };
    let mut steps: Vec<(&'static str, (f64, f64))> = order.iter().map(|k| { let c = camp(k); (c.0, mirror(c.1)) }).collect();
    let last = steps[steps.len() - 1].1;
    let dist = |a: (f64, f64)| (a.0 - last.0).hypot(a.1 - last.1);
    let crab = if dist(SCUTTLES[0]) <= dist(SCUTTLES[1]) { SCUTTLES[0] } else { SCUTTLES[1] };
    let side = if crab.1 > crab.0 { "top" } else { "bot" };
    steps.push(("Crabe", crab));
    Route { steps, side }
}

pub fn pt((x, y): (f64, f64)) -> (f64, f64) {
    (x / SIZE * 100.0, 100.0 - y / SIZE * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes() {
        let r = route("blue", "red");
        assert_eq!(r.steps[0].0, "Buff rouge");
        assert_eq!(r.steps.last().unwrap().0, "Crabe");
        assert_eq!(r.steps.len(), 7);
        assert_eq!(r.side, "top");
        assert_eq!(route("blue", "blue").side, "bot");
        assert_eq!(route("red", "red").steps[0].1 .0, SIZE - 7765.0);
    }
}
