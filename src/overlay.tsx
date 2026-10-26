import { createRoot } from "react-dom/client";
import { useEffect, useState } from "react";
import { DD, ddragon, live, mmss, settings } from "./lcu";
import "./style.css";

const DRAGON_FIRST = 300, DRAGON_RESPAWN = 300, BARON_FIRST = 1200, BARON_RESPAWN = 360;

function Overlay() {
  const [g, setG] = useState<any>(null);
  const [d, setD] = useState<DD | null>(null);
  const [cfg, setCfg] = useState(settings);

  useEffect(() => {
    ddragon().then(setD, () => {});
    const onStorage = () => setCfg(settings());
    window.addEventListener("storage", onStorage);
    const id = setInterval(() => live("allgamedata").then(setG, () => setG(null)), 1000);
    return () => (clearInterval(id), window.removeEventListener("storage", onStorage));
  }, []);

  if (!g || !cfg.ovEnabled || !g.activePlayer?.riotId) return null;

  const t: number = g.gameData.gameTime;
  const players: any[] = g.allPlayers;
  const meP = players.find((p) => p.riotId === g.activePlayer.riotId) ?? players[0];
  const s = meP.scores;
  const events: any[] = g.events?.Events ?? [];
  const last = (name: string) => events.filter((e) => e.EventName === name).at(-1)?.EventTime as number | undefined;
  const dragon = last("DragonKill");
  const baron = last("BaronKill");
  const nextDragon = dragon ? dragon + DRAGON_RESPAWN : DRAGON_FIRST;
  const nextBaron = baron ? baron + BARON_RESPAWN : BARON_FIRST;
  const worth = (team: string) =>
    players.filter((p) => p.team === team).reduce((sum, p) => sum + p.items.reduce((a: number, it: any) => a + (d?.items[it.itemID]?.gold.total ?? 0) * (it.count || 1), 0), 0);
  const diff = worth(meP.team) - worth(meP.team === "ORDER" ? "CHAOS" : "ORDER");
  const timer = (label: string, at: number) => (
    <div className="ovt"><span>{label}</span><b>{at <= t ? "UP" : mmss(at - t)}</b></div>
  );

  return (
    <div className="ov">
      {cfg.ovStats && (
        <div className="ovbox">
          <div className="ovt"><span>KDA</span><b>{s.kills}/{s.deaths}/{s.assists}</b></div>
          <div className="ovt"><span>CS/min</span><b>{(s.creepScore / Math.max(1, t / 60)).toFixed(1)}</b></div>
          <div className="ovt"><span>Or</span><b>{Math.floor(g.activePlayer.currentGold)}</b></div>
        </div>
      )}
      {cfg.ovGold && d && (
        <div className="ovbox">
          <div className="ovt"><span>Diff. or (objets)</span><b className={diff >= 0 ? "good" : "bad"}>{diff >= 0 ? "+" : ""}{(diff / 1000).toFixed(1)}k</b></div>
        </div>
      )}
      {cfg.ovTimers && g.gameData.mapNumber === 11 && (
        <div className="ovbox">
          {timer("Dragon", nextDragon)}
          {t >= BARON_FIRST - 120 && timer("Nashor", nextBaron)}
        </div>
      )}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<Overlay />);
