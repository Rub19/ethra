import { createRoot } from "react-dom/client";
import { useEffect, useState } from "react";
import { DD, ddragon, live, mmss, settings } from "./lcu";
import { champId, opggBuild } from "./opgg";
import "./style.css";

const DRAGON_FIRST = 300, DRAGON_RESPAWN = 300, BARON_FIRST = 1200, BARON_RESPAWN = 360, INHIB_RESPAWN = 300, ALERT = 30;
const KEYS = ["Q", "W", "E", "R"];

function Overlay() {
  const [g, setG] = useState<any>(null);
  const [d, setD] = useState<DD | null>(null);
  const [cfg, setCfg] = useState(settings);
  const [skills, setSkills] = useState<{ key: string; order: number[] } | null>(null);

  useEffect(() => {
    ddragon().then(setD, () => {});
    const onStorage = () => setCfg(settings());
    window.addEventListener("storage", onStorage);
    const id = setInterval(() => live("allgamedata").then(setG, () => setG(null)), 1000);
    return () => (clearInterval(id), window.removeEventListener("storage", onStorage));
  }, []);

  const players: any[] = g?.allPlayers ?? [];
  const meP = players.find((p) => p.riotId === g?.activePlayer?.riotId);
  const key = meP && d ? `${meP.championName}-${meP.position}-${g.gameData.mapNumber}` : "";

  useEffect(() => {
    if (!key || !d || !cfg.ovSkill || skills?.key === key) return;
    const champ = champId(d, meP.championName);
    const aram = g.gameData.mapNumber === 12;
    opggBuild(d, champ, aram ? "mid" : meP.position || undefined, aram ? "aram" : "ranked").then(
      (b) => setSkills({ key, order: b?.skills ?? [] }),
      () => setSkills({ key, order: [] }),
    );
  }, [key, d, cfg.ovSkill]);

  if (!g || !cfg.ovEnabled || !meP) return null;

  const t: number = g.gameData.gameTime;
  const sr = g.gameData.mapNumber === 11;
  const s = meP.scores;
  const events: any[] = g.events?.Events ?? [];
  const last = (name: string) => events.filter((e) => e.EventName === name).at(-1)?.EventTime as number | undefined;
  const dragon = last("DragonKill"), baron = last("BaronKill");
  const timers: { label: string; at: number }[] = sr
    ? [
        { label: "Dragon", at: dragon ? dragon + DRAGON_RESPAWN : DRAGON_FIRST },
        ...(t >= BARON_FIRST - 120 ? [{ label: "Nashor", at: baron ? baron + BARON_RESPAWN : BARON_FIRST }] : []),
      ]
    : [];
  const inhibs = events
    .filter((e) => e.EventName === "InhibKilled" && e.EventTime + INHIB_RESPAWN > t)
    .map((e) => {
      const name = String(e.InhibKilled);
      const lane = /_L\d/.test(name) ? "top" : /_C\d/.test(name) ? "mid" : /_R\d/.test(name) ? "bot" : "";
      return { label: `Inhib ${lane} ${name.includes("_T1_") ? "bleu" : "rouge"}`.replace("  ", " "), at: e.EventTime + INHIB_RESPAWN };
    });
  const alerts = timers.filter((x) => x.at > t && x.at - t <= ALERT);

  const worth = (p: any) => p.items.reduce((a: number, it: any) => a + (d?.items[it.itemID]?.gold.total ?? 0) * (it.count || 1), 0);
  const teamWorth = (team: string) => players.filter((p) => p.team === team).reduce((a, p) => a + worth(p), 0);
  const enemyTeam = meP.team === "ORDER" ? "CHAOS" : "ORDER";
  const diff = teamWorth(meP.team) - teamWorth(enemyTeam);
  const opp = meP.position ? players.find((p) => p.team === enemyTeam && p.position === meP.position) : undefined;

  const abilities = g.activePlayer.abilities ?? {};
  const spent = KEYS.reduce((a, k) => a + (abilities[k]?.abilityLevel ?? 0), 0);
  const nextSlot = skills?.order[spent];
  const nextKey = spent < g.activePlayer.level && nextSlot ? KEYS[nextSlot - 1] : null;

  const sign = (x: number) => `${x >= 0 ? "+" : ""}${x}`;

  return (
    <>
      {cfg.ovAlerts && alerts.length > 0 && (
        <div className="ovalert">
          {alerts.map((a) => <div key={a.label}><b>{a.label}</b> apparaît dans <b>{Math.ceil(a.at - t)} s</b></div>)}
        </div>
      )}
      {cfg.ovSkill && nextKey && (
        <div className="ovskill"><span>Monte</span><b>{nextKey}</b></div>
      )}
      <div className="ov">
        {cfg.ovStats && (
          <div className="ovbox">
            <div className="ovt"><span>KDA</span><b>{s.kills}/{s.deaths}/{s.assists}</b></div>
            <div className="ovt"><span>CS/min</span><b>{(s.creepScore / Math.max(1, t / 60)).toFixed(1)}</b></div>
            <div className="ovt"><span>Or</span><b>{Math.floor(g.activePlayer.currentGold)}</b></div>
          </div>
        )}
        {cfg.ovBench && opp && (
          <div className="ovbox">
            <div className="ovh">vs {opp.championName}</div>
            <div className="ovt"><span>CS</span><b className={s.creepScore >= opp.scores.creepScore ? "good" : "bad"}>{sign(s.creepScore - opp.scores.creepScore)}</b></div>
            <div className="ovt"><span>Objets</span><b className={worth(meP) >= worth(opp) ? "good" : "bad"}>{sign(Math.round((worth(meP) - worth(opp)) / 100) / 10)}k</b></div>
            <div className="ovt"><span>Niveau</span><b>{sign(meP.level - opp.level)}</b></div>
          </div>
        )}
        {cfg.ovGold && d && (
          <div className="ovbox">
            <div className="ovt"><span>Valeur objets équipe</span><b className={diff >= 0 ? "good" : "bad"}>{diff >= 0 ? "+" : ""}{(diff / 1000).toFixed(1)}k</b></div>
          </div>
        )}
        {cfg.ovTimers && timers.length + inhibs.length > 0 && (
          <div className="ovbox">
            {[...timers, ...inhibs].map((x) => (
              <div key={x.label} className="ovt"><span>{x.label}</span><b className={x.at <= t ? "good" : ""}>{x.at <= t ? "UP" : mmss(x.at - t)}</b></div>
            ))}
          </div>
        )}
      </div>
    </>
  );
}

document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea")) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(<Overlay />);
