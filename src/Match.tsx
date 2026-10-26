import { useEffect, useRef, useState } from "react";
import { toBlob } from "html-to-image";
import { writeImage } from "@tauri-apps/plugin-clipboard-manager";
import { cs, DD, ddragon, Game, gameDetails, img, kda, lcu, mmss, n, Participant, queueNames } from "./lcu";
import { badges, Scoreboard } from "./Profile";
import type { Go } from "./main";

type Frame = { participantFrames: Record<string, { minionsKilled: number; jungleMinionsKilled: number; totalGold: number; xp: number; level: number }> };
type Metric = "cs" | "gold" | "xp";

const ME = "#c08a1e", OPP = "#7189f0";
const SMITE = 11;

function laneOpponent(g: Game, p: Participant, frames: Frame[]) {
  const enemies = g.participants.filter((x) => x.teamId !== p.teamId);
  const smite = (x: Participant) => x.spell1Id === SMITE || x.spell2Id === SMITE;
  if (smite(p)) return enemies.find(smite);
  const f = frames[Math.min(10, frames.length - 1)]?.participantFrames ?? {};
  const csAt = (x: Participant) => f[x.participantId]?.minionsKilled ?? 0;
  // ponytail: pairs laners by CS rank at 10 min within each team; swap for real positions if Riot exposes them in history
  const byCs = (list: Participant[]) => list.filter((x) => !smite(x)).sort((a, b) => csAt(b) - csAt(a));
  const mine = byCs(g.participants.filter((x) => x.teamId === p.teamId));
  return byCs(enemies)[mine.findIndex((x) => x.participantId === p.participantId)];
}

function score(g: Game, p: Participant, f: (x: Participant) => number) {
  const v = f(p);
  return Math.round((100 * g.participants.filter((x) => f(x) < v).length) / Math.max(1, g.participants.length - 1));
}
const grade = (s: number) => (s >= 80 ? "Excellent" : s >= 60 ? "Bon" : s >= 40 ? "Moyen" : "À travailler");

function Chart({ series, unit }: { series: { name: string; color: string; pts: number[] }[]; unit: string }) {
  const [hover, setHover] = useState<number | null>(null);
  const w = 640, h = 240, l = 44, r = 90, t = 12, b = 26;
  const len = Math.max(...series.map((s) => s.pts.length));
  const max = Math.max(1, ...series.flatMap((s) => s.pts));
  const x = (i: number) => l + (i / Math.max(1, len - 1)) * (w - l - r);
  const y = (v: number) => t + (1 - v / max) * (h - t - b);
  const ticks = [0, 0.5, 1].map((k) => Math.round(max * k));
  const fmt = (v: number) => (unit === "or" ? v.toLocaleString("fr") : String(v));
  return (
    <div className="chart">
      <div className="legend">{series.map((s) => <span key={s.name}><i style={{ background: s.color }} />{s.name}</span>)}</div>
      <svg
        viewBox={`0 0 ${w} ${h}`}
        onMouseMove={(e) => {
          const box = e.currentTarget.getBoundingClientRect();
          const px = ((e.clientX - box.left) / box.width) * w;
          setHover(Math.max(0, Math.min(len - 1, Math.round(((px - l) / (w - l - r)) * (len - 1)))));
        }}
        onMouseLeave={() => setHover(null)}
        role="img"
        aria-label={series.map((s) => `${s.name} : ${fmt(s.pts.at(-1) ?? 0)} ${unit} en fin de partie`).join(", ")}
      >
        {ticks.map((v) => (
          <g key={v}>
            <line x1={l} x2={w - r} y1={y(v)} y2={y(v)} className="grid" />
            <text x={l - 6} y={y(v) + 4} textAnchor="end" className="axis">{fmt(v)}</text>
          </g>
        ))}
        {Array.from({ length: len }, (_, i) => i).filter((i) => i % 5 === 0).map((i) => (
          <text key={i} x={x(i)} y={h - 6} textAnchor="middle" className="axis">{i}</text>
        ))}
        {len > 14 && <line x1={x(14)} x2={x(14)} y1={t} y2={h - b} className="mark14" />}
        {series.map((s) => <polyline key={s.name} points={s.pts.map((v, i) => `${x(i)},${y(v)}`).join(" ")} fill="none" stroke={s.color} strokeWidth="2" strokeLinejoin="round" />)}
        {series
          .map((s) => ({ s, ly: y(s.pts.at(-1) ?? 0) + 4 }))
          .sort((a, b) => a.ly - b.ly)
          .map((e, i, all) => {
            const ly = i > 0 ? Math.max(e.ly, all[i - 1].ly + 14) : e.ly;
            all[i] = { ...e, ly };
            return <text key={e.s.name} x={x(e.s.pts.length - 1) + 8} y={ly} className="endlabel">{e.s.name}</text>;
          })}
        {hover !== null && (
          <g>
            <line x1={x(hover)} x2={x(hover)} y1={t} y2={h - b} className="cross" />
            {series.map((s) => s.pts[hover] !== undefined && <circle key={s.name} cx={x(hover)} cy={y(s.pts[hover])} r="4" fill={s.color} stroke="var(--panel)" strokeWidth="2" />)}
          </g>
        )}
      </svg>
      {hover !== null && (
        <div className="tip" style={{ left: `${(x(hover) / w) * 100}%` }}>
          <b>{hover} min</b>
          {series.map((s) => <span key={s.name}><i style={{ background: s.color }} />{s.name} <b>{fmt(s.pts[hover] ?? 0)}</b></span>)}
        </div>
      )}
      <small className="muted center">Minutes de jeu · ligne pointillée : 14 min (fin de la phase de lane)</small>
    </div>
  );
}

function ShareCard({ g, p, d, label, mvp }: { g: Game; p: Participant; d: DD; label: string; mvp: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const [msg, setMsg] = useState("");
  const s = p.stats;
  const c = d.champs[p.championId];
  const remake = g.gameDuration < 300;
  const copy = async () => {
    if (!ref.current) return;
    setMsg("…");
    try {
      const blob = await toBlob(ref.current, { pixelRatio: 2, cacheBust: true, filter: (el) => !(el as HTMLElement).classList?.contains("noshot") });
      if (!blob) throw new Error("image vide");
      await writeImage(new Uint8Array(await blob.arrayBuffer()));
      setMsg("Image copiée, colle-la sur Discord");
    } catch (e) {
      setMsg(`Copie impossible : ${e}`);
    }
  };
  const stat = (v: string | number, l: string) => <div><b>{typeof v === "number" ? v.toLocaleString("fr") : v}</b><small>{l}</small></div>;
  return (
    <div className="share" ref={ref} style={{ backgroundImage: `linear-gradient(90deg, #0b0f17 30%, #0b0f17cc 55%, #0b0f1733), url(https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${c?.id}_0.jpg)` }}>
      <div className="shtop">
        <b>{label}</b>
        <span className={remake ? "" : s.win ? "good" : "bad"}>{remake ? "REMAKE" : s.win ? "VICTOIRE" : "DÉFAITE"}</span>
        {mvp && <span className="hl">MVP</span>}
        <span className="muted">{mmss(g.gameDuration)}</span>
        <span className="shbrand">ETHRA</span>
      </div>
      <div className="shbig"><b>{n(s, "totalDamageDealtToChampions").toLocaleString("fr")}</b><span>dégâts aux champions</span></div>
      <div className="shstats">
        {stat(`${n(s, "kills")} / ${n(s, "deaths")} / ${n(s, "assists")}`, "KDA")}
        {stat(n(s, "largestCriticalStrike"), "Plus gros critique")}
        {stat(n(s, "goldEarned"), "Or gagné")}
        {stat(cs(s), "CS")}
        {stat(n(s, "totalHeal"), "Soins")}
      </div>
      <div className="shitems">
        {[n(s, "perk0"), n(s, "perkSubStyle")].filter((id) => d.runes[id]).map((id) => <img key={id} className="rn" src={img.rune(d, id)} />)}
        <i />
        {[0, 1, 2, 3, 4, 5, 6].map((k) => n(s, `item${k}`)).filter(Boolean).map((id, i) => <img key={i} src={img.item(d, id)} />)}
      </div>
      <div className="shact noshot">
        {msg && <small className="muted">{msg}</small>}
        <button className="sharebtn" onClick={copy}>Copier l'image</button>
      </div>
    </div>
  );
}

export default function Match({ gameId, puuid, go }: { gameId: number; puuid: string; go: Go }) {
  const [d, setD] = useState<DD | null>(null);
  const [g, setG] = useState<Game | null>(null);
  const [frames, setFrames] = useState<Frame[]>([]);
  const [queues, setQueues] = useState<Record<number, string>>({});
  const [metric, setMetric] = useState<Metric>("cs");
  const [err, setErr] = useState("");

  useEffect(() => {
    Promise.all([ddragon(), gameDetails(gameId), queueNames().catch(() => ({}))]).then(([dd, gg, q]) => (setD(dd), setG(gg), setQueues(q)), (e) => setErr(String(e)));
    lcu<{ frames: Frame[] }>(`/lol-match-history/v1/game-timelines/${gameId}`).then((t) => setFrames(t.frames ?? []), () => {});
  }, [gameId]);

  if (err) return <div className="empty"><p>{err}</p></div>;
  if (!d || !g) return <div className="empty"><p>Chargement…</p></div>;

  const id = g.participantIdentities.find((x) => x.player.puuid === puuid)?.participantId;
  const p = g.participants.find((x) => x.participantId === id);
  if (!p) return <div className="empty"><p>Joueur absent de cette partie.</p></div>;
  const s = p.stats;
  const min = g.gameDuration / 60;
  const opp = frames.length ? laneOpponent(g, p, frames) : undefined;
  const oppName = opp && g.participantIdentities.find((x) => x.participantId === opp.participantId)?.player.gameName;
  const val = (pid: number, m: Metric) => frames.map((f) => {
    const x = f.participantFrames[pid];
    return !x ? 0 : m === "cs" ? x.minionsKilled + x.jungleMinionsKilled : m === "gold" ? x.totalGold : x.xp;
  });
  const at = Math.min(14, frames.length - 1);
  const diff = (m: Metric) => (opp && at > 0 ? val(p.participantId, m)[at] - val(opp.participantId, m)[at] : 0);
  const team = g.participants.filter((x) => x.teamId === p.teamId);
  const teamKills = team.reduce((a, x) => a + n(x.stats, "kills"), 0);
  const kp = (x: Participant) => (n(x.stats, "kills") + n(x.stats, "assists")) / Math.max(1, g.participants.filter((y) => y.teamId === x.teamId).reduce((a, y) => a + n(y.stats, "kills"), 0));
  const scores = [
    { k: "Farming", v: score(g, p, (x) => cs(x.stats)) },
    { k: "Combat", v: score(g, p, (x) => kda(x.stats) + kp(x) * 3 + n(x.stats, "totalDamageDealtToChampions") / 10000) },
    ...(g.mapId === 11 ? [{ k: "Vision", v: score(g, p, (x) => n(x.stats, "visionScore")) }] : []),
    { k: "Objectifs", v: score(g, p, (x) => n(x.stats, "damageDealtToObjectives")) },
  ];
  const best = [...scores].sort((a, b) => b.v - a.v)[0];
  const csD = diff("cs"), goldD = diff("gold");
  const teams = (g as any).teams as { teamId: number; towerKills: number; dragonKills: number; baronKills: number; riftHeraldKills: number; hordeKills: number }[] | undefined;
  const remake = g.gameDuration < 300;

  return (
    <div className="matchpage">
      <div className={`mhead ${remake ? "" : s.win ? "win" : "loss"}`}>
        <img className="champ" src={img.champ(d, p.championId)} />
        <div>
          <h1>{remake ? "Remake" : s.win ? "Victoire" : "Défaite"}</h1>
          <small>{queues[g.queueId] ?? `File ${g.queueId}`} · {mmss(g.gameDuration)} · {new Date(g.gameCreation).toLocaleString("fr")}</small>
        </div>
        <div className="badges">{badges(g, puuid).map((b) => <span key={b} className={b === "MVP" || b === "ACE" ? "gold" : ""}>{b}</span>)}</div>
        <button className="ghostbtn" onClick={() => go({ page: "champion", champ: p.championId })}>Build {d.champs[p.championId]?.name}</button>
      </div>

      <ShareCard g={g} p={p} d={d} label={queues[g.queueId] ?? `File ${g.queueId}`} mvp={badges(g, puuid).includes("MVP")} />

      <section className="card perf2">
        <div className="gauges">
          {scores.map((x) => (
            <div key={x.k} className="gauge">
              <svg viewBox="0 0 60 36" aria-hidden="true">
                <path d="M6 32 A24 24 0 0 1 54 32" className="track" />
                <path d="M6 32 A24 24 0 0 1 54 32" className="fill" pathLength={100} strokeDasharray={`${x.v} 100`} />
              </svg>
              <b>{x.v}</b>
              <span>{x.k}</span>
              <small>{grade(x.v)}</small>
            </div>
          ))}
        </div>
        <div className="summary">
          <small>Comparé aux 9 autres joueurs de la partie</small>
          <p>
            Ton point fort : <b className="hl">{best.k.toLowerCase()}</b>.{" "}
            {opp && at >= 10 ? (
              <>
                À {at} min, tu avais <b className="hl">{Math.abs(csD)} CS</b> et <b className="hl">{Math.abs(goldD).toLocaleString("fr")} or</b>{" "}
                {goldD >= 0 ? "d'avance" : "de retard"} sur {oppName ?? "ton adversaire"} ({d.champs[opp.championId]?.name}).
              </>
            ) : (
              <>{(cs(s) / min).toFixed(1)} CS/min, {Math.round((n(s, "kills") + n(s, "assists")) / Math.max(1, teamKills) * 100)}% de participation aux kills.</>
            )}
          </p>
        </div>
      </section>

      {frames.length > 1 && (
        <section className="card">
          <div className="bbar">
            <h3>Courbe de la partie</h3>
            <span className="grow" />
            <div className="seg">
              {([["cs", "CS"], ["gold", "Or"], ["xp", "XP"]] as const).map(([k, l]) => <button key={k} className={metric === k ? "on" : ""} onClick={() => setMetric(k)}>{l}</button>)}
            </div>
          </div>
          <Chart
            unit={metric === "gold" ? "or" : metric === "cs" ? "CS" : "XP"}
            series={[
              { name: "Toi", color: ME, pts: val(p.participantId, metric) },
              ...(opp ? [{ name: d.champs[opp.championId]?.name ?? "Adversaire", color: OPP, pts: val(opp.participantId, metric) }] : []),
            ]}
          />
        </section>
      )}

      {teams && (
        <div className="objs">
          {teams.map((t) => (
            <div key={t.teamId} className={`card ${t.teamId === p.teamId ? "mine" : ""}`}>
              <b>{t.teamId === p.teamId ? "Ton équipe" : "Adversaires"}</b>
              <span>{t.towerKills} tours</span>
              <span>{t.dragonKills} dragons</span>
              <span>{t.hordeKills ?? 0} larves</span>
              <span>{t.riftHeraldKills} héraut</span>
              <span>{t.baronKills} nashor</span>
            </div>
          ))}
        </div>
      )}

      <section className="card">
        <Scoreboard g={g} d={d} puuid={puuid} onOpen={(x) => go({ page: "profil", puuid: x })} />
      </section>
    </div>
  );
}
