import { useEffect, useState } from "react";
import { DD, ddragon, img } from "./lcu";
import { meta, MetaRow, POS_LABEL, TIER_LABEL } from "./opgg";
import type { Go } from "./main";

type Key = "tier" | "winRate" | "pickRate" | "banRate" | "play" | "kda";
const pct = (x: number) => `${(x * 100).toFixed(1)}%`;

export default function TierList({ go }: { go: Go }) {
  const [d, setD] = useState<DD | null>(null);
  const [rows, setRows] = useState<MetaRow[] | null>(null);
  const [err, setErr] = useState("");
  const [pos, setPos] = useState("all");
  const [q, setQ] = useState("");
  const [sort, setSort] = useState<{ k: Key; desc: boolean }>({ k: "tier", desc: false });

  useEffect(() => {
    ddragon().then((dd) => {
      setD(dd);
      return meta(dd).then(setRows);
    }).catch((e) => setErr(String(e)));
  }, []);

  if (err) return <div className="empty"><h2>OP.GG ne répond pas</h2><p>{err}</p></div>;
  if (!d || !rows) return <div className="empty"><p>Chargement de la méta…</p></div>;

  const list = rows
    .filter((r) => (pos === "all" || r.position === pos) && r.name.toLowerCase().includes(q.trim().toLowerCase()))
    .sort((a, b) => {
      const c = sort.k === "tier" ? a.tier - b.tier || a.rank - b.rank : a[sort.k] - b[sort.k];
      return sort.desc ? -c : c;
    });
  const th = (k: Key, l: string) => (
    <th className={sort.k === k ? "sorted" : ""} onClick={() => setSort({ k, desc: sort.k === k ? !sort.desc : k !== "tier" })}>
      {l} {sort.k === k ? (sort.desc ? "▾" : "▴") : ""}
    </th>
  );

  return (
    <div className="tierlist">
      <div className="pagehead">
        <h1>Tier list <span className="muted">Classé Solo/Duo</span></h1>
        <p className="muted">Statistiques mondiales du patch en cours, fournies par OP.GG. Clique sur un champion pour son build.</p>
      </div>
      <div className="filters">
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Rechercher un champion" />
        <div className="seg">
          {[["all", "Tous"], ...Object.entries(POS_LABEL)].map(([k, l]) => (
            <button key={k} className={pos === k ? "on" : ""} onClick={() => setPos(k)}>{l}</button>
          ))}
        </div>
      </div>
      <table className="table">
        <thead>
          <tr>
            <th>#</th>
            <th>Rôle</th>
            <th>Champion</th>
            {th("tier", "Tier")}
            {th("winRate", "Winrate")}
            {th("pickRate", "Pick")}
            {th("banRate", "Ban")}
            {th("kda", "KDA")}
            {th("play", "Parties")}
            <th>Évolution</th>
          </tr>
        </thead>
        <tbody>
          {list.map((r, i) => {
            const delta = r.rankPrev ? r.rankPrev - r.rank : 0;
            return (
              <tr key={`${r.position}-${r.name}`} onClick={() => r.champ && go({ page: "champion", champ: r.champ, role: r.position })}>
                <td className="muted">{i + 1}</td>
                <td className="muted">{POS_LABEL[r.position]}</td>
                <td className="cname">{r.champ ? <img src={img.champ(d, r.champ)} /> : null}{d.champs[r.champ]?.name ?? r.name}</td>
                <td><span className={`tier t${r.tier}`}>{TIER_LABEL[r.tier] ?? r.tier}</span></td>
                <td className={r.winRate >= 0.5 ? "good" : "bad"}>{pct(r.winRate)}</td>
                <td>{pct(r.pickRate)}</td>
                <td>{pct(r.banRate)}</td>
                <td>{r.kda.toFixed(2)}</td>
                <td>{r.play.toLocaleString("fr")}</td>
                <td className={delta > 0 ? "good" : delta < 0 ? "bad" : "muted"}>{delta > 0 ? `▲ ${delta}` : delta < 0 ? `▼ ${-delta}` : "–"}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
