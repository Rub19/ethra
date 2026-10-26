import { useEffect, useState } from "react";
import {
  ago, cs, DD, ddragon, Game, gameDetails, history, img, kda, me, mmss, n, Queue, queueNames, rankLabel, ranked, rankScore, summoner, Summoner,
} from "./lcu";
import type { Go } from "./main";

const QUEUES: [string, string][] = [["RANKED_SOLO_5x5", "Classé Solo/Duo"], ["RANKED_FLEX_SR", "Classé Flex"], ["CHERRY", "Arena"]];

function lpHistory(puuid: string, q?: Queue) {
  const key = `rw-lp-${puuid}`;
  let pts: { t: number; v: number }[] = [];
  try {
    pts = JSON.parse(localStorage.getItem(key) || "[]");
    if (q?.tier && q.tier !== "NONE" && pts.at(-1)?.v !== rankScore(q)) {
      pts = [...pts, { t: Date.now(), v: rankScore(q) }].slice(-200);
      localStorage.setItem(key, JSON.stringify(pts));
    }
  } catch {}
  return pts;
}

export function badges(g: Game, puuid: string) {
  const id = g.participantIdentities.find((x) => x.player.puuid === puuid)?.participantId;
  const p = g.participants.find((x) => x.participantId === id);
  if (!p) return [];
  const all = g.participants;
  const team = all.filter((x) => x.teamId === p.teamId);
  const max = (list: typeof all, f: (x: (typeof all)[0]) => number) => Math.max(...list.map(f)) === f(p);
  const out: string[] = [];
  if (max(all, (x) => kda(x.stats) * 3 + n(x.stats, "totalDamageDealtToChampions") / 4000)) out.push("MVP");
  else if (p.stats.win && max(team, (x) => kda(x.stats))) out.push("ACE");
  if (max(all, (x) => n(x.stats, "totalDamageDealtToChampions"))) out.push("Top dégâts");
  if (max(all, (x) => n(x.stats, "goldEarned"))) out.push("Cash Money");
  if (max(all, (x) => n(x.stats, "visionScore")) && g.mapId === 11) out.push("Vision");
  if (cs(p.stats) / (g.gameDuration / 60) >= 8) out.push("Farmeur");
  const multi = n(p.stats, "largestMultiKill");
  if (multi >= 2) out.push(["", "", "Double", "Triple", "Quadra", "Penta"][Math.min(multi, 5)]);
  return out;
}

function MatchRow({ g, puuid, d, queues, onOpen }: { g: Game; puuid: string; d: DD; queues: Record<number, string>; onOpen: (p: string) => void }) {
  const [full, setFull] = useState<Game | null>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    gameDetails(g.gameId).then(setFull, () => {});
  }, [g.gameId]);
  const p = g.participants[0];
  const s = p.stats;
  const min = g.gameDuration / 60;
  const remake = g.gameDuration < 300;
  const items = [0, 1, 2, 3, 4, 5, 6].map((k) => n(s, `item${k}`));
  const names = (team: number) =>
    full?.participants.filter((x) => x.teamId === team).map((x) => {
      const who = full.participantIdentities.find((i) => i.participantId === x.participantId)?.player;
      return (
        <button key={x.participantId} className={`pl ${who?.puuid === puuid ? "me" : ""}`} onClick={() => who?.puuid && onOpen(who.puuid)}>
          <img src={img.champ(d, x.championId)} />
          <span>{who?.gameName || who?.summonerName || "?"}</span>
        </button>
      );
    });
  return (
    <div className={`match ${remake ? "remake" : s.win ? "win" : "loss"}`}>
      <div className="mrow" onClick={() => setOpen(!open)}>
        <img className="champ" src={img.champ(d, p.championId)} />
        <div className="mmain">
          <div className="mtitle">
            <b>{remake ? "Remake" : s.win ? "Victoire" : "Défaite"}</b>
            <span>· {queues[g.queueId] ?? `File ${g.queueId}`} · {mmss(g.gameDuration)} · {ago(g.gameCreation)}</span>
          </div>
          <div className="mstats">
            <div><b>{kda(s).toFixed(1)} KDA</b><small>{n(s, "kills")} / {n(s, "deaths")} / {n(s, "assists")}</small></div>
            <div><b>{(cs(s) / min).toFixed(1)} CS/min</b><small>{cs(s)} CS</small></div>
            <div><b>{Math.round(n(s, "totalDamageDealtToChampions") / min).toLocaleString("fr")} DPM</b><small>{n(s, "totalDamageDealtToChampions").toLocaleString("fr")}</small></div>
            <div><b>{Math.round(n(s, "goldEarned") / min)} GPM</b><small>{n(s, "visionScore")} vision</small></div>
            <div className="spells">
              <img src={img.spell(d, p.spell1Id)} />
              <img src={img.spell(d, p.spell2Id)} />
              <img src={img.rune(d, n(s, "perk0"))} />
              <img src={img.rune(d, n(s, "perkSubStyle"))} />
            </div>
            <div className="items">{items.map((id, i) => (id ? <img key={i} src={img.item(d, id)} /> : <span key={i} />))}</div>
          </div>
          <div className="badges">{full && badges(full, puuid).map((b) => <span key={b} className={b === "MVP" || b === "ACE" ? "gold" : ""}>{b}</span>)}</div>
        </div>
        <div className="teams">
          <div>{names(100)}</div>
          <div>{names(200)}</div>
        </div>
      </div>
      {open && full && <Scoreboard g={full} d={d} puuid={puuid} onOpen={onOpen} />}
    </div>
  );
}

function Scoreboard({ g, d, puuid, onOpen }: { g: Game; d: DD; puuid: string; onOpen: (p: string) => void }) {
  const min = g.gameDuration / 60;
  const maxDmg = Math.max(...g.participants.map((x) => n(x.stats, "totalDamageDealtToChampions")));
  return (
    <div className="board">
      {[100, 200].map((t) => (
        <table key={t}>
          <tbody>
            {g.participants.filter((x) => x.teamId === t).map((x) => {
              const who = g.participantIdentities.find((i) => i.participantId === x.participantId)?.player;
              const dmg = n(x.stats, "totalDamageDealtToChampions");
              return (
                <tr key={x.participantId} className={who?.puuid === puuid ? "me" : ""}>
                  <td><img src={img.champ(d, x.championId)} /></td>
                  <td className="name" onClick={() => who?.puuid && onOpen(who.puuid)}>{who?.gameName ?? "?"}<small>#{who?.tagLine}</small></td>
                  <td>{n(x.stats, "kills")}/{n(x.stats, "deaths")}/{n(x.stats, "assists")}</td>
                  <td>{cs(x.stats)} ({(cs(x.stats) / min).toFixed(1)})</td>
                  <td className="bar"><i style={{ width: `${(dmg / maxDmg) * 100}%` }} />{dmg.toLocaleString("fr")}</td>
                  <td className="items">{[0, 1, 2, 3, 4, 5, 6].map((k) => n(x.stats, `item${k}`)).map((id, i) => (id ? <img key={i} src={img.item(d, id)} /> : <span key={i} />))}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ))}
    </div>
  );
}

export default function Profile({ puuid, go }: { puuid: string | null; go: Go }) {
  const onOpen = (p: string) => go({ page: "profil", puuid: p });
  const [sum, setSum] = useState<Summoner | null>(null);
  const [ranks, setRanks] = useState<Record<string, Queue>>({});
  const [games, setGames] = useState<Game[] | null>(null);
  const [d, setD] = useState<DD | null>(null);
  const [queues, setQueues] = useState<Record<number, string>>({});
  const [err, setErr] = useState("");
  const [sub, setSub] = useState<"apercu" | "pool">("apercu");

  useEffect(() => {
    setGames(null);
    setErr("");
    (async () => {
      const s = puuid ? await summoner(puuid) : await me();
      setSum(s);
      const [r, h, dd, q] = await Promise.all([ranked(s.puuid).catch(() => ({})), history(s.puuid, 20), ddragon(), queueNames().catch(() => ({}))]);
      setRanks(r);
      setD(dd);
      setQueues(q);
      setGames(h);
    })().catch((e) => setErr(String(e)));
  }, [puuid]);

  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!sum || !d) return <div className="empty"><p>Chargement…</p></div>;

  const lp = lpHistory(sum.puuid, ranks.RANKED_SOLO_5x5);
  const pool = new Map<number, { g: number; w: number; k: number; cs: number }>();
  for (const g of games ?? []) {
    const p = g.participants[0];
    const c = pool.get(p.championId) ?? { g: 0, w: 0, k: 0, cs: 0 };
    pool.set(p.championId, { g: c.g + 1, w: c.w + (p.stats.win ? 1 : 0), k: c.k + kda(p.stats), cs: c.cs + cs(p.stats) / (g.gameDuration / 60) });
  }
  const champs = [...pool.entries()].sort((a, b) => b[1].g - a[1].g);
  const wins = games?.filter((g) => g.participants[0].stats.win).length ?? 0;

  return (
    <div className="profile">
      <div className="phead">
        <div className="avatar"><img src={img.icon(d, sum.profileIconId)} /><span>{sum.summonerLevel}</span></div>
        <h1>{sum.gameName} <span>#{sum.tagLine}</span></h1>
      </div>
      <div className="subtabs">
        <button className={sub === "apercu" ? "on" : ""} onClick={() => setSub("apercu")}>Aperçu</button>
        <button className={sub === "pool" ? "on" : ""} onClick={() => setSub("pool")}>Champions</button>
      </div>
      <div className="cols">
        <aside>
          <section className="card">
            {QUEUES.map(([k, l]) => {
              const q = ranks[k];
              return (
                <div key={k} className="rank">
                  <span>{l}</span>
                  <b>{rankLabel(q)}</b>
                  {q && q.wins + q.losses > 0 && <small>{q.wins}V - {q.losses}D · {Math.round((q.wins / (q.wins + q.losses)) * 100)}%</small>}
                </div>
              );
            })}
          </section>
          {lp.length > 1 && (
            <section className="card">
              <h3>Historique LP (Solo)</h3>
              <Spark pts={lp.map((p) => p.v)} />
            </section>
          )}
          <section className="card">
            <h3>20 dernières parties</h3>
            <div className="wr"><i style={{ width: `${games?.length ? (wins / games.length) * 100 : 0}%` }} /></div>
            <small className="muted">{wins}V - {(games?.length ?? 0) - wins}D</small>
            {champs.slice(0, 6).map(([c, x]) => (
              <div key={c} className="perf link" onClick={() => go({ page: "champion", champ: c })}>
                <img src={img.champ(d, c)} />
                <div><b>{d.champs[c]?.name}</b><small>{(x.k / x.g).toFixed(1)} KDA</small></div>
                <div className="r"><b className={x.w / x.g >= 0.5 ? "good" : "bad"}>{Math.round((x.w / x.g) * 100)}%</b><small>{x.g} parties</small></div>
              </div>
            ))}
          </section>
        </aside>
        <div className="list">
          {!games ? <p className="muted">Chargement de l'historique…</p> : sub === "apercu" ? (
            games.map((g) => <MatchRow key={g.gameId} g={g} puuid={sum.puuid} d={d} queues={queues} onOpen={onOpen} />)
          ) : (
            <table className="pool">
              <thead><tr><th>Champion</th><th>Parties</th><th>Winrate</th><th>KDA</th><th>CS/min</th></tr></thead>
              <tbody>
                {champs.map(([c, x]) => (
                  <tr key={c} className="link" onClick={() => go({ page: "champion", champ: c })}>
                    <td><img src={img.champ(d, c)} /> {d.champs[c]?.name}</td>
                    <td>{x.g}</td>
                    <td className={x.w / x.g >= 0.5 ? "good" : "bad"}>{Math.round((x.w / x.g) * 100)}%</td>
                    <td>{(x.k / x.g).toFixed(2)}</td>
                    <td>{(x.cs / x.g).toFixed(1)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}

function Spark({ pts }: { pts: number[] }) {
  const lo = Math.min(...pts), hi = Math.max(...pts), w = 260, h = 60;
  const xy = pts.map((v, i) => `${(i / (pts.length - 1)) * w},${h - ((v - lo) / Math.max(1, hi - lo)) * h}`).join(" ");
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="spark">
      <polyline points={xy} fill="none" stroke="var(--accent)" strokeWidth="2" />
    </svg>
  );
}
