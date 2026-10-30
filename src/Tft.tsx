import { useEffect, useState } from "react";
import Skeleton from "./Skeleton";
import { tftData, TftData, TftDeck, tftDecks } from "./tftApi";

const pct = (x: number) => `${(x * 100).toFixed(1)}%`;
const TRAIT_STYLE = ["", "bronze", "silver", "gold", "prism"];

function Deck({ k, t }: { k: TftDeck; t: TftData }) {
  const s = k.stat.deck;
  const units = [...k.units].sort((a, b) => (t.units[a.key]?.cost ?? 0) - (t.units[b.key]?.cost ?? 0));
  const traits = k.traits.filter((x) => x.style > 0).sort((a, b) => b.style - a.style || b.numUnits - a.numUnits);
  const difficulty = k.badge.find((b) => b.key === "difficulty")?.value as number | undefined;
  const reroll = k.badge.find((b) => b.key === "reroll")?.value as number | undefined;
  return (
    <section className="card tdeck">
      <div className="tdhead">
        <span className={`ttier t-${k.stat.opTier.toLowerCase()}`}>{k.stat.opTier}</span>
        <h3>{k.name.fr_FR ?? k.name.en_US}</h3>
        <div className="tdtags">
          {difficulty ? <span>Difficulté {difficulty}/5</span> : null}
          {reroll ? <span>Reroll niveau {reroll}</span> : null}
        </div>
      </div>
      <div className="tdstats">
        <div><b>{s.avgPlacement.toFixed(2)}</b><small>Place moyenne</small></div>
        <div><b className={s.top4Rate >= 0.5 ? "good" : "bad"}>{pct(s.top4Rate)}</b><small>Top 4</small></div>
        <div><b>{pct(s.winRate)}</b><small>1re place</small></div>
        <div><b>{pct(s.pickRate)}</b><small>Popularité</small></div>
      </div>
      <div className="tdunits">
        {units.map((u, i) => {
          const info = t.units[u.key];
          return (
            <div key={`${u.key}-${i}`} className={`tunit c${info?.cost ?? 0} ${u.isCore ? "core" : ""}`} title={info?.name ?? u.key}>
              <span className="tstars">{"★".repeat(Math.max(1, u.tier))}</span>
              {info?.icon ? <img src={info.icon} alt={info.name} /> : <span className="tph">{(info?.name ?? "?")[0]}</span>}
              <small>{info?.name ?? u.key.replace(/^DA_|18/g, "")}</small>
              <div className="titems">
                {u.items.map((it, j) => t.items[it]?.icon ? <img key={j} src={t.items[it].icon} title={t.items[it].name} alt={t.items[it].name} /> : null)}
              </div>
            </div>
          );
        })}
      </div>
      <div className="tdtraits">
        {traits.map((x) => {
          const info = t.traits[x.key];
          return (
            <span key={x.key} className={`ttrait ${TRAIT_STYLE[x.style] ?? ""}`} title={info?.name}>
              {info?.icon && <img src={info.icon} alt="" />}
              <b>{x.numUnits}</b> {info?.name ?? x.key}
            </span>
          );
        })}
      </div>
    </section>
  );
}

export default function TftDecks() {
  const [t, setT] = useState<TftData | null>(null);
  const [r, setR] = useState<{ decks: TftDeck[]; games: number; at: string } | null>(null);
  const [err, setErr] = useState("");

  useEffect(() => {
    tftData().then(setT, (e) => setErr(String(e)));
    tftDecks().then(setR, (e) => setErr(String(e)));
  }, []);

  if (err) return <div className="empty"><h2>Données TFT indisponibles</h2><p>{err}</p></div>;
  if (!t || !r) return <div className="tierlist"><div className="pagehead"><h1>Decks méta</h1></div><Skeleton rows={4} height={200} /></div>;

  return (
    <div className="tft">
      <div className="pagehead">
        <h1>Decks méta <span className="muted">Set {t.set}</span></h1>
        <p className="muted">
          Les meilleures compositions du moment d'après OP.GG, sur {r.games?.toLocaleString("fr")} parties classées
          {r.at ? `, mises à jour le ${new Date(r.at).toLocaleDateString("fr", { day: "numeric", month: "long" })}` : ""}. Les unités encadrées sont les carrys à équiper en priorité.
        </p>
      </div>
      <div className="tdecks">
        {r.decks.map((k) => <Deck key={k.id} k={k} t={t} />)}
      </div>
    </div>
  );
}
