import { useEffect, useState } from "react";
import type { Go } from "./main";
import Skeleton from "./Skeleton";
import { ago, mmss } from "./lcu";
import {
  queueLabel, valo, vdata, VD, vHistory, vMatch, VMatch, vName, VName, vNames, VPhase, VPlayer, vRank, VRank, vScore, vStats, vTeamWon,
} from "./valoApi";

const pct = (x: number) => `${Math.round(x * 100)}%`;

function useNames(players: VPlayer[] | undefined) {
  const [names, setNames] = useState<Record<string, VName>>({});
  const key = players?.map((p) => p.subject).join() ?? "";
  useEffect(() => {
    let live = true;
    players?.filter((p) => !p.gameName).forEach((p) => vName(p.subject).then((n) => live && n && setNames((o) => ({ ...o, [p.subject]: n }))));
    return () => {
      live = false;
    };
  }, [key]);
  return (p: VPlayer) => ({ name: p.gameName || names[p.subject]?.GameName || "", tag: p.tagLine || names[p.subject]?.TagLine || "" });
}

function Rank({ d, tier, rr, label, sub }: { d: VD; tier: number; rr?: number; label: string; sub?: string }) {
  const t = d.tiers[tier];
  return (
    <div className="rank">
      {tier > 2 && t ? <img className="emblem" src={t.icon} /> : <div className="emblem none" />}
      <div>
        <span>{label}</span>
        <b style={tier > 2 && t ? { color: t.color } : undefined}>{tier > 2 && t ? `${t.name}${rr !== undefined ? ` · ${rr} RR` : ""}` : "Non classé"}</b>
        {sub && <small>{sub}</small>}
      </div>
    </div>
  );
}

function VRow({ id, puuid, d, go }: { id: string; puuid: string; d: VD; go: Go }) {
  const [m, setM] = useState<VMatch | null>(null);
  const [err, setErr] = useState(false);
  useEffect(() => {
    vMatch(id).then(setM, () => setErr(true));
  }, [id]);
  const nameOf = useNames(m?.players);
  if (err) return null;
  if (!m) return <div className="skel"><div style={{ height: 76 }} /></div>;
  const p = m.players.find((x) => x.subject === puuid);
  if (!p?.stats) return null;
  const st = vStats(m);
  const s = st[puuid];
  const dm = m.matchInfo.queueID === "deathmatch";
  const win = dm ? [...m.players].sort((a, b) => (b.stats?.kills ?? 0) - (a.stats?.kills ?? 0))[0]?.subject === puuid : vTeamWon(m, p.teamId);
  const draw = !dm && m.teams?.every((t) => !t.won);
  const ranked = [...m.players].sort((a, b) => st[b.subject].acs - st[a.subject].acs);
  const place = ranked.findIndex((x) => x.subject === puuid) + 1;
  const teamBest = ranked.find((x) => x.teamId === p.teamId)?.subject === puuid;
  const map = d.maps[m.matchInfo.mapId];
  const agent = d.agents[p.characterId.toLowerCase()];
  const open = () => go({ page: "v-match", id, puuid });
  const res = draw ? "remake" : win ? "win" : "loss";
  const teams = dm ? [m.players] : [m.players.filter((x) => x.teamId === p.teamId), m.players.filter((x) => x.teamId !== p.teamId)];
  return (
    <div className={`vmatch ${res}`} onClick={open} role="button" tabIndex={0} onKeyDown={(e) => e.key === "Enter" && open()}>
      <div className="vface">
        {agent ? <img src={agent.icon} alt={agent.name} /> : <span />}
        {p.competitiveTier > 2 && d.tiers[p.competitiveTier] && <img className="vtier" src={d.tiers[p.competitiveTier].icon} title={d.tiers[p.competitiveTier].name} alt="" />}
      </div>
      <div className="vbody">
        <div className="vhead">
          <b className="vres">{draw ? "Égalité" : win ? "Victoire" : "Défaite"}</b>
          <span>{queueLabel(m.matchInfo.queueID)}</span>
          <span>{ago(m.matchInfo.gameStartMillis)}</span>
          <span>{mmss(m.matchInfo.gameLengthMillis / 1000)}</span>
          {place === 1 ? <em className="vmvp">MVP</em> : teamBest && <em className="vmvp">MVP équipe</em>}
        </div>
        <div className="vstats">
          <div className="vmapstat" style={map ? { backgroundImage: `url(${map.icon})` } : undefined}>
            <b>{vScore(m, p.teamId) || `${place}${place === 1 ? "er" : "e"}`}</b>
            <small>{map?.name ?? "Carte"}</small>
          </div>
          <div><b>{p.stats.kills} / <span className="bad">{p.stats.deaths}</span> / {p.stats.assists}</b><small><span className={s.kd >= 1 ? "good" : ""}>{s.kd.toFixed(2)}</span> K/D</small></div>
          <div><b>{(p.stats.kills / Math.max(1, p.stats.roundsPlayed)).toFixed(2)}</b><small>KPR</small></div>
          <div><b>{Math.round(s.adr)}</b><small>ADR</small></div>
          <div><b>{Math.round(s.acs)}</b><small>ACS</small></div>
          <div><b>{pct(s.hs)}</b><small>HS%</small></div>
        </div>
      </div>
      <div className={`vplayers ${dm ? "dm" : ""}`}>
        {teams.map((t, i) => (
          <div key={i}>
            {t.slice(0, dm ? 14 : 5).map((x) => (
              <button
                key={x.subject}
                className={x.subject === puuid ? "me" : ""}
                onClick={(e) => (e.stopPropagation(), go({ page: "v-profil", puuid: x.subject }))}
                title={nameOf(x).name ? `${nameOf(x).name}#${nameOf(x).tag}` : undefined}
              >
                <img src={d.agents[x.characterId.toLowerCase()]?.icon} alt="" />
                <span>{nameOf(x).name || "…"}</span>
              </button>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export function ValoProfile({ puuid, self, go }: { puuid?: string; self: string; go: Go }) {
  const who = puuid ?? self;
  const [d, setD] = useState<VD | null>(null);
  const [name, setName] = useState<VName | null>(null);
  const [rank, setRank] = useState<VRank | null>(null);
  const [ids, setIds] = useState<string[] | null>(null);
  const [matches, setMatches] = useState<VMatch[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    setIds(null);
    setMatches([]);
    setErr("");
    vdata().then(setD, (e) => setErr(String(e)));
    vNames([who]).then((n) => setName(n[0] ?? null), () => {});
    vRank(who).then(setRank);
    vHistory(who, 20).then((h) => {
      const list = h.map((x) => x.MatchID);
      setIds(list);
      list.forEach((id) => vMatch(id).then((m) => setMatches((old) => (old.some((x) => x.matchInfo.matchId === id) ? old : [...old, m])), () => {}));
    }, (e) => setErr(String(e)));
  }, [who]);

  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!d) return <div className="profile"><div className="skel"><div style={{ height: 150 }} /></div><Skeleton rows={6} /></div>;

  const mine = matches.map((m) => ({ m, p: m.players.find((x) => x.subject === who) })).filter((x) => x.p?.stats) as { m: VMatch; p: VPlayer }[];
  const won = (m: VMatch, p: VPlayer) => vTeamWon(m, p.teamId);
  const wins = mine.filter((x) => won(x.m, x.p)).length;
  const agg = mine.reduce(
    (a, { m, p }) => {
      const s = vStats(m)[who];
      return { k: a.k + p.stats!.kills, d: a.d + p.stats!.deaths, acs: a.acs + s.acs, hs: a.hs + s.hs, adr: a.adr + s.adr };
    },
    { k: 0, d: 0, acs: 0, hs: 0, adr: 0 },
  );
  const n = Math.max(1, mine.length);
  const pool = new Map<string, { g: number; w: number; k: number; d: number; acs: number }>();
  for (const { m, p } of mine) {
    const c = pool.get(p.characterId.toLowerCase()) ?? { g: 0, w: 0, k: 0, d: 0, acs: 0 };
    pool.set(p.characterId.toLowerCase(), { g: c.g + 1, w: c.w + (won(m, p) ? 1 : 0), k: c.k + p.stats!.kills, d: c.d + p.stats!.deaths, acs: c.acs + vStats(m)[who].acs });
  }
  const agents = [...pool.entries()].sort((a, b) => b[1].g - a[1].g);
  const fav = agents[0] && d.agents[agents[0][0]];
  const sorted = ids ?? [];

  return (
    <div className="profile">
      <div className="phero vhero" style={fav ? { backgroundImage: `linear-gradient(90deg, var(--bg) 30%, #0b0f17cc 55%, #0b0f1700), url(${fav.portrait})` } : undefined}>
        <div className="pid">
          <h1>{name?.GameName ?? "…"} <span>#{name?.TagLine}</span></h1>
          {mine.length > 0 && (
            <div className="pquick">
              <div><b className={wins / n >= 0.5 ? "good" : "bad"}>{pct(wins / n)}</b><small>Winrate récent</small></div>
              <div><b>{(agg.k / Math.max(1, agg.d)).toFixed(2)}</b><small>K/D</small></div>
              <div><b>{Math.round(agg.acs / n)}</b><small>ACS moyen</small></div>
              <div><b>{pct(agg.hs / n)}</b><small>Headshots</small></div>
              {fav && <div><b>{fav.name}</b><small>Agent favori</small></div>}
            </div>
          )}
        </div>
      </div>
      <div className="cols">
        <aside>
          <section className="card">
            {rank ? (
              <>
                <Rank d={d} tier={rank.tier} rr={rank.tier > 2 ? rank.rr : undefined} label="Compétition · acte en cours" sub={rank.games ? `${rank.wins}V sur ${rank.games} parties · ${pct(rank.wins / rank.games)}` : undefined} />
                <Rank d={d} tier={rank.peak} label="Meilleur rang" />
              </>
            ) : <Skeleton rows={2} />}
          </section>
          <section className="card">
            <h3>{mine.length} dernières parties</h3>
            <div className="wr"><i style={{ width: `${(wins / n) * 100}%` }} /></div>
            <small className="muted">{wins}V - {mine.length - wins}D · {Math.round(agg.adr / n)} ADR moyen</small>
            {agents.slice(0, 6).map(([a, x]) => (
              <div key={a} className="perf">
                <img src={d.agents[a]?.icon} />
                <div><b>{d.agents[a]?.name ?? "?"}</b><small>{(x.k / Math.max(1, x.d)).toFixed(2)} K/D · {Math.round(x.acs / x.g)} ACS</small></div>
                <div className="r"><b className={x.w / x.g >= 0.5 ? "good" : "bad"}>{pct(x.w / x.g)}</b><small>{x.g} parties</small></div>
              </div>
            ))}
          </section>
        </aside>
        <div className="list">
          {!ids ? <Skeleton rows={8} /> : sorted.length === 0 ? <div className="card muted">Aucune partie récente.</div> : sorted.map((id) => <VRow key={id} id={id} puuid={who} d={d} go={go} />)}
        </div>
      </div>
    </div>
  );
}

const RESULT: Record<string, string> = { Eliminated: "É", "Bomb detonated": "S", "Bomb defused": "D", "Round timer expired": "T", Surrendered: "F" };

export function ValoMatch({ id, puuid, go }: { id: string; puuid: string; go: Go }) {
  const [m, setM] = useState<VMatch | null>(null);
  const [d, setD] = useState<VD | null>(null);
  const [err, setErr] = useState("");
  useEffect(() => {
    Promise.all([vMatch(id), vdata()]).then(([m, d]) => (setM(m), setD(d)), (e) => setErr(String(e)));
  }, [id]);
  const nameOf = useNames(m?.players);
  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!m || !d) return <Skeleton rows={10} />;
  const st = vStats(m);
  const me = m.players.find((x) => x.subject === puuid);
  const myTeam = me?.teamId ?? "Blue";
  const teams = [...new Set([myTeam, ...m.players.map((x) => x.teamId)])];
  const map = d.maps[m.matchInfo.mapId];
  const maxAcs = Math.max(1, ...Object.values(st).map((s) => s.acs));
  return (
    <div className="vmatchpage">
      <div className="phero vmhead" style={map ? { backgroundImage: `linear-gradient(90deg, var(--bg) 25%, #0b0f17aa 60%, #0b0f1733), url(${map.splash})` } : undefined}>
        <div className="pid">
          <h1>{map?.name ?? "Carte"} <span>{queueLabel(m.matchInfo.queueID)}</span></h1>
          <div className="pquick">
            {me && vScore(m, myTeam) && <div><b className={vTeamWon(m, myTeam) ? "good" : "bad"}>{vScore(m, myTeam)}</b><small>{vTeamWon(m, myTeam) ? "Victoire" : "Défaite"}</small></div>}
            <div><b>{mmss(m.matchInfo.gameLengthMillis / 1000)}</b><small>Durée</small></div>
            <div><b>{new Date(m.matchInfo.gameStartMillis).toLocaleDateString("fr", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })}</b><small>Date</small></div>
          </div>
        </div>
      </div>
      {m.roundResults && m.roundResults.length > 0 && (
        <section className="card vrounds">
          <h3>Manches</h3>
          <div>
            {m.roundResults.map((r) => (
              <span key={r.roundNum} className={r.winningTeam === myTeam ? "rw" : "rl"} title={`Manche ${r.roundNum + 1} · ${r.roundResult}`}>
                {RESULT[r.roundResult] ?? "·"}
                {r.roundNum === 11 && <i />}
              </span>
            ))}
          </div>
        </section>
      )}
      <section className="card">
        <table className="board vboard">
          <colgroup>
            <col style={{ width: 52 }} /><col /><col style={{ width: 54 }} /><col style={{ width: 130 }} /><col style={{ width: 120 }} />
            <col style={{ width: 60 }} /><col style={{ width: 70 }} /><col style={{ width: 70 }} /><col style={{ width: 54 }} /><col style={{ width: 54 }} />
          </colgroup>
          {teams.map((t) => {
            const list = m.players.filter((x) => x.teamId === t).sort((a, b) => st[b.subject].acs - st[a.subject].acs);
            const win = vTeamWon(m, t);
            return (
              <tbody key={t}>
                <tr className="bhead">
                  <th colSpan={2}>{teams.length > 1 && <><b className={win ? "good" : "bad"}>{win ? "Victoire" : "Défaite"}</b> · {t === myTeam ? "Ton équipe" : "Adversaires"}</>}</th>
                  <th>Rang</th><th>ACS</th><th>K / D / A</th><th>+/-</th><th>ADR</th><th>HS</th><th>FK</th><th>FD</th>
                </tr>
                {list.map((x) => {
                  const s = st[x.subject];
                  const diff = (x.stats?.kills ?? 0) - (x.stats?.deaths ?? 0);
                  const tier = d.tiers[x.competitiveTier];
                  return (
                    <tr key={x.subject} className={x.subject === puuid ? "me" : ""}>
                      <td><img className="bc" src={d.agents[x.characterId.toLowerCase()]?.icon} title={d.agents[x.characterId.toLowerCase()]?.name} /></td>
                      <td className="name" onClick={() => go({ page: "v-profil", puuid: x.subject })}>{nameOf(x).name || "…"}{nameOf(x).tag && <small>#{nameOf(x).tag}</small>}</td>
                      <td className="c">{x.competitiveTier > 2 && tier ? <img className="vtier" src={tier.icon} title={tier.name} /> : "–"}</td>
                      <td className="c"><b>{Math.round(s.acs)}</b><i className="dbar2"><i style={{ width: `${(s.acs / maxAcs) * 100}%` }} /></i></td>
                      <td className="c"><b>{x.stats?.kills} / <span className="bad">{x.stats?.deaths}</span> / {x.stats?.assists}</b></td>
                      <td className={`c ${diff > 0 ? "good" : diff < 0 ? "bad" : ""}`}>{diff > 0 ? "+" : ""}{diff}</td>
                      <td className="c">{Math.round(s.adr)}</td>
                      <td className="c">{pct(s.hs)}</td>
                      <td className="c">{s.fk}</td>
                      <td className="c">{s.fd}</td>
                    </tr>
                  );
                })}
              </tbody>
            );
          })}
        </table>
      </section>
    </div>
  );
}

type LiveP = { puuid: string; agent: string; team: string; incognito: boolean; level: number; locked?: boolean };

function LiveCard({ p, d, name, self, go }: { p: LiveP; d: VD; name?: VName; self: string; go: Go }) {
  const [r, setR] = useState<VRank | null>(null);
  useEffect(() => {
    vRank(p.puuid).then(setR);
  }, [p.puuid]);
  const a = d.agents[p.agent.toLowerCase()];
  const hidden = p.incognito && p.puuid !== self;
  return (
    <div className={`vlive ${p.puuid === self ? "me" : ""}`} onClick={() => !hidden && go({ page: "v-profil", puuid: p.puuid })} role="button" tabIndex={0}>
      {a ? <img className={`vlag ${p.locked === false ? "hover" : ""}`} src={a.icon} title={a.name} /> : <div className="vlag none" />}
      <div className="vlid">
        <b>{hidden ? "Joueur masqué" : name ? name.GameName : "…"}{!hidden && name && <small>#{name.TagLine}</small>}</b>
        <small>{a?.name ?? "Pas encore choisi"}{p.level > 0 ? ` · niv. ${p.level}` : ""}</small>
      </div>
      {r ? (
        <div className="vlr">
          {r.tier > 2 && d.tiers[r.tier] ? <img src={d.tiers[r.tier].icon} title={d.tiers[r.tier].name} /> : <span className="emblem none" />}
          <div>
            <b style={{ color: d.tiers[r.tier]?.color }}>{r.tier > 2 ? `${d.tiers[r.tier]?.name} · ${r.rr} RR` : "Non classé"}</b>
            <small>{r.games ? `${pct(r.wins / r.games)} sur ${r.games} parties` : "Aucune partie classée"}{r.peak > 2 ? ` · max ${d.tiers[r.peak]?.name}` : ""}</small>
          </div>
        </div>
      ) : <div className="vlr muted">…</div>}
    </div>
  );
}

export function ValoLive({ phase, self, go }: { phase: VPhase; self: string; go: Go }) {
  const [d, setD] = useState<VD | null>(null);
  const [game, setGame] = useState<{ map: string; queue: string; players: LiveP[] } | null>(null);
  const [names, setNames] = useState<Record<string, VName>>({});

  useEffect(() => {
    vdata().then(setD, () => {});
  }, []);

  useEffect(() => {
    if (phase !== "PREGAME" && phase !== "INGAME") return setGame(null);
    let stop = false;
    const load = async () => {
      const pre = phase === "PREGAME";
      const { MatchID } = await valo("glz", pre ? `/pregame/v1/players/${self}` : `/core-game/v1/players/${self}`);
      const m = await valo("glz", pre ? `/pregame/v1/matches/${MatchID}` : `/core-game/v1/matches/${MatchID}`);
      const raw: any[] = pre ? m.AllyTeam?.Players ?? [] : m.Players ?? [];
      const players: LiveP[] = raw.map((x) => ({
        puuid: x.Subject, agent: x.CharacterID ?? "", team: x.TeamID ?? m.AllyTeam?.TeamID ?? "", incognito: !!x.PlayerIdentity?.Incognito,
        level: x.PlayerIdentity?.HideAccountLevel ? 0 : x.PlayerIdentity?.AccountLevel ?? 0, locked: pre ? x.CharacterSelectionState === "locked" : undefined,
      }));
      if (stop) return;
      setGame({ map: m.MapID, queue: m.QueueID ?? m.MatchmakingData?.QueueID ?? "", players });
      const missing = players.map((p) => p.puuid).filter((p) => !names[p]);
      if (missing.length) vNames(missing).then((list) => !stop && setNames((o) => ({ ...o, ...Object.fromEntries(list.map((n) => [n.Subject, n])) })), () => {});
    };
    load().catch(() => {});
    const id = setInterval(() => load().catch(() => {}), phase === "PREGAME" ? 3000 : 15000);
    return () => {
      stop = true;
      clearInterval(id);
    };
  }, [phase, self]);

  if (!game || !d) {
    return (
      <div className="liveempty">
        <span className="pulse" />
        <h2>{phase === "PREGAME" || phase === "INGAME" ? "Chargement de la partie…" : "Aucune partie en cours"}</h2>
        <p className="muted">Lance une file dans VALORANT : Ethra affiche les rangs de ton équipe dès la sélection des agents, puis ceux des adversaires en partie.</p>
        <div className="lbtns"><button className="primary" onClick={() => go({ page: "v-profil" })}>Voir mon profil</button></div>
      </div>
    );
  }
  const me = game.players.find((p) => p.puuid === self);
  const teams = [...new Set([me?.team ?? "", ...game.players.map((p) => p.team)])].filter((t, i, a) => t || a.length === 1);
  const map = d.maps[game.map];
  return (
    <div className="live">
      <div className="phero vmhead" style={map ? { backgroundImage: `linear-gradient(90deg, var(--bg) 25%, #0b0f17aa 60%, #0b0f1733), url(${map.splash})` } : undefined}>
        <div className="pid">
          <h1>{map?.name ?? "Carte"} <span>{queueLabel(game.queue)}</span></h1>
          <small>{phase === "PREGAME" ? "Sélection des agents : les adversaires apparaîtront au lancement de la partie." : "Partie en cours"}</small>
        </div>
      </div>
      <div className="vteams">
        {teams.map((t) => (
          <section key={t} className="card">
            <h3>{t === me?.team ? "Ton équipe" : "Adversaires"}</h3>
            {game.players.filter((p) => p.team === t).map((p) => <LiveCard key={p.puuid} p={p} d={d} name={names[p.puuid]} self={self} go={go} />)}
          </section>
        ))}
      </div>
    </div>
  );
}
