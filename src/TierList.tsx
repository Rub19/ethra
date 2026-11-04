import { useEffect, useState } from "react";
import { favChamps, pressable, sortProps, toggleFavChamp } from "./util";
import { DD, ddragon, img } from "./lcu";
import { meta, MetaRow, POS_LABEL, TIER_LABEL } from "./opgg";
import type { Go } from "./main";
import Skeleton from "./Skeleton";

type Key = "tier" | "winRate" | "pickRate" | "banRate" | "play" | "kda";
const pct = (x: number) => `${(x * 100).toFixed(1)}%`;

export default function TierList({ go }: { go: Go }) {
  const [d, setD] = useState<DD | null>(null);
  const [rows, setRows] = useState<MetaRow[] | null>(null);
  const [err, setErr] = useState("");
  const [pos, setPos] = useState("all");
  const [q, setQ] = useState("");
  const [favs, setFavs] = useState(favChamps);
  const [favOnly, setFavOnly] = useState(false);
  const [sort, setSort] = useState<{ k: Key; desc: boolean }>({ k: "tier", desc: false });

  useEffect(() => {
    ddragon().then((dd) => {
      setD(dd);
      return meta(dd).then(setRows);
    }).catch((e) => setErr(String(e)));
  }, []);

  if (err) return <div className="empty"><h2>OP.GG ne répond pas</h2><p>{err}</p></div>;
  if (!d || !rows) return <div className="tierlist"><div className="pagehead"><h1>Tier list</h1></div><Skeleton rows={10} height={56} /></div>;

  const list = rows
    .filter((r) => (!favOnly || favs.includes(r.champ)) && (pos === "all" || r.position === pos) && r.name.toLowerCase().includes(q.trim().toLowerCase()))
    .sort((a, b) => {
      const c = sort.k === "tier" ? a.tier - b.tier || a.rank - b.rank : a[sort.k] - b[sort.k];
      return sort.desc ? -c : c;
    });
  const moves = rows.some((r) => r.rankPrev && r.rankPrev !== r.rank);
  const th = (k: Key, l: string) => (
    <th className={sort.k === k ? "sorted" : ""} {...sortProps(sort.k === k, sort.desc, () => setSort({ k, desc: sort.k === k ? !sort.desc : k !== "tier" }))}>
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
          <button className={favOnly ? "on" : ""} aria-pressed={favOnly} onClick={() => setFavOnly(!favOnly)}>Favoris{favs.length > 0 && ` (${favs.length})`}</button>
        </div>
        <div className="seg">
          {[["all", "Tous"], ...Object.entries(POS_LABEL)].map(([k, l]) => (
            <button key={k} className={`rolebtn ${pos === k ? "on" : ""}`} onClick={() => setPos(k)}>{k !== "all" && <img src={img.role(k)} />}{l}</button>
          ))}
        </div>
      </div>
      {list.length === 0 && <div className="empty"><h2>{favOnly && !favs.length ? "Aucun favori" : "Aucun champion"}</h2><p>{favOnly && !favs.length ? "Clique sur l'étoile d'un champion pour le retrouver ici et dans la recherche." : "Change de rôle ou de recherche."}</p></div>}
      <table className="table" hidden={list.length === 0}>
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
            {moves && <th>Évolution</th>}
          </tr>
        </thead>
        <tbody>
          {list.map((r, i) => {
            const delta = r.rankPrev ? r.rankPrev - r.rank : 0;
            return (
              <tr key={`${r.position}-${r.name}`} {...pressable(() => r.champ && go({ page: "champion", champ: r.champ, role: r.position }))}>
                <td className="muted">{i + 1}</td>
                <td><span className="role"><img src={img.role(r.position)} />{POS_LABEL[r.position]}</span></td>
                <td><div className="cname">{r.champ ? <img src={img.champ(d, r.champ)} /> : null}{d.champs[r.champ]?.name ?? r.name}{r.champ ? <button className={`favstar ${favs.includes(r.champ) ? "on" : ""}`} aria-pressed={favs.includes(r.champ)} aria-label={favs.includes(r.champ) ? "Retirer des favoris" : "Ajouter aux favoris"} onKeyDown={(e) => e.stopPropagation()} onClick={(e) => (e.stopPropagation(), setFavs(toggleFavChamp(r.champ)))}><svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="m12 3.5 2.7 5.5 6 .9-4.3 4.2 1 6-5.4-2.8-5.4 2.8 1-6-4.3-4.2 6-.9z" /></svg></button> : null}</div></td>
                <td><span className={`tier t${r.tier}`}>{TIER_LABEL[r.tier] ?? r.tier}</span></td>
                <td className={r.winRate >= 0.5 ? "good" : "bad"}>{pct(r.winRate)}</td>
                <td>{pct(r.pickRate)}</td>
                <td>{pct(r.banRate)}</td>
                <td>{r.kda.toFixed(2)}</td>
                <td>{r.play.toLocaleString("fr")}</td>
                {moves && <td className={delta > 0 ? "good" : delta < 0 ? "bad" : "muted"}>{delta > 0 ? `▲ ${delta}` : delta < 0 ? `▼ ${-delta}` : "–"}</td>}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
