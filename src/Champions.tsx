import { useEffect, useState } from "react";
import { ago, cs, DD, ddragon, Game, history, img, kda, me } from "./lcu";
import type { Go } from "./main";

const CLASSES: [string, string][] = [["", "Tous"], ["Assassin", "Assassins"], ["Fighter", "Combattants"], ["Mage", "Mages"], ["Marksman", "Tireurs"], ["Support", "Supports"], ["Tank", "Tanks"]];
type Row = { id: number; name: string; g: number; w: number; k: number; cs: number; last: number };
type Key = "name" | "g" | "wr" | "k" | "cs" | "last";

export default function Champions({ go }: { go: Go }) {
  const [d, setD] = useState<DD | null>(null);
  const [games, setGames] = useState<Game[] | null>(null);
  const [q, setQ] = useState("");
  const [cls, setCls] = useState("");
  const [played, setPlayed] = useState(true);
  const [sort, setSort] = useState<{ k: Key; desc: boolean }>({ k: "g", desc: true });

  useEffect(() => {
    ddragon().then(setD);
    me().then((s) => history(s.puuid, 100)).then(setGames, () => setGames([]));
  }, []);

  if (!d) return null;

  const stats = new Map<number, Row>();
  for (const g of games ?? []) {
    const p = g.participants[0];
    const r = stats.get(p.championId) ?? { id: p.championId, name: d.champs[p.championId]?.name ?? "?", g: 0, w: 0, k: 0, cs: 0, last: 0 };
    r.g++;
    r.w += p.stats.win ? 1 : 0;
    r.k += kda(p.stats);
    r.cs += cs(p.stats) / (g.gameDuration / 60);
    r.last = Math.max(r.last, g.gameCreation);
    stats.set(p.championId, r);
  }
  const showPlayed = played && stats.size > 0;
  const val = (r: Row, k: Key) => (k === "name" ? r.name : k === "wr" ? r.w / (r.g || 1) : k === "k" || k === "cs" ? r[k] / (r.g || 1) : r[k]);
  const rows = Object.entries(d.champs)
    .map(([id, c]) => stats.get(+id) ?? { id: +id, name: c.name, g: 0, w: 0, k: 0, cs: 0, last: 0 })
    .filter((r) => (!showPlayed || r.g) && (!cls || d.champs[r.id].tags.includes(cls)) && r.name.toLowerCase().includes(q.trim().toLowerCase()))
    .sort((a, b) => {
      const x = val(a, sort.k), y = val(b, sort.k);
      const c = typeof x === "string" ? x.localeCompare(y as string) : (x as number) - (y as number);
      return sort.desc ? -c : c;
    });
  if (!showPlayed) rows.sort((a, b) => a.name.localeCompare(b.name, "fr"));
  const th = (k: Key, l: string) => (
    <th className={sort.k === k ? "sorted" : ""} onClick={() => setSort({ k, desc: sort.k === k ? !sort.desc : k !== "name" })}>
      {l} {sort.k === k ? (sort.desc ? "▾" : "▴") : ""}
    </th>
  );

  return (
    <div className="champions">
      <div className="pagehead">
        <h1>Champions</h1>
        <p className="muted">
          {games === null ? "Analyse de tes 100 dernières parties…" : games.length ? `Tes stats sur tes ${games.length} dernières parties. Clique sur un champion pour voir et importer son build.` : "Clique sur un champion pour voir son build méta. Lance le client League pour ajouter tes stats personnelles."}
        </p>
      </div>
      <div className="filters">
        <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Rechercher un champion" />
        <div className="seg">
          {CLASSES.map(([k, l]) => (
            <button key={k} className={cls === k ? "on" : ""} onClick={() => setCls(k)}>{l}</button>
          ))}
        </div>
        {stats.size > 0 && (
          <div className="seg">
            <button className={played ? "on" : ""} onClick={() => setPlayed(true)}>Joués</button>
            <button className={!played ? "on" : ""} onClick={() => setPlayed(false)}>Tous</button>
          </div>
        )}
      </div>
      {!showPlayed ? (
        <div className="cgrid">
          {rows.map((r) => (
            <button key={r.id} onClick={() => go({ page: "champion", champ: r.id })}>
              <img src={img.champ(d, r.id)} />
              <span>{r.name}</span>
              {r.g > 0 && <small>{r.g} parties · {Math.round((r.w / r.g) * 100)}%</small>}
            </button>
          ))}
        </div>
      ) : (
        <table className="table">
          <thead>
            <tr>
              <th>#</th>
              {th("name", "Champion")}
              <th>Classe</th>
              {th("g", "Parties")}
              {th("wr", "Winrate")}
              {th("k", "KDA")}
              {th("cs", "CS/min")}
              {th("last", "Dernière")}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={r.id} onClick={() => go({ page: "champion", champ: r.id })}>
                <td className="muted">{i + 1}</td>
                <td><div className="cname"><img src={img.champ(d, r.id)} />{r.name}</div></td>
                <td className="muted">{CLASSES.find(([k]) => k === d.champs[r.id].tags[0])?.[1] ?? ""}</td>
                <td>{r.g}<div className="mini"><i style={{ width: `${(r.g / Math.max(...rows.map((x) => x.g))) * 100}%` }} /></div></td>
                <td className={r.w / r.g >= 0.5 ? "good" : "bad"}>{Math.round((r.w / r.g) * 100)}%</td>
                <td>{(r.k / r.g).toFixed(2)}</td>
                <td>{(r.cs / r.g).toFixed(1)}</td>
                <td className="muted">{ago(r.last)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
