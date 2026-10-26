import { useEffect, useState } from "react";
import { applyBuild, Build, myBuild, riotBuild } from "./buildData";
import { DD, ddragon, img, lcu } from "./lcu";
import type { Go } from "./main";

const ROLES: [string, string][] = [["NONE", "Toutes"], ["TOP", "Top"], ["JUNGLE", "Jungle"], ["MIDDLE", "Mid"], ["BOTTOM", "ADC"], ["UTILITY", "Support"]];

export default function Builds({ champ, go, role: initialRole = "NONE", compact = false }: { champ: number; go?: Go; role?: string; compact?: boolean }) {
  const [d, setD] = useState<DD | null>(null);
  const [role, setRole] = useState(initialRole);
  const [src, setSrc] = useState<"toi" | "riot">("toi");
  const [b, setB] = useState<Build | null | undefined>();
  const [msg, setMsg] = useState("");

  useEffect(() => {
    ddragon().then(setD);
  }, []);

  useEffect(() => {
    if (!d || !champ) return;
    setB(undefined);
    setMsg("");
    (src === "toi" ? myBuild(d, champ) : riotBuild(champ, role)).then(
      (x) => (x === null && src === "toi" ? setSrc("riot") : setB(x)),
      () => setB(null),
    );
  }, [d, champ, src, role]);

  if (!d) return null;

  const apply = async () => {
    if (!b) return;
    const inCs = await lcu<string>("/lol-gameflow/v1/gameflow-phase").catch(() => "");
    applyBuild(d, champ, b, { runes: true, spells: inCs === "ChampSelect", items: true }).then(
      (done) => setMsg(`Importé : ${done.join(", ")}`),
      (e) => setMsg(`Erreur : ${e}`),
    );
  };
  const c = d.champs[champ];

  return (
    <div className="builds">
      {!compact && (
        <div className="chead" style={{ backgroundImage: `url(https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${c?.id}_0.jpg)` }}>
          <img src={img.champ(d, champ)} />
          <div>
            <h1>{c?.name}</h1>
            <small>{c?.tags.join(" · ")}</small>
          </div>
          {go && <button className="ghostbtn" onClick={() => go({ page: "champions" })}>Changer de champion</button>}
        </div>
      )}
      <div className="bbar">
        {compact && (
          <button className="cmini" onClick={() => go?.({ page: "champion", champ })}>
            <img src={img.champ(d, champ)} />
            <b>{c?.name}</b>
          </button>
        )}
        <div className="seg">
          <button className={src === "toi" ? "on" : ""} onClick={() => setSrc("toi")}>Tes parties</button>
          <button className={src === "riot" ? "on" : ""} onClick={() => setSrc("riot")}>Reco Riot</button>
        </div>
        {src === "riot" && (
          <div className="seg">{ROLES.map(([k, l]) => <button key={k} className={role === k ? "on" : ""} onClick={() => setRole(k)}>{l}</button>)}</div>
        )}
        <span className="grow" />
        {msg && <span className="muted">{msg}</span>}
        <button className="primary" disabled={!b} onClick={apply}>Importer dans le client</button>
      </div>
      {b === undefined ? (
        <p className="muted">Analyse…</p>
      ) : b === null ? (
        <div className="empty">
          <p>{src === "toi" ? "Aucune partie sur ce champion dans tes 100 dernières. Passe sur la reco Riot." : "Pas de recommandation Riot pour ce champion."}</p>
        </div>
      ) : (
        <div className="bgrid">
          <section className="card">
            <h3>Runes {b.games > 0 && <small className="muted">{b.games} parties · {Math.round((b.wins / b.games) * 100)}% WR</small>}</h3>
            <div className="trees">
              {[b.primary, b.sub].map((sid, i) => {
                const st = d.styles.find((s) => s.id === sid);
                if (!st) return null;
                return (
                  <div key={sid} className="tree">
                    <img className="style" src={img.rune(d, sid)} title={st.name} />
                    {st.slots.slice(i ? 1 : 0).map((sl, k) => (
                      <div key={k} className={`slot ${k === 0 && !i ? "keys" : ""}`}>
                        {sl.runes.map((r) => <img key={r.id} src={img.rune(d, r.id)} title={r.name} className={b.perks.includes(r.id) ? "on" : ""} />)}
                      </div>
                    ))}
                  </div>
                );
              })}
            </div>
          </section>
          <section className="card">
            <h3>Sorts d'invocateur</h3>
            <div className="row">{b.spells.map((s) => <img key={s} className="ic" src={img.spell(d, s)} title={d.spells[s]?.name} />)}</div>
            {b.source === "toi" && (
              <>
                <h3>Objets de départ</h3>
                <div className="row">{b.start.map((id, i) => <img key={i} className="ic" src={img.item(d, id)} title={d.items[id]?.name} />)}</div>
                <h3>Ordre d'achat</h3>
                <div className="row">{b.order.map((id, i) => <span key={id} className="step">{i > 0 && "›"}<img className="ic" src={img.item(d, id)} title={d.items[id]?.name} /></span>)}</div>
                <h3>Objets finis les plus joués</h3>
                <div className="row">{b.core.map((id) => <img key={id} className="ic" src={img.item(d, id)} title={d.items[id]?.name} />)}</div>
              </>
            )}
            {b.source === "riot" && <p className="muted">Les objets recommandés par Riot sont déjà dans la boutique en jeu.</p>}
          </section>
          {b.skills.some(Boolean) && (
            <section className="card wide">
              <h3>Ordre des compétences</h3>
              <div className="skills">
                {["Q", "W", "E", "R"].map((k, slot) => (
                  <div key={k} className="srow">
                    <img src={img.ability(d, d.champs[champ].id, k)} onError={(e) => (e.currentTarget.style.visibility = "hidden")} />
                    {b.skills.map((s, lvl) => <span key={lvl} className={s === slot + 1 ? `on k${k}` : ""}>{s === slot + 1 ? k : ""}</span>)}
                  </div>
                ))}
                <div className="srow lv"><span />{b.skills.map((_, i) => <span key={i}>{i + 1}</span>)}</div>
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}
