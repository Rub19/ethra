import { createRoot } from "react-dom/client";
import { useEffect, useState } from "react";
import { DD, ddragon, img, live, mmss, settings } from "./lcu";
import { Alert, ICON, Row, SkillHint, Widget } from "./Widget";
import { champId, opggBuild } from "./opgg";
import "./style.css";

const DRAGON_FIRST = 300, DRAGON_RESPAWN = 300, BARON_FIRST = 1200, BARON_RESPAWN = 360, INHIB_RESPAWN = 300, ALERT = 30;
const KEYS = ["Q", "W", "E", "R"];

function Overlay() {
  const [g, setG] = useState<any>(null);
  const [d, setD] = useState<DD | null>(null);
  const [cfg, setCfg] = useState(settings);
  const [skills, setSkills] = useState<{ key: string; order: number[]; items: number[] } | null>(null);

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
    if (!key || !d || !(cfg.ovSkill || cfg.ovItem) || skills?.key === key) return;
    const champ = champId(d, meP.championName);
    const aram = g.gameData.mapNumber === 12;
    opggBuild(d, champ, aram ? "mid" : meP.position || undefined, aram ? "aram" : "ranked").then(
      (b) => setSkills({ key, order: b?.skills ?? [], items: b ? [...b.order, ...b.boots.slice(0, 1), ...b.core] : [] }),
      () => setSkills({ key, order: [], items: [] }),
    );
  }, [key, d, cfg.ovSkill, cfg.ovItem]);

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
  const deadEnemies = players.filter((p) => p.team === enemyTeam && p.isDead && p.respawnTimer > 0).sort((a, b) => b.respawnTimer - a.respawnTimer);
  const owned = new Set<number>(meP.items.map((it: any) => it.itemID));
  // ponytail: missing gold uses the item's full price, ignoring components already bought; subtract component prices if players ask for precision
  const nextItem = skills?.items.find((id) => !owned.has(id));
  const nextPrice = nextItem && d ? d.items[nextItem]?.gold.total ?? 0 : 0;
  const gold = Math.floor(g.activePlayer.currentGold);
  const opp = meP.position ? players.find((p) => p.team === enemyTeam && p.position === meP.position) : undefined;

  const abilities = g.activePlayer.abilities ?? {};
  const spent = KEYS.reduce((a, k) => a + (abilities[k]?.abilityLevel ?? 0), 0);
  const nextSlot = skills?.order[spent];
  const nextKey = spent < g.activePlayer.level && nextSlot ? KEYS[nextSlot - 1] : null;

  const sign = (x: number) => `${x >= 0 ? "+" : ""}${x}`;
  const tone = (x: number): "good" | "bad" => (x >= 0 ? "good" : "bad");
  const champ = d ? champId(d, meP.championName) : 0;
  const objIcon = (label: string) => (label.startsWith("Dragon") ? ICON.dragon : label.startsWith("Nashor") ? ICON.baron : ICON.inhib);
  const period = (label: string) => (label.startsWith("Dragon") ? DRAGON_RESPAWN : label.startsWith("Nashor") ? BARON_RESPAWN : INHIB_RESPAWN);
  const all = [...timers, ...inhibs];
  const itemDiff = opp ? worth(meP) - worth(opp) : 0;

  return (
    <>
      {cfg.ovAlerts && alerts.length > 0 && (
        <div className="ovalert">
          {alerts.map((a) => <Alert key={a.label} icon={objIcon(a.label)} name={a.label} secs={Math.ceil(a.at - t)} />)}
        </div>
      )}
      {cfg.ovSkill && nextKey && d && (
        <div className="ovskill">
          <SkillHint skill={nextKey} level={g.activePlayer.level} icon={champ ? img.ability(d, d.champs[champ].id, nextKey) : undefined} />
        </div>
      )}
      <div className="ov">
        {cfg.ovTimers && all.length > 0 && (
          <Widget title="Objectifs">
            {all.map((x) => (
              <Row key={x.label} icon={objIcon(x.label)} label={x.label} value={x.at <= t ? "UP" : mmss(x.at - t)} tone={x.at <= t ? "up" : undefined} bar={x.at <= t ? undefined : 1 - (x.at - t) / period(x.label)} />
            ))}
          </Widget>
        )}
        {cfg.ovDeaths && d && deadEnemies.length > 0 && (
          <Widget title="Ennemis morts">
            {deadEnemies.map((p) => {
              const id = champId(d, p.championName);
              return <Row key={p.riotId ?? p.summonerName} icon={id ? img.champ(d, id) : undefined} label={d.champs[id]?.name ?? p.championName} value={`${Math.ceil(p.respawnTimer)}s`} tone="good" />;
            })}
          </Widget>
        )}
        {cfg.ovItem && d && nextItem && (
          <Widget title="Prochain objet">
            <Row icon={img.item(d, nextItem)} label={d.items[nextItem]?.name ?? ""} value={gold >= nextPrice ? "Achetable" : `-${(nextPrice - gold).toLocaleString("fr")}`} tone={gold >= nextPrice ? "up" : undefined} bar={nextPrice ? gold / nextPrice : undefined} />
          </Widget>
        )}
        {cfg.ovBench && opp && (
          <Widget title={`vs ${opp.championName}`}>
            <Row label="CS" value={sign(s.creepScore - opp.scores.creepScore)} tone={tone(s.creepScore - opp.scores.creepScore)} />
            <Row label="Objets" value={`${sign(Math.round(itemDiff / 100) / 10)}k`} tone={tone(itemDiff)} />
            <Row label="Niveau" value={sign(meP.level - opp.level)} tone={tone(meP.level - opp.level)} />
          </Widget>
        )}
        {cfg.ovGold && d && (
          <Widget title="Valeur des objets">
            <Row label="Équipes" value={`${diff >= 0 ? "+" : ""}${(diff / 1000).toFixed(1)}k`} tone={tone(diff)} />
          </Widget>
        )}
        {cfg.ovStats && (
          <Widget title="Toi">
            <Row label="KDA" value={`${s.kills} / ${s.deaths} / ${s.assists}`} />
            <Row label="CS/min" value={(s.creepScore / Math.max(1, t / 60)).toFixed(1)} />
            <Row label="Or" value={Math.floor(g.activePlayer.currentGold).toLocaleString("fr")} />
          </Widget>
        )}
      </div>
    </>
  );
}

document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea")) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(<Overlay />);
