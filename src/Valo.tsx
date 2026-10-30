import { useEffect, useRef, useState } from "react";
import type { Go } from "./main";
import Skeleton from "./Skeleton";
import { call as opggCall } from "./opgg";
import { copyText } from "./util";
import { ago, mmss } from "./lcu";
import {
  queueLabel, valo, vdata, VD, vHistory, vMatch, VMatch, vName, VName, vNames, VPhase, VPlayer, vRank, VRank, VRR, vRRUpdates, vScore, vStats, vTeamWon, mkLabel, vWeaponKills, vEconomy, vParties, PARTY_LABEL, seenPlayers, Seen,
} from "./valoApi";

const FILTERS: [string, string][] = [["", "Toutes"], ["competitive", "Compétition"], ["unrated", "Non classé"], ["swiftplay", "Vélocité"], ["deathmatch", "Combat à mort"], ["premier", "Premier"]];
const PAGE = 20;

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
        <b style={tier > 2 && t ? { color: t.color } : undefined}>{tier > 2 && t ? `${t.name}${rr !== undefined ? `, ${rr} RR` : ""}` : "Non classé"}</b>
        {sub && <small>{sub}</small>}
      </div>
    </div>
  );
}

function VRow({ id, puuid, d, go, rr }: { id: string; puuid: string; d: VD; go: Go; rr?: VRR }) {
  const [m, setM] = useState<VMatch | null>(null);
  const [err, setErr] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    setErr(false);
    vMatch(id).then(setM, () => setErr(true));
  }, [id, attempt]);
  const nameOf = useNames(m?.players);
  if (err)
    return (
      <div className="vmatch vfail">
        <span>Impossible de charger cette partie pour l'instant.</span>
        <button className="ghostbtn" onClick={() => setAttempt((a) => a + 1)}>Réessayer</button>
      </div>
    );
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
          {mkLabel(s.mk).slice(0, 1).map((l) => <em key={l} className="vmk">{l}</em>)}
          {rr && !rr.placement && <em className={`vrr ${rr.earned >= 0 ? "up" : "down"}`}>{rr.earned > 0 ? "+" : ""}{rr.earned} RR</em>}
          {rr && rr.tierAfter > rr.tierBefore && <em className="vrr up">Promu {d.tiers[rr.tierAfter]?.name}</em>}
          {rr && rr.tierAfter < rr.tierBefore && <em className="vrr down">Rétrogradé</em>}
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
  const [rr, setRR] = useState<Record<string, VRR>>({});
  const [queue, setQueue] = useState("");
  const [ids, setIds] = useState<string[] | null>(null);
  const [more, setMore] = useState<"idle" | "loading" | "end">("idle");
  const [byId, setById] = useState<Record<string, VMatch>>({});
  const [tab, setTab] = useState<"parties" | "agents" | "cartes" | "armes">("parties");
  const [err, setErr] = useState("");

  useEffect(() => {
    setErr("");
    vdata().then(setD, (e) => setErr(String(e)));
    vNames([who]).then((n) => setName(n[0] ?? null), () => {});
    vRank(who).then(setRank);
    vRRUpdates(who).then(setRR);
  }, [who]);

  const track = (list: string[]) =>
    list.forEach((id) => vMatch(id).then((m) => setById((o) => (o[id] ? o : { ...o, [id]: m })), () => {}));

  useEffect(() => {
    let live = true;
    setIds(null);
    setMore("idle");
    vHistory(who, PAGE, queue).then((h) => {
      if (!live) return;
      const list = h.map((x) => x.MatchID);
      setIds(list);
      if (list.length < PAGE) setMore("end");
      track(list);
    }, (e) => live && setErr(String(e)));
    return () => {
      live = false;
    };
  }, [who, queue]);

  const loadMore = () => {
    if (!ids || more !== "idle") return;
    setMore("loading");
    vHistory(who, PAGE, queue, ids.length).then((h) => {
      const list = h.map((x) => x.MatchID).filter((id) => !ids.includes(id));
      setIds([...ids, ...list]);
      setMore(list.length < PAGE ? "end" : "idle");
      track(list);
    }, () => setMore("idle"));
  };

  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!d) return <div className="profile"><div className="skel"><div style={{ height: 150 }} /></div><Skeleton rows={6} /></div>;

  const mine = (ids ?? []).map((id) => byId[id]).filter(Boolean).map((m) => ({ m, p: m.players.find((x) => x.subject === who)! })).filter((x) => x.p?.stats);
  const won = (m: VMatch, p: VPlayer) => vTeamWon(m, p.teamId);
  const wins = mine.filter((x) => won(x.m, x.p)).length;
  const sum = (list: typeof mine) =>
    list.reduce(
      (a, { m, p }) => {
        const s = vStats(m)[who];
        return {
          g: a.g + 1, w: a.w + (won(m, p) ? 1 : 0), k: a.k + p.stats!.kills, d: a.d + p.stats!.deaths, acs: a.acs + s.acs, hs: a.hs + s.hs, adr: a.adr + s.adr,
          r: a.r + p.stats!.roundsPlayed, rw: a.rw + (m.teams?.find((t) => t.teamId === p.teamId)?.roundsWon ?? 0),
        };
      },
      { g: 0, w: 0, k: 0, d: 0, acs: 0, hs: 0, adr: 0, r: 0, rw: 0 },
    );
  const agg = sum(mine);
  const n = Math.max(1, agg.g);
  const group = (key: (m: VMatch, p: VPlayer) => string) => {
    const g = new Map<string, typeof mine>();
    for (const x of mine) g.set(key(x.m, x.p), [...(g.get(key(x.m, x.p)) ?? []), x]);
    return [...g.entries()].map(([k, list]) => [k, sum(list)] as const).sort((a, b) => b[1].g - a[1].g);
  };
  const agents = group((_, p) => p.characterId.toLowerCase());
  const maps = group((m) => m.matchInfo.mapId);
  const fav = agents[0] && d.agents[agents[0][0]];
  const midnight = new Date().setHours(0, 0, 0, 0);
  const today = mine.filter((x) => x.m.matchInfo.gameStartMillis >= midnight);
  const todayAgg = sum(today);
  const todayRR = today.reduce((a, x) => a + (rr[x.m.matchInfo.matchId]?.earned ?? 0), 0);
  const hasRR = today.some((x) => rr[x.m.matchInfo.matchId]);
  const kd = (x: { k: number; d: number }) => (x.k / Math.max(1, x.d)).toFixed(2);

  return (
    <div className="profile">
      <div className="phero vhero" style={fav ? { backgroundImage: `linear-gradient(90deg, var(--bg) 30%, color-mix(in srgb, var(--bg) 80%, transparent) 55%, transparent), url(${fav.portrait})` } : undefined}>
        <div className="pid">
          <h1><button className="copyid" disabled={!name} onClick={() => name && copyText(`${name.GameName}#${name.TagLine}`, "Riot ID")} title="Copier le Riot ID">{name?.GameName ?? "…"} <span>#{name?.TagLine}</span></button></h1>
          {mine.length > 0 && (
            <div className="pquick">
              <div><b className={wins / n >= 0.5 ? "good" : "bad"}>{pct(wins / n)}</b><small>Winrate</small></div>
              <div><b>{kd(agg)}</b><small>K/D</small></div>
              <div><b>{Math.round(agg.acs / n)}</b><small>ACS moyen</small></div>
              <div><b>{Math.round(agg.adr / n)}</b><small>ADR moyen</small></div>
              <div><b>{pct(agg.hs / n)}</b><small>Headshots</small></div>
              {fav && <div><b>{fav.name}</b><small>Agent favori</small></div>}
            </div>
          )}
        </div>
        {rank && rank.tier > 2 && d.tiers[rank.tier] && (
          <div className="vherorank">
            <img src={d.tiers[rank.tier].icon} alt="" />
            <b style={{ color: d.tiers[rank.tier].color }}>{d.tiers[rank.tier].name}</b>
            <small>{rank.rr} RR</small>
          </div>
        )}
      </div>
      <div className="subtabs">
        {([["parties", "Parties"], ["agents", "Agents"], ["cartes", "Cartes"], ["armes", "Armes"]] as const).map(([k, l]) => (
          <button key={k} className={tab === k ? "on" : ""} onClick={() => setTab(k)}>{l}</button>
        ))}
        <div className="seg vfilter">
          {FILTERS.map(([k, l]) => <button key={k} className={queue === k ? "on" : ""} onClick={() => setQueue(k)}>{l}</button>)}
        </div>
      </div>
      <div className="cols">
        <aside>
          {who === self && (
            <section className="card session">
              <h3>Session du jour</h3>
              {today.length === 0 ? (
                <p className="muted">Aucune partie aujourd'hui{queue ? " dans ce mode" : ""}.</p>
              ) : (
                <>
                  <div className="sess">
                    <div><b><span className="good">{todayAgg.w}V</span> <span className="bad">{todayAgg.g - todayAgg.w}D</span></b><small>{pct(todayAgg.w / todayAgg.g)} de victoires</small></div>
                    <div><b>{kd(todayAgg)}</b><small>K/D</small></div>
                    {hasRR && <div><b className={todayRR >= 0 ? "good" : "bad"}>{todayRR >= 0 ? "+" : ""}{todayRR} RR</b><small>Compétition</small></div>}
                  </div>
                  <div className="streak">{today.slice(0, 14).reverse().map((x) => <i key={x.m.matchInfo.matchId} className={won(x.m, x.p) ? "sw" : "sl"} />)}</div>
                </>
              )}
            </section>
          )}
          <section className="card">
            {rank ? (
              <>
                <Rank d={d} tier={rank.tier} rr={rank.tier > 2 ? rank.rr : undefined} label="Compétition, acte en cours" sub={rank.games ? `${rank.wins}V sur ${rank.games} parties (${pct(rank.wins / rank.games)})` : undefined} />
                <Rank d={d} tier={rank.peak} label="Meilleur rang" />
              </>
            ) : <Skeleton rows={2} />}
          </section>
          <section className="card">
            <h3>{agg.g} parties analysées</h3>
            <div className="wr"><i style={{ width: `${(wins / n) * 100}%` }} /></div>
            <small className="muted">{wins}V, {agg.g - wins}D, {agg.r ? pct(agg.rw / agg.r) : "0%"} de manches gagnées</small>
            {agents.slice(0, 5).map(([a, x]) => (
              <div key={a} className="perf">
                <img src={d.agents[a]?.icon} alt="" />
                <div><b>{d.agents[a]?.name ?? "?"}</b><small>{kd(x)} K/D, {Math.round(x.acs / x.g)} ACS</small></div>
                <div className="r"><b className={x.w / x.g >= 0.5 ? "good" : "bad"}>{pct(x.w / x.g)}</b><small>{x.g} {x.g > 1 ? "parties" : "partie"}</small></div>
              </div>
            ))}
          </section>
        </aside>
        <div className="list">
          {!ids ? (
            <Skeleton rows={8} />
          ) : ids.length === 0 ? (
            <div className="card muted">Aucune partie récente{queue ? " dans ce mode" : ""}.</div>
          ) : tab === "parties" ? (
            <>
              {ids.map((id) => <VRow key={id} id={id} puuid={who} d={d} go={go} rr={rr[id]} />)}
              {more !== "end" && (
                <button className="ghostbtn vmore" onClick={loadMore} disabled={more === "loading"}>{more === "loading" ? "Chargement…" : `Charger ${PAGE} parties de plus`}</button>
              )}
            </>
          ) : tab === "armes" ? (
            (() => {
              const kills: Record<string, number> = {};
              for (const x of mine)
                for (const [w, c] of Object.entries(vWeaponKills(x.m, who))) {
                  const key = w === "ability" || w === "melee" || d.weapons[w] ? w : `agent:${x.p.characterId.toLowerCase()}`;
                  kills[key] = (kills[key] ?? 0) + c;
                }
              const total = Math.max(1, Object.values(kills).reduce((a, c) => a + c, 0));
              const rows = Object.entries(kills).sort((a, b) => b[1] - a[1]);
              const label = (w: string) =>
                w === "ability" ? "Compétences" : w === "melee" ? "Couteau" : w.startsWith("agent:") ? `Arme de ${d.agents[w.slice(6)]?.name ?? "l'agent"}` : d.weapons[w]?.name ?? "Arme";
              const icon = (w: string) => (w.startsWith("agent:") ? d.agents[w.slice(6)]?.icon : d.weapons[w]?.icon);
              return (
                <div className="card vweapons">
                  {rows.map(([w, c]) => (
                    <div key={w} className="vw">
                      <div className={`vwicon ${w.startsWith("agent:") ? "agent" : ""}`}>{icon(w) ? <img src={icon(w)} alt="" /> : <span>{label(w)[0]}</span>}</div>
                      <b>{label(w)}</b>
                      <div className="vwbar"><i style={{ width: `${(c / rows[0][1]) * 100}%` }} /></div>
                      <span><b>{c}</b> {c > 1 ? "éliminations" : "élimination"}</span>
                      <small>{pct(c / total)}</small>
                    </div>
                  ))}
                  {rows.length === 0 && <p className="muted">Aucune élimination sur ces parties.</p>}
                </div>
              );
            })()
          ) : (
            <table className="pool vpool">
              <thead>
                <tr><th>{tab === "agents" ? "Agent" : "Carte"}</th><th>Parties</th><th>Winrate</th><th>K/D</th><th>ACS</th><th>ADR</th><th>HS</th><th>Manches</th></tr>
              </thead>
              <tbody>
                {(tab === "agents" ? agents : maps).map(([k, x]) => (
                  <tr key={k}>
                    <td>
                      {tab === "agents" ? <img src={d.agents[k]?.icon} alt="" /> : <img className="vmapimg" src={d.maps[k]?.icon} alt="" />}
                      {tab === "agents" ? d.agents[k]?.name ?? "?" : d.maps[k]?.name ?? "?"}
                    </td>
                    <td>{x.g}</td>
                    <td className={x.w / x.g >= 0.5 ? "good" : "bad"}>{pct(x.w / x.g)}</td>
                    <td>{kd(x)}</td>
                    <td>{Math.round(x.acs / x.g)}</td>
                    <td>{Math.round(x.adr / x.g)}</td>
                    <td>{pct(x.hs / x.g)}</td>
                    <td>{x.r ? pct(x.rw / x.r) : "–"}</td>
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

export function ValoSearch({ go }: { go: Go }) {
  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  const [sel, setSel] = useState(0);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        ref.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const t = q.trim().toLowerCase();
  const all = open ? seenPlayers() : [];
  const rank = (p: Seen) => (p.name.toLowerCase().startsWith(t) ? 0 : p.name.toLowerCase().includes(t) ? 1 : 2);
  const opts = (t ? all.filter((p) => `${p.name}#${p.tag}`.toLowerCase().includes(t)).sort((a, b) => rank(a) - rank(b)) : all).slice(0, 8);
  const pick = (p?: Seen) => {
    if (!p) return;
    go({ page: "v-profil", puuid: p.puuid });
    setQ("");
    setOpen(false);
    ref.current?.blur();
  };

  return (
    <div className="search">
      <svg className="sicon" width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" strokeWidth="1.7" /><path d="m11 11 3.5 3.5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" /></svg>
      <input
        ref={ref}
        value={q}
        aria-label="Rechercher un joueur croisé"
        placeholder="Joueur croisé"
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        onChange={(e) => (setQ(e.target.value), setSel(0), setOpen(true))}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") (e.preventDefault(), setSel((s) => Math.min(opts.length - 1, s + 1)));
          if (e.key === "ArrowUp") (e.preventDefault(), setSel((s) => Math.max(0, s - 1)));
          if (e.key === "Enter") pick(opts[sel]);
          if (e.key === "Escape") ref.current?.blur();
        }}
      />
      <kbd>Ctrl K</kbd>
      {open && (
        <div className="sresults">
          {opts.map((p, i) => (
            <button key={p.puuid} className={i === sel ? "on" : ""} onMouseEnter={() => setSel(i)} onMouseDown={() => pick(p)}>
              <span className="ph">{p.name[0]?.toUpperCase()}</span>
              <b>{p.name}<span className="muted">#{p.tag}</span></b>
              <small>Croisé {ago(p.t)}</small>
            </button>
          ))}
          {opts.length === 0 && (
            <p className="sempty">{t ? "Aucun joueur croisé avec ce nom." : "Les joueurs de tes parties apparaîtront ici."} Riot ne permet pas de chercher un joueur que tu n'as jamais croisé.</p>
          )}
        </div>
      )}
    </div>
  );
}

type AgentRow = { id: string; games: number; wr: number; pick: number; kd: number; acs: number; hs: number; fkfd: number; tier: number };
type AgentKey = "wr" | "pick" | "kd" | "acs" | "hs" | "fkfd" | "games";

export function ValoAgents() {
  const [d, setD] = useState<VD | null>(null);
  const [rows, setRows] = useState<AgentRow[] | null>(null);
  const [patch, setPatch] = useState("");
  const [err, setErr] = useState("");
  const [role, setRole] = useState("");
  const [sort, setSort] = useState<{ k: AgentKey; desc: boolean }>({ k: "wr", desc: true });

  useEffect(() => {
    vdata().then(setD, (e) => setErr(String(e)));
    opggCall("valorant_list_agent_statistics", {}).then(
      (r: any) => {
        const list: any[] = r.data ?? [];
        const picks = Math.max(1, list.reduce((a, x) => a + x.gameCount, 0));
        const out: AgentRow[] = list.map((x) => ({
          id: String(x.characterId).toLowerCase(), games: x.gameCount, wr: x.wins / Math.max(1, x.gameCount - (x.draws ?? 0)), pick: x.gameCount / picks,
          kd: x.kills / Math.max(1, x.deaths), acs: x.acs, hs: x.headShots / Math.max(1, x.headShots + x.bodyShots + x.legShots), fkfd: x.firstKills / Math.max(1, x.firstDeaths), tier: 0,
        }));
        const byWr = [...out].sort((a, b) => b.wr - a.wr);
        byWr.forEach((x, i) => (x.tier = i < byWr.length * 0.15 ? 1 : i < byWr.length * 0.4 ? 2 : i < byWr.length * 0.75 ? 3 : 4));
        setRows(out);
        setPatch(r.meta?.gameVersion ? String(r.meta.gameVersion) : "");
      },
      (e) => setErr(String(e)),
    );
  }, []);

  if (err) return <div className="empty"><h2>OP.GG ne répond pas</h2><p>{err}</p></div>;
  if (!d || !rows) return <div className="tierlist"><div className="pagehead"><h1>Agents</h1></div><Skeleton rows={10} height={56} /></div>;

  const roles = [...new Set(rows.map((x) => d.agents[x.id]?.role).filter(Boolean))].sort();
  const list = rows
    .filter((x) => d.agents[x.id] && (!role || d.agents[x.id].role === role))
    .sort((a, b) => (sort.desc ? b[sort.k] - a[sort.k] : a[sort.k] - b[sort.k]));
  const th = (k: AgentKey, l: string) => (
    <th className={sort.k === k ? "sorted" : ""} onClick={() => setSort({ k, desc: sort.k === k ? !sort.desc : true })}>
      {l} {sort.k === k ? (sort.desc ? "▾" : "▴") : ""}
    </th>
  );
  const pc = (x: number) => `${(x * 100).toFixed(1)}%`;

  return (
    <div className="tierlist">
      <div className="pagehead">
        <h1>Agents <span className="muted">Compétition{patch ? `, patch ${patch}` : ""}</span></h1>
        <p className="muted">Statistiques mondiales fournies par OP.GG. Le tier est calculé sur le winrate : les 15 % meilleurs agents sont en S. La part des choix indique la proportion de joueurs qui prennent l'agent.</p>
      </div>
      <div className="filters">
        <div className="seg">
          <button className={!role ? "on" : ""} onClick={() => setRole("")}>Tous</button>
          {roles.map((r) => (
            <button key={r} className={`rolebtn ${role === r ? "on" : ""}`} onClick={() => setRole(r)}>
              <img src={Object.values(d.agents).find((a) => a.role === r)?.roleIcon} alt="" />{r}
            </button>
          ))}
        </div>
      </div>
      <table className="table vagents">
        <thead>
          <tr>
            <th>#</th>
            <th>Agent</th>
            <th>Tier</th>
            {th("wr", "Winrate")}
            {th("pick", "Part des choix")}
            {th("kd", "K/D")}
            {th("acs", "ACS")}
            {th("hs", "HS")}
            {th("fkfd", "FK/FD")}
            {th("games", "Parties")}
          </tr>
        </thead>
        <tbody>
          {list.map((x, i) => {
            const a = d.agents[x.id];
            return (
              <tr key={x.id}>
                <td className="muted">{i + 1}</td>
                <td><div className="cname"><img src={a.icon} alt="" />{a.name}<small className="muted">{a.role}</small></div></td>
                <td><span className={`tier t${x.tier}`}>{["", "S", "A", "B", "C"][x.tier]}</span></td>
                <td className={x.wr >= 0.5 ? "good" : "bad"}>{pc(x.wr)}</td>
                <td>{pc(x.pick)}</td>
                <td>{x.kd.toFixed(2)}</td>
                <td>{Math.round(x.acs)}</td>
                <td>{pc(x.hs)}</td>
                <td className={x.fkfd >= 1 ? "good" : "bad"}>{x.fkfd.toFixed(2)}</td>
                <td>{x.games.toLocaleString("fr")}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

const RESULT_FR: Record<string, string> = { Eliminated: "Élimination", "Bomb detonated": "Spike explosé", "Bomb defused": "Spike désamorcé", "Round timer expired": "Temps écoulé", Surrendered: "Abandon" };
const RESULT: Record<string, string> = { Eliminated: "É", "Bomb detonated": "S", "Bomb defused": "D", "Round timer expired": "T", Surrendered: "F" };

export function ValoMatch({ id, puuid, go }: { id: string; puuid: string; go: Go }) {
  const [m, setM] = useState<VMatch | null>(null);
  const [d, setD] = useState<VD | null>(null);
  const [err, setErr] = useState("");
  useEffect(() => {
    Promise.all([vMatch(id), vdata()]).then(([m, d]) => (setM(m), setD(d)), (e) => setErr(String(e)));
  }, [id]);
  const nameOf = useNames(m?.players);
  const [round, setRound] = useState<number | null>(null);
  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!m || !d) return <Skeleton rows={10} />;
  const st = vStats(m);
  const me = m.players.find((x) => x.subject === puuid);
  const myTeam = me?.teamId ?? "Blue";
  const teams = [...new Set([myTeam, ...m.players.map((x) => x.teamId)])];
  const map = d.maps[m.matchInfo.mapId];
  const maxAcs = Math.max(1, ...Object.values(st).map((s) => s.acs));
  const eco = vEconomy(m, myTeam);
  const ecoEnemy = vEconomy(m, teams.find((t) => t !== myTeam) ?? myTeam);
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
              <div key={r.roundNum} className={`vround ${r.roundNum === 11 ? "half" : ""}`}>
                <button
                  className={`${r.winningTeam === myTeam ? "rw" : "rl"} ${round === r.roundNum ? "sel" : ""}`}
                  title={`Manche ${r.roundNum + 1} : ${RESULT_FR[r.roundResult] ?? r.roundResult}`}
                  aria-pressed={round === r.roundNum}
                  onClick={() => setRound(round === r.roundNum ? null : r.roundNum)}
                >
                  {RESULT[r.roundResult] ?? "·"}
                </button>
                {eco[r.roundNum]?.value > 0 && (
                  <small className={`eco e${eco[r.roundNum].kind.replace(/[^A-Za-z]/g, "")}`} title={`Équipement de ton équipe : ${eco[r.roundNum].value.toLocaleString("fr")} crédits, adversaires : ${ecoEnemy[r.roundNum]?.value.toLocaleString("fr")}`}>
                    {eco[r.roundNum].kind}
                  </small>
                )}
              </div>
            ))}
          </div>
          {round === null ? (
            <p className="muted vrhint">Clique sur une manche pour voir ses éliminations.</p>
          ) : (() => {
            const r = m.roundResults![round];
            const who = (id?: string) => m.players.find((x) => x.subject === id);
            const label = (id?: string) => {
              const x = who(id);
              return x ? nameOf(x).name || d.agents[x.characterId.toLowerCase()]?.name || "?" : "?";
            };
            const face = (id?: string) => {
              const x = who(id);
              return x ? <img src={d.agents[x.characterId.toLowerCase()]?.icon} alt="" /> : <span />;
            };
            const side = (id?: string) => (who(id)?.teamId === myTeam ? "ally" : "foe");
            const kills = r.playerStats.flatMap((p) => p.kills ?? []).sort((a, b) => a.roundTime - b.roundTime);
            const weapon = (k: (typeof kills)[0]) => {
              const f = k.finishingDamage;
              if (!f) return null;
              if (f.damageType === "Weapon") {
                const w = d.weapons[f.damageItem.toLowerCase()];
                return w ? <img className="vkw" src={w.icon} alt={w.name} title={w.name} /> : <small>Arme d'agent</small>;
              }
              return <small>{f.damageType === "Melee" ? "Couteau" : f.damageType === "Bomb" ? "Spike" : "Compétence"}</small>;
            };
            return (
              <div className="vround-detail">
                <div className="vrdhead">
                  <b className={r.winningTeam === myTeam ? "good" : "bad"}>Manche {round + 1} : {r.winningTeam === myTeam ? "gagnée" : "perdue"}</b>
                  <span className="muted">{RESULT_FR[r.roundResult] ?? r.roundResult}</span>
                  {eco[round] && <span className="muted">Équipement : {eco[round].value.toLocaleString("fr")} contre {ecoEnemy[round]?.value.toLocaleString("fr")} crédits</span>}
                </div>
                {r.bombPlanter && (
                  <p className="vspike">Spike posé{r.plantSite ? ` sur le site ${r.plantSite}` : ""} à {mmss((r.plantRoundTime ?? 0) / 1000)} par <b className={side(r.bombPlanter)}>{label(r.bombPlanter)}</b>
                    {r.bombDefuser && <>, désamorcé à {mmss((r.defuseRoundTime ?? 0) / 1000)} par <b className={side(r.bombDefuser)}>{label(r.bombDefuser)}</b></>}
                  </p>
                )}
                <ol className="vkills">
                  {kills.map((k, i) => (
                    <li key={i}>
                      <span className="vkt">{mmss(k.roundTime / 1000)}</span>
                      <span className={`vkp ${side(k.killer)}`}>{face(k.killer)}{label(k.killer)}</span>
                      <span className="vkweap">{weapon(k)}</span>
                      <span className={`vkp ${side(k.victim)}`}>{face(k.victim)}{label(k.victim)}</span>
                    </li>
                  ))}
                  {kills.length === 0 && <li className="muted">Aucune élimination dans cette manche.</li>}
                </ol>
              </div>
            );
          })()}
        </section>
      )}
      <section className="card">
        <table className="board vboard">
          <colgroup>
            <col style={{ width: 52 }} /><col /><col style={{ width: 54 }} /><col style={{ width: 130 }} /><col style={{ width: 120 }} />
            <col style={{ width: 60 }} /><col style={{ width: 70 }} /><col style={{ width: 70 }} /><col style={{ width: 54 }} /><col style={{ width: 54 }} /><col style={{ width: 96 }} />
          </colgroup>
          {teams.map((t) => {
            const list = m.players.filter((x) => x.teamId === t).sort((a, b) => st[b.subject].acs - st[a.subject].acs);
            const win = vTeamWon(m, t);
            return (
              <tbody key={t}>
                <tr className="bhead">
                  <th colSpan={2}>{teams.length > 1 && <><b className={win ? "good" : "bad"}>{win ? "Victoire" : "Défaite"}</b> <span className="muted">{t === myTeam ? "Ton équipe" : "Adversaires"}</span></>}</th>
                  <th>Rang</th><th>ACS</th><th>K / D / A</th><th>+/-</th><th>ADR</th><th>HS</th><th>FK</th><th>FD</th><th>Multi</th>
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
                      <td className="c vmulti">{mkLabel(s.mk).join(", ") || "–"}</td>
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
type Form = number[];
type Party = { label: string; c: number };

function LiveCard({ p, d, name, r, form, party, self, go }: { p: LiveP; d: VD; name?: VName; r?: VRank; form?: Form; party?: Party; self: string; go: Go }) {
  const a = d.agents[p.agent.toLowerCase()];
  const hidden = p.incognito && p.puuid !== self;
  const net = form?.reduce((x, y) => x + y, 0) ?? 0;
  const tier = r && r.tier > 2 ? d.tiers[r.tier] : undefined;
  const open = () => !hidden && go({ page: "v-profil", puuid: p.puuid });
  return (
    <div
      className={`vlive ${p.puuid === self ? "me" : ""} ${hidden ? "hidden" : ""} ${party ? `inparty p${party.c}` : ""}`}
      onClick={open}
      role={hidden ? undefined : "button"}
      tabIndex={hidden ? -1 : 0}
      onKeyDown={(e) => e.key === "Enter" && open()}
    >
      <div className="vlface">
        {a ? <img className={p.locked === false ? "hover" : ""} src={a.icon} alt={a.name} /> : <span />}
        {a?.roleIcon && <img className="vlrole" src={a.roleIcon} alt="" title={a.role} />}
      </div>
      <div className="vlid">
        <b>{hidden ? "Joueur masqué" : name ? name.GameName : "…"}{!hidden && name && <small>#{name.TagLine}</small>}</b>
        <small>
          {a ? a.name : "Pas encore choisi"}
          {party && <em className={`vparty p${party.c}`}>{party.label}</em>}
          {p.locked === true && <em className="vlock">Verrouillé</em>}
          {p.level > 0 && <span className="vlvl">niv. {p.level}</span>}
        </small>
      </div>
      <div className="vlr">
        {!r ? <span className="muted">…</span> : (
          <>
            {tier ? <img src={tier.icon} alt="" /> : <span className="emblem none" />}
            <div>
              <b style={tier ? { color: tier.color } : undefined}>{tier ? `${tier.name}, ${r.rr} RR` : "Non classé"}</b>
              <small>{r.peak > 2 ? `Max ${d.tiers[r.peak]?.name}` : "Pas de rang max"}</small>
            </div>
          </>
        )}
      </div>
      <div className="vlwr">
        {r && r.games > 0 ? (
          <><b className={r.wins / r.games >= 0.5 ? "good" : "bad"}>{pct(r.wins / r.games)}</b><small>{r.games} parties</small></>
        ) : <small>–</small>}
      </div>
      <div className="vlform" title="5 dernières parties classées">
        {form && form.length > 0 ? (
          <>
            <div>{[...form].reverse().map((x, i) => <i key={i} className={x > 0 ? "fw" : x < 0 ? "fl" : ""} />)}</div>
            <small className={net >= 0 ? "good" : "bad"}>{net > 0 ? "+" : ""}{net} RR</small>
          </>
        ) : <small>–</small>}
      </div>
    </div>
  );
}

export function ValoLive({ phase, self, go }: { phase: VPhase; self: string; go: Go }) {
  const [d, setD] = useState<VD | null>(null);
  const [game, setGame] = useState<{ map: string; queue: string; players: LiveP[] } | null>(null);
  const [names, setNames] = useState<Record<string, VName>>({});
  const [ranks, setRanks] = useState<Record<string, VRank>>({});
  const [forms, setForms] = useState<Record<string, Form>>({});
  const [parties, setParties] = useState<Record<string, string>>({});

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
      vParties(players.map((p) => p.puuid), self).then((x) => !stop && setParties(x));
      const missing = players.map((p) => p.puuid).filter((p) => !names[p]);
      if (missing.length) vNames(missing).then((list) => !stop && setNames((o) => ({ ...o, ...Object.fromEntries(list.map((n) => [n.Subject, n])) })), () => {});
      for (const p of players) {
        vRank(p.puuid).then((r) => !stop && setRanks((o) => (o[p.puuid] ? o : { ...o, [p.puuid]: r })));
        vRRUpdates(p.puuid).then((u) => !stop && setForms((o) => (o[p.puuid] ? o : { ...o, [p.puuid]: Object.values(u).slice(0, 5).map((x) => x.earned) })));
      }
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
      <div className="waiting">
        <div>
          <span className="wstatus"><i />{phase === "PREGAME" || phase === "INGAME" ? "Chargement de la partie" : "En attente d'une partie"}</span>
          <h1>Lance une file dans VALORANT</h1>
          <p className="muted">Ethra suit la partie depuis le client Riot, de la sélection des agents à la fin du match.</p>
          <div className="lbtns">
            <button className="primary" onClick={() => go({ page: "v-profil" })}>Voir mon profil</button>
            <button className="ghostbtn" onClick={() => go({ page: "v-agents" })}>Tier list des agents</button>
          </div>
        </div>
        <ol className="wsteps">
          <li><b>Sélection des agents</b><small>Rang, winrate de l'acte, forme récente et groupes de ton équipe.</small></li>
          <li><b>En partie</b><small>Les mêmes infos pour les adversaires, avec leur rang moyen. Les joueurs masqués le restent.</small></li>
          <li><b>Après le match</b><small>RR gagnés, détail des manches et tableau des scores dans ton profil.</small></li>
        </ol>
      </div>
    );
  }
  const me = game.players.find((p) => p.puuid === self);
  const teams = [...new Set([me?.team ?? "", ...game.players.map((p) => p.team)])].filter((t, i, a) => t || a.length === 1);
  const map = d.maps[game.map];
  const pre = phase === "PREGAME";
  const locked = game.players.filter((p) => p.locked).length;
  const groups = new Map<string, string[]>();
  for (const p of game.players) if (parties[p.puuid]) groups.set(parties[p.puuid], [...(groups.get(parties[p.puuid]) ?? []), p.puuid]);
  const partyOf: Record<string, Party> = {};
  [...groups.values()].filter((g) => g.length > 1).forEach((g, i) => g.forEach((id) => (partyOf[id] = { label: PARTY_LABEL[Math.min(g.length, 5)], c: i % 5 })));
  const avg = (list: LiveP[]) => {
    const tiers = list.map((p) => ranks[p.puuid]?.tier ?? 0).filter((t) => t > 2);
    return tiers.length ? Math.round(tiers.reduce((a, t) => a + t, 0) / tiers.length) : 0;
  };
  return (
    <div className="live vlivepage">
      <div className="phero vmhead" style={map ? { backgroundImage: `linear-gradient(90deg, var(--bg) 25%, color-mix(in srgb, var(--bg) 65%, transparent) 60%, transparent), url(${map.splash})` } : undefined}>
        <div className="pid">
          <span className={`vphase ${pre ? "pre" : "in"}`}><i />{pre ? "Sélection des agents" : "En partie"}</span>
          <h1>{map?.name ?? "Carte"} <span>{queueLabel(game.queue)}</span></h1>
          <small>{pre ? `${locked} agent${locked > 1 ? "s" : ""} verrouillé${locked > 1 ? "s" : ""} sur ${game.players.length}. Les adversaires apparaîtront au lancement de la partie.` : "Clique sur un joueur pour ouvrir son profil."}</small>
        </div>
      </div>
      <div className="vteams">
        {teams.map((t) => {
          const list = game.players.filter((p) => p.team === t);
          const at = avg(list);
          return (
            <section key={t} className={`card vteam ${t === me?.team ? "mine" : "enemy"}`}>
              <div className="vthead">
                <h3>{t === me?.team ? "Ton équipe" : "Adversaires"}</h3>
                <span className="vtparties">
                  {[...new Map(list.filter((p) => partyOf[p.puuid]).map((p) => [partyOf[p.puuid].c, partyOf[p.puuid]])).values()].map((x) => (
                    <em key={x.c} className={`vparty p${x.c}`}>{x.label}</em>
                  ))}
                </span>
                {at > 0 && d.tiers[at] && (
                  <span className="vtavg"><img src={d.tiers[at].icon} alt="" />Rang moyen <b style={{ color: d.tiers[at].color }}>{d.tiers[at].name}</b></span>
                )}
              </div>
              <div className="vlcols"><span /><span>Joueur</span><span>Rang</span><span>Winrate</span><span>Forme</span></div>
              {list.map((p) => <LiveCard key={p.puuid} p={p} d={d} name={names[p.puuid]} r={ranks[p.puuid]} form={forms[p.puuid]} party={partyOf[p.puuid]} self={self} go={go} />)}
            </section>
          );
        })}
      </div>
    </div>
  );
}
