import { useEffect, useState } from "react";
import { ago, DD, ddragon, img, me, mmss, Queue, rankLabel, ranked, Summoner, tftHistory } from "./lcu";
import { parseTftGames, TftGame, tftQueue, tftSummary } from "./history";
import Skeleton from "./Skeleton";
import { tftData, TftData } from "./tftApi";

const PAGE = 20;

function GameRow({ g, t }: { g: TftGame; t: TftData | null }) {
  const units = [...g.units].sort((a, b) => b.rarity - a.rarity || b.tier - a.tier);
  const traits = [...g.traits].sort((a, b) => b.style - a.style || b.num_units - a.num_units).slice(0, 6);
  return (
    <div className={`tgame ${g.placement <= 4 ? "top" : "low"}`}>
      <b className="tplace">{g.placement}<small>/8</small></b>
      <div className="tmeta">
        <b>{tftQueue(g.queueId)}</b>
        <small>{mmss(g.seconds)}, {ago(g.at)}</small>
        <small>Niveau {g.level}</small>
      </div>
      <div className="tgunits">
        {units.map((u, i) => {
          const info = t?.units[u.characterId];
          return (
            <span key={`${u.characterId}-${i}`} className={`tgunit c${info?.cost ?? 0}`} title={info?.name ?? u.characterId}>
              <small>{"★".repeat(Math.max(1, u.tier))}</small>
              {info?.icon ? <img src={info.icon} alt={info.name} /> : <i>{(info?.name ?? u.characterId.replace(/^TFT\d*_/, ""))[0]}</i>}
            </span>
          );
        })}
      </div>
      <div className="tgtraits">
        {traits.map((x) => <span key={x.name} title={t?.traits[x.name]?.name}>{t?.traits[x.name]?.icon && <img src={t.traits[x.name].icon} alt="" />}{x.num_units} {t?.traits[x.name]?.name ?? x.name.replace(/^TFT\d*_|^Set\d*_/, "")}</span>)}
      </div>
    </div>
  );
}

export default function TftProfile() {
  const [sum, setSum] = useState<Summoner | null>(null);
  const [d, setD] = useState<DD | null>(null);
  const [rank, setRank] = useState<Queue | undefined>();
  const [games, setGames] = useState<TftGame[] | null>(null);
  const [t, setT] = useState<TftData | null>(null);
  const [err, setErr] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [more, setMore] = useState<"idle" | "loading" | "end">("idle");

  useEffect(() => {
    setErr("");
    setGames(null);
    setMore("idle");
    tftData().then(setT, () => {});
    (async () => {
      const s = await me();
      setSum(s);
      const [q, raw, dd] = await Promise.all([ranked(s.puuid).catch(() => ({} as Record<string, Queue>)), tftHistory(s.puuid, 0, PAGE), ddragon()]);
      setRank(q.RANKED_TFT);
      setD(dd);
      const list = parseTftGames(raw, s.puuid);
      setGames(list);
      if (list.length < PAGE) setMore("end");
    })().catch((e) => setErr(String(e)));
  }, [attempt]);

  const loadMore = () => {
    if (!sum || !games || more !== "idle") return;
    setMore("loading");
    tftHistory(sum.puuid, games.length, PAGE).then(
      (raw) => {
        const next = parseTftGames(raw, sum.puuid).filter((g) => !games.some((x) => x.id === g.id));
        setGames([...games, ...next]);
        setMore(next.length === 0 ? "end" : "idle");
      },
      () => setMore("idle"),
    );
  };

  if (err) return <div className="empty"><h2>Historique TFT indisponible</h2><p>{err}</p><button className="primary" onClick={() => setAttempt((x) => x + 1)}>Réessayer</button></div>;
  if (!sum || !games || !d) return <div className="tierlist"><div className="pagehead"><h1>Profil TFT</h1></div><Skeleton rows={6} /></div>;

  const s = tftSummary(games);
  const placed = rank?.tier && rank.tier !== "NONE" && rank.tier !== "UNRANKED";
  return (
    <div className="profile">
      <div className="phero tfthero">
        <div className="avatar"><img src={img.icon(d, sum.profileIconId)} alt="" /><span>{sum.summonerLevel}</span></div>
        <div className="pid">
          <h1>{sum.gameName} <span>#{sum.tagLine}</span></h1>
          {s.games > 0 && (
            <div className="pquick">
              <div><b className={s.average <= 4.5 ? "good" : "bad"}>{s.average.toFixed(2)}</b><small>Place moyenne</small></div>
              <div><b className={s.top4 >= 0.5 ? "good" : "bad"}>{Math.round(s.top4 * 100)}%</b><small>Top 4</small></div>
              <div><b>{s.wins}</b><small>1res places</small></div>
              <div><b>{s.games}</b><small>Parties analysées</small></div>
            </div>
          )}
        </div>
      </div>
      <div className="cols tftcols">
        <aside>
          <section className="card">
            <div className="rank">
              {placed ? <img className="emblem" src={img.emblem(rank!.tier)} alt="" /> : <div className="emblem none" />}
              <div>
                <span>Classé TFT</span>
                <b className={placed ? `t-${rank!.tier.toLowerCase()}` : ""}>{rankLabel(rank)}</b>
                {rank && rank.wins + rank.losses > 0 && <small>{rank.wins} top 1, {rank.wins + rank.losses} parties</small>}
              </div>
            </div>
          </section>
        </aside>
        <div className="list">
          {games.length === 0 ? <div className="empty"><h2>Aucune partie TFT</h2><p>Tes parties récentes s'afficheront ici.</p></div> : games.map((g) => <GameRow key={g.id} g={g} t={t} />)}
          {games.length > 0 && more !== "end" && <button className="ghostbtn vmore" onClick={loadMore} disabled={more === "loading"}>{more === "loading" ? "Chargement…" : "Voir plus de parties"}</button>}
        </div>
      </div>
    </div>
  );
}
