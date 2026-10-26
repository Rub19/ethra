import { useEffect, useState } from "react";
import {
  ago, cs, DD, ddragon, Game, gameDetails, history, img, kda, me, mmss, n, Queue, queueNames, rankLabel, ranked, rankScore, summoner, Summoner,
} from "./lcu";
import type { Go } from "./main";
import { useAvatar } from "./Account";
import Skeleton from "./Skeleton";

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

function MatchRow({ g, puuid, d, queues, onOpen, onMatch }: { g: Game; puuid: string; d: DD; queues: Record<number, string>; onOpen: (p: string) => void; onMatch: () => void }) {
  const [full, setFull] = useState<Game | null>(null);
  useEffect(() => {
    gameDetails(g.gameId).then(setFull, () => {});
  }, [g.gameId]);
  const p = g.participants[0];
  const s = p.stats;
  const min = g.gameDuration / 60;
  const remake = g.gameDuration < 300;
  const result = remake ? "remake" : s.win ? "win" : "loss";
  const items = [0, 1, 2, 3, 4, 5].map((k) => n(s, `item${k}`));
  const trinket = n(s, "item6");
  const sr = g.mapId === 11;
  const me = full?.participants.find((x) => x.participantId === full.participantIdentities.find((i) => i.player.puuid === puuid)?.participantId);
  const teamKills = me ? full!.participants.filter((x) => x.teamId === me.teamId).reduce((a, x) => a + n(x.stats, "kills"), 0) : 0;
  const kp = teamKills ? Math.round(((n(s, "kills") + n(s, "assists")) / teamKills) * 100) : null;
  const icons = [
    d.spells[p.spell1Id] && img.spell(d, p.spell1Id),
    d.spells[p.spell2Id] && img.spell(d, p.spell2Id),
    d.runes[n(s, "perk0")] && img.rune(d, n(s, "perk0")),
    d.runes[n(s, "perkSubStyle")] && img.rune(d, n(s, "perkSubStyle")),
  ];
  const names = (team: number) =>
    full?.participants.filter((x) => x.teamId === team).map((x) => {
      const who = full.participantIdentities.find((i) => i.participantId === x.participantId)?.player;
      return (
        <button key={x.participantId} className={`pl ${who?.puuid === puuid ? "me" : ""}`} onClick={(e) => (e.stopPropagation(), who?.puuid && onOpen(who.puuid))}>
          <img src={img.champ(d, x.championId)} />
          <span>{who?.gameName || who?.summonerName || "?"}</span>
        </button>
      );
    });
  return (
    <div className={`match ${result}`} onClick={onMatch} role="button" tabIndex={0} onKeyDown={(e) => e.key === "Enter" && onMatch()}>
      <div className="mres">
        <b>{remake ? "Remake" : s.win ? "Victoire" : "Défaite"}</b>
        <small>{queues[g.queueId] ?? `File ${g.queueId}`}</small>
        <small>{mmss(g.gameDuration)} · {ago(g.gameCreation)}</small>
      </div>
      <div className="mchamp">
        <img src={img.champ(d, p.championId)} />
        <span>{n(s, "champLevel")}</span>
      </div>
      <div className="micons">{icons.filter(Boolean).map((u, i) => <img key={i} src={u as string} />)}</div>
      <div className="mkda">
        <b>{n(s, "kills")} <i>/</i> <em>{n(s, "deaths")}</em> <i>/</i> {n(s, "assists")}</b>
        <small><span className={kda(s) >= 3 ? "good" : ""}>{kda(s).toFixed(2)} KDA</span>{kp !== null && ` · ${kp}% KP`}</small>
      </div>
      <div className="mnums">
        <span><b>{(cs(s) / min).toFixed(1)}</b> CS/min</span>
        <span><b>{(n(s, "totalDamageDealtToChampions") / 1000).toFixed(1)}k</b> dégâts</span>
        <span>{sr ? <><b>{n(s, "visionScore")}</b> vision</> : <><b>{(n(s, "goldEarned") / 1000).toFixed(1)}k</b> or</>}</span>
      </div>
      <div className="mitems">
        {items.map((id, i) => (id ? <img key={i} src={img.item(d, id)} title={d.items[id]?.name} /> : <span key={i} />))}
        {trinket ? <img className="trk" src={img.item(d, trinket)} /> : <span className="trk" />}
      </div>
      <div className="mbadges">{full && badges(full, puuid).slice(0, 3).map((b) => <span key={b} className={b === "MVP" || b === "ACE" ? "gold" : ""}>{b}</span>)}</div>
      <div className="teams">
        <div>{names(100)}</div>
        <div>{names(200)}</div>
      </div>
    </div>
  );
}

export function Scoreboard({ g, d, puuid, onOpen }: { g: Game; d: DD; puuid: string; onOpen: (p: string) => void }) {
  const min = g.gameDuration / 60;
  const maxDmg = Math.max(...g.participants.map((x) => n(x.stats, "totalDamageDealtToChampions")));
  const myId = g.participantIdentities.find((i) => i.player.puuid === puuid)?.participantId;
  const myTeam = g.participants.find((x) => x.participantId === myId)?.teamId ?? 100;
  const teams = [myTeam, myTeam === 100 ? 200 : 100];
  return (
    <table className="board">
      <colgroup>
        <col style={{ width: 52 }} />
        <col />
        <col style={{ width: 120 }} />
        <col style={{ width: 70 }} />
        <col style={{ width: 120 }} />
        <col style={{ width: 170 }} />
        <col style={{ width: 252 }} />
      </colgroup>
      {teams.map((t) => {
        const list = g.participants.filter((x) => x.teamId === t);
        const win = list[0]?.stats.win;
        const k = list.reduce((a, x) => a + n(x.stats, "kills"), 0);
        const de = list.reduce((a, x) => a + n(x.stats, "deaths"), 0);
        const as = list.reduce((a, x) => a + n(x.stats, "assists"), 0);
        const teamKills = Math.max(1, k);
        return (
          <tbody key={t}>
            <tr className="bhead">
              <th colSpan={2}><b className={win ? "good" : "bad"}>{win ? "Victoire" : "Défaite"}</b> · {t === myTeam ? "Ton équipe" : "Adversaires"}</th>
              <th>{k} / {de} / {as}</th>
              <th>KP</th>
              <th>CS</th>
              <th>Dégâts</th>
              <th>Objets</th>
            </tr>
            {list.map((x) => {
              const who = g.participantIdentities.find((i) => i.participantId === x.participantId)?.player;
              const dmg = n(x.stats, "totalDamageDealtToChampions");
              return (
                <tr key={x.participantId} className={who?.puuid === puuid ? "me" : ""}>
                  <td><img className="bc" src={img.champ(d, x.championId)} /></td>
                  <td className="name" onClick={() => who?.puuid && onOpen(who.puuid)}>{who?.gameName ?? "?"}<small>#{who?.tagLine}</small></td>
                  <td className="c"><b>{n(x.stats, "kills")} / <span className="bad">{n(x.stats, "deaths")}</span> / {n(x.stats, "assists")}</b><small>{kda(x.stats).toFixed(2)} KDA</small></td>
                  <td className="c">{Math.round(((n(x.stats, "kills") + n(x.stats, "assists")) / teamKills) * 100)}%</td>
                  <td className="c">{cs(x.stats)}<small>{(cs(x.stats) / min).toFixed(1)}/min</small></td>
                  <td className="c">{dmg.toLocaleString("fr")}<i className="dbar2"><i style={{ width: `${(dmg / maxDmg) * 100}%` }} /></i></td>
                  <td className="items">{[0, 1, 2, 3, 4, 5, 6].map((k) => n(x.stats, `item${k}`)).map((id, i) => (id ? <img key={i} src={img.item(d, id)} /> : <span key={i} />))}</td>
                </tr>
              );
            })}
          </tbody>
        );
      })}
    </table>
  );
}

export default function Profile({ puuid, go }: { puuid: string | null; go: Go }) {
  const onOpen = (p: string) => go({ page: "profil", puuid: p });
  const [avatar] = useAvatar();
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
  if (!sum || !d) return <div className="profile"><div className="skel"><div style={{ height: 150 }} /></div><Skeleton rows={6} /></div>;

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
      <div className="phero" style={champs[0] ? { backgroundImage: `linear-gradient(90deg, var(--bg) 18%, #0b0f17cc 50%, #0b0f1755), url(https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${d.champs[champs[0][0]]?.id}_0.jpg)` } : undefined}>
        <div className="avatar"><img src={(!puuid && avatar) || img.icon(d, sum.profileIconId)} /><span>{sum.summonerLevel}</span></div>
        <div className="pid">
          <h1>{sum.gameName} <span>#{sum.tagLine}</span></h1>
          {games && games.length > 0 && (
            <div className="pquick">
              <div><b className={wins / games.length >= 0.5 ? "good" : "bad"}>{Math.round((wins / games.length) * 100)}%</b><small>Winrate récent</small></div>
              <div><b>{(games.reduce((a, g) => a + kda(g.participants[0].stats), 0) / games.length).toFixed(2)}</b><small>KDA moyen</small></div>
              <div><b>{games.length}</b><small>Parties analysées</small></div>
              {champs[0] && <div><b>{d.champs[champs[0][0]]?.name}</b><small>Champion favori</small></div>}
            </div>
          )}
        </div>
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
                  {q?.tier && q.tier !== "NONE" && q.tier !== "UNRANKED" ? <img className="emblem" src={img.emblem(q.tier)} /> : <div className="emblem none" />}
                  <div>
                    <span>{l}</span>
                    <b className={q?.tier ? `t-${q.tier.toLowerCase()}` : ""}>{rankLabel(q)}</b>
                    {q && q.wins + q.losses > 0 && <small>{q.wins}V - {q.losses}D · {Math.round((q.wins / (q.wins + q.losses)) * 100)}%</small>}
                  </div>
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
          {!games ? <Skeleton rows={8} /> : sub === "apercu" ? (
            games.map((g) => <MatchRow key={g.gameId} g={g} puuid={sum.puuid} d={d} queues={queues} onOpen={onOpen} onMatch={() => go({ page: "match", gameId: g.gameId, puuid: sum.puuid })} />)
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
