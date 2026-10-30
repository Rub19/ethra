import { useEffect, useState } from "react";
import { DD, ddragon, Game, history, img, kda, lcu, Queue, ranked, summoner, TIERS } from "./lcu";
import { POSITIONS } from "./buildData";
import Builds from "./Builds";
import type { Go } from "./main";

type Slot = { puuid: string; championId: number; position?: string; name?: string; me?: boolean };

const POS: Record<string, string> = { top: "Top", jungle: "Jungle", middle: "Mid", bottom: "ADC", utility: "Support" };
const LIVE = ["ChampSelect", "InProgress", "Reconnect", "GameStart"];

function PlayerCard({ s, d, go }: { s: Slot; d: DD; go: Go }) {
  const [info, setInfo] = useState<{ name: string; q?: Queue; games: Game[] } | null>(null);
  useEffect(() => {
    if (!s.puuid) return;
    Promise.all([summoner(s.puuid), ranked(s.puuid).catch(() => ({}) as Record<string, Queue>), history(s.puuid, 20).catch(() => [])]).then(
      ([sum, r, games]) => setInfo({ name: sum.gameName, q: r.RANKED_SOLO_5x5, games }),
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
  if (streak >= 3) tags.push(g[0].participants[0].stats.win ? `${streak} wins` : `${streak} défaites`);
  if (s.championId && g.length && !onChamp.length) tags.push("1re sur ce champ");
  const q = info?.q;
  const tier = q?.tier && TIERS.includes(q.tier) ? q.tier : "";
  const c = d.champs[s.championId];

  return (
    <div
      className={`pcard ${s.me ? "me" : ""} ${s.puuid ? "link" : ""}`}
      onClick={() => s.puuid && go({ page: "profil", puuid: s.puuid })}
      style={c ? { backgroundImage: `linear-gradient(180deg, #0b0f17 0%, #0b0f17aa 45%, #0b0f17ee 100%), url(https://ddragon.leagueoflegends.com/cdn/img/champion/loading/${c.id}_0.jpg)` } : undefined}
    >
      <div className="ptags">{tags.map((t) => <span key={t}>{t}</span>)}</div>
      <div className="prank">
        <b className={`t-${tier.toLowerCase()}`}>{tier ? `${tier[0]}${tier.slice(1).toLowerCase()} ${TIERS.indexOf(tier) >= 7 ? "" : q!.division}` : info ? "Non classé" : ""}</b>
        {q && q.wins + q.losses > 0 && <small>{q.wins}V - {q.losses}D</small>}
      </div>
      <div className="pbottom">
        <b>{info?.name ?? s.name ?? (s.puuid ? "…" : "Joueur masqué")}</b>
        <small>{c ? c.name : "Pas encore choisi"}{s.position ? `, ${POS[s.position] ?? s.position}` : ""}</small>
        {info && g.length > 0 && (
          <div className="pnums">
            <span className={wins / g.length >= 0.5 ? "good" : "bad"}>{Math.round((wins / g.length) * 100)}% récent</span>
            {onChamp.length > 0 && (
              <span>
                {Math.round((champWins / onChamp.length) * 100)}% en {onChamp.length} parties, {(onChamp.reduce((a, x) => a + kda(x.participants[0].stats), 0) / onChamp.length).toFixed(1)} KDA
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function Damage({ team, d, label }: { team: Slot[]; d: DD; label: string }) {
  let ad = 0, ap = 0;
  for (const s of team) {
    const i = d.champs[s.championId]?.info;
    if (!i) continue;
    ad += i.attack / Math.max(1, i.attack + i.magic);
    ap += i.magic / Math.max(1, i.attack + i.magic);
  }
  const tot = ad + ap;
  if (!tot) return null;
  return (
    <div className="dmg">
      <small>{label}</small>
      <div className="dbar">
        <span className="ad" style={{ flex: ad }}>{Math.round((ad / tot) * 100)}% AD</span>
        <span className="ap" style={{ flex: ap }}>{Math.round((ap / tot) * 100)}% AP</span>
      </div>
    </div>
  );
}

export default function Live({ phase, go }: { phase: string; go: Go }) {
  const [d, setD] = useState<DD | null>(null);
  const [teams, setTeams] = useState<[Slot[], Slot[]]>([[], []]);
  const [mine, setMine] = useState<{ champ: number; role: string }>({ champ: 0, role: "NONE" });
  const [aram, setAram] = useState(false);

  useEffect(() => {
    ddragon().then(setD);
    const load = async () => {
      if (phase === "ChampSelect") {
        const s = await lcu<any>("/lol-champ-select/v1/session").catch(() => null);
        if (!s) return;
        lcu<any>("/lol-gameflow/v1/session").then((f) => setAram((f?.map?.id ?? f?.gameData?.queue?.mapId) === 12), () => {});
        const slot = (m: any): Slot => ({
          puuid: m.puuid ?? "", championId: m.championId || m.championPickIntent, position: m.assignedPosition, name: m.gameName, me: m.cellId === s.localPlayerCellId,
        });
        const t: [Slot[], Slot[]] = [s.myTeam.map(slot), s.theirTeam.map(slot)];
        setTeams(t);
        const self = t[0].find((x) => x.me);
        if (self?.championId) setMine({ champ: self.championId, role: POSITIONS[self.position ?? ""] ?? "NONE" });
      } else if (LIVE.includes(phase)) {
        const [s, self] = await Promise.all([lcu<any>("/lol-gameflow/v1/session").catch(() => null), lcu<any>("/lol-summoner/v1/current-summoner").catch(() => null)]);
        const slot = (m: any): Slot => ({ puuid: m.puuid ?? "", championId: m.championId, position: m.selectedPosition?.toLowerCase(), me: m.puuid === self?.puuid });
        if (s) {
          const a: Slot[] = s.gameData.teamOne.map(slot), b: Slot[] = s.gameData.teamTwo.map(slot);
          setTeams(b.some((x) => x.me) ? [b, a] : [a, b]);
        }
      }
    };
    load();
    const id = setInterval(load, 3000);
    return () => clearInterval(id);
  }, [phase]);

  if (!LIVE.includes(phase))
    return (
      <div className="liveempty">
        <span className="pulse" />
        <h1>En attente d'une partie</h1>
        <p className="muted">Lance une file dans le client League : Ethra prend le relais dès la sélection des champions.</p>
        <div className="ltiles">
          <div><b>Build importé</b><small>Runes, sorts et item set méta posés automatiquement quand tu verrouilles.</small></div>
          <div><b>Les 10 joueurs</b><small>Rang, winrate récent, mains et séries de chaque joueur.</small></div>
          <div><b>Overlays en jeu</b><small>Objectifs, ennemis morts, prochain objet et ordre des compétences.</small></div>
        </div>
        <div className="lbtns">
          <button className="primary" onClick={() => go({ page: "tierlist" })}>Voir la tier list</button>
          <button className="ghostbtn" onClick={() => go({ page: "overlays" })}>Configurer les overlays</button>
        </div>
      </div>
    );
  if (!d) return null;

  return (
    <div className="live">
      <div className="pagehead">
        <h1>{phase === "ChampSelect" ? "Sélection des champions" : "Partie en cours"}</h1>
      </div>
      {phase === "ChampSelect" && (
        mine.champ ? (
          <section className="card"><Builds key={`${mine.champ}-${mine.role}-${aram}`} champ={mine.champ} role={mine.role} mode={aram ? "aram" : "ranked"} compact go={go} /></section>
        ) : (
          <section className="card muted">Survole ou choisis un champion pour voir son build ici.</section>
        )
      )}
      <div className="dmgs">
        <Damage team={teams[0]} d={d} label="Dégâts de ton équipe (estimés)" />
        {teams[1].some((x) => x.championId) && <Damage team={teams[1]} d={d} label="Dégâts adverses (estimés)" />}
      </div>
      {teams.map((t, i) =>
        t.length ? (
          <div key={i}>
            <h3 className={i ? "bad" : "good"}>{i ? "Équipe adverse" : "Ton équipe"}</h3>
            <div className="prow">{t.map((s, j) => <PlayerCard key={s.puuid || `${i}-${j}`} s={s} d={d} go={go} />)}</div>
          </div>
        ) : null,
      )}
    </div>
  );
}
