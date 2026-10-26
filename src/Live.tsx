import { useEffect, useState } from "react";
import { DD, ddragon, Game, history, img, kda, lcu, Queue, rankLabel, ranked, summoner } from "./lcu";

type Slot = { puuid: string; championId: number; position?: string; name?: string };

const POS: Record<string, string> = { top: "Top", jungle: "Jungle", middle: "Mid", bottom: "ADC", utility: "Support" };

function PlayerCard({ s, d, onOpen }: { s: Slot; d: DD; onOpen: (p: string) => void }) {
  const [info, setInfo] = useState<{ name: string; q?: Queue; games: Game[] } | null>(null);
  useEffect(() => {
    if (!s.puuid) return;
    Promise.all([summoner(s.puuid), ranked(s.puuid).catch(() => ({}) as Record<string, Queue>), history(s.puuid, 20).catch(() => [])]).then(
      ([sum, r, games]) => setInfo({ name: `${sum.gameName}#${sum.tagLine}`, q: r.RANKED_SOLO_5x5, games }),
      () => {},
    );
  }, [s.puuid]);

  const g = info?.games ?? [];
  const wins = g.filter((x) => x.participants[0].stats.win).length;
  const onChamp = g.filter((x) => x.participants[0].championId === s.championId);
  const champWins = onChamp.filter((x) => x.participants[0].stats.win).length;
  let streak = 0;
  for (const x of g) if (x.participants[0].stats.win === g[0]?.participants[0].stats.win) streak++; else break;
  const tags: string[] = [];
  if (onChamp.length >= 12) tags.push("OTP");
  if (streak >= 3) tags.push(g[0].participants[0].stats.win ? `${streak} win d'affilée` : `${streak} défaites d'affilée`);
  if (s.championId && onChamp.length === 0 && g.length) tags.push("Premier match sur ce champion (20 dernières)");

  return (
    <div className="pcard" onClick={() => s.puuid && onOpen(s.puuid)}>
      {s.championId ? <img className="champ" src={img.champ(d, s.championId)} /> : <div className="champ ghost" />}
      <div className="pinfo">
        <b>{info?.name ?? s.name ?? (s.puuid ? "…" : "Joueur masqué")}</b>
        <small>{s.position ? `${POS[s.position] ?? s.position} · ` : ""}{s.championId ? d.champs[s.championId]?.name : "Pas encore choisi"}</small>
      </div>
      {info && (
        <div className="pnums">
          <div><b>{rankLabel(info.q)}</b><small>{info.q?.wins ?? 0}V {info.q?.losses ?? 0}D</small></div>
          <div><b className={wins / Math.max(1, g.length) >= 0.5 ? "good" : "bad"}>{g.length ? Math.round((wins / g.length) * 100) : 0}%</b><small>{g.length} récentes</small></div>
          <div>
            <b>{onChamp.length ? `${Math.round((champWins / onChamp.length) * 100)}%` : "-"}</b>
            <small>{onChamp.length} sur le champ · {onChamp.length ? (onChamp.reduce((a, x) => a + kda(x.participants[0].stats), 0) / onChamp.length).toFixed(1) : "-"} KDA</small>
          </div>
        </div>
      )}
      <div className="tags">{tags.map((t) => <span key={t}>{t}</span>)}</div>
    </div>
  );
}

export default function Live({ phase, onOpen }: { phase: string; onOpen: (p: string) => void }) {
  const [d, setD] = useState<DD | null>(null);
  const [teams, setTeams] = useState<[Slot[], Slot[]]>([[], []]);

  useEffect(() => {
    ddragon().then(setD);
    const load = async () => {
      if (phase === "ChampSelect") {
        const s = await lcu<any>("/lol-champ-select/v1/session").catch(() => null);
        if (!s) return;
        const slot = (m: any): Slot => ({ puuid: m.puuid ?? "", championId: m.championId || m.championPickIntent, position: m.assignedPosition, name: m.gameName });
        setTeams([s.myTeam.map(slot), s.theirTeam.map(slot)]);
      } else if (["InProgress", "Reconnect", "GameStart"].includes(phase)) {
        const s = await lcu<any>("/lol-gameflow/v1/session").catch(() => null);
        const slot = (m: any): Slot => ({ puuid: m.puuid ?? "", championId: m.championId, position: m.selectedPosition?.toLowerCase(), name: m.gameName ?? m.summonerName });
        if (s) setTeams([s.gameData.teamOne.map(slot), s.gameData.teamTwo.map(slot)]);
      }
    };
    load();
    const id = setInterval(load, 3000);
    return () => clearInterval(id);
  }, [phase]);

  if (!["ChampSelect", "InProgress", "Reconnect", "GameStart"].includes(phase))
    return (
      <div className="empty">
        <h2>Pas de partie en cours</h2>
        <p>Dès que tu entres en sélection des champions, Ethra affiche les rangs, winrates et mains de chaque joueur, et importe tes runes automatiquement.</p>
      </div>
    );
  if (!d) return null;
  return (
    <div className="live">
      <h2>{phase === "ChampSelect" ? "Sélection des champions" : "Partie en cours"}</h2>
      <div className="lteams">
        {teams.map((t, i) => (
          <div key={i}>
            <h3 className={i ? "bad" : "good"}>{i ? "Ennemis" : "Ton équipe"}</h3>
            {t.map((s, j) => <PlayerCard key={s.puuid || `${i}-${j}`} s={s} d={d} onOpen={onOpen} />)}
          </div>
        ))}
      </div>
    </div>
  );
}
