import { useEffect, useState } from "react";
import { applyBuild, Build, myBuild, pageName, riotBuild } from "./buildData";
import { DD, ddragon, img, lcu, settings, Settings as S } from "./lcu";
import { OPGG_POS, opggBuild, OpggBuild, POS_LABEL, TIER_LABEL, Counter, mainPosition, Opt, RANK_FILTERS } from "./opgg";
import type { Go } from "./main";
import Skeleton from "./Skeleton";
import JunglePath from "./Jungle";

type Src = "opgg" | "toi" | "riot";

function Auto({ k, label = "Import auto" }: { k: "autoRunes" | "autoSpells" | "autoItems"; label?: string }) {
  const [on, setOn] = useState(() => settings()[k]);
  const flip = () => {
    const next: S = { ...settings(), [k]: !on };
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
    setOn(!on);
  };
  return (
    <button className={`auto ${on ? "on" : ""}`} onClick={flip} role="switch" aria-checked={on}>
      <span>{label}</span>
      <i />
    </button>
  );
}
function OptRow({ label, opts, d, arrows = false }: { label: string; opts: Opt[]; d: DD; arrows?: boolean }) {
  return (
    <div className="iopt">
      <span className="iolabel">{label}</span>
      <div className="iolist">
        {opts.map((o, i) => (
          <div key={i} className={`iochoice ${i === 0 ? "best" : ""}`}>
            <div className="row">
              {o.ids.map((id, k) => (
                <span key={`${id}-${k}`} className="step">
                  {arrows && k > 0 && "›"}
                  <img className="ic" src={img.item(d, id)} title={d.items[id]?.name} alt={d.items[id]?.name ?? ""} />
                </span>
              ))}
            </div>
            <div className="iostat">
              <b className={o.win >= 0.5 ? "good" : "bad"}>{pct(o.win)}</b>
              <small>{pct(o.pick)} · {o.play.toLocaleString("fr")}</small>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
const RIOT_POS: Record<string, string> = { top: "TOP", jungle: "JUNGLE", mid: "MIDDLE", adc: "BOTTOM", support: "UTILITY" };
const pct = (x: number) => `${(x * 100).toFixed(1)}%`;

export default function Builds({ champ, go, role: initialRole = "", compact = false, mode: initialMode = "ranked" }: { champ: number; go?: Go; role?: string; compact?: boolean; mode?: "ranked" | "aram" }) {
  const [mode, setMode] = useState(initialMode);
  const [d, setD] = useState<DD | null>(null);
  const [role, setRole] = useState(OPGG_POS[initialRole] ?? (POS_LABEL[initialRole] ? initialRole : ""));
  const [src, setSrc] = useState<Src>("opgg");
  const [b, setB] = useState<Build | null | undefined>();
  const [msg, setMsg] = useState("");
  const [rank, setRank] = useState(() => settings().rankFilter);
  const [sb, setSb] = useState(0);
  const pickRank = (r: string) => {
    setRank(r);
    try {
      localStorage.setItem("rw-settings", JSON.stringify({ ...settings(), rankFilter: r }));
    } catch {}
  };

  useEffect(() => {
    ddragon().then(setD);
  }, []);

  useEffect(() => {
    if (!d || !champ) return;
    setB(undefined);
    setMsg("");
    const load =
      src === "opgg" && mode === "aram" ? opggBuild(d, champ, "mid", "aram")
      : src === "opgg" ? (role ? Promise.resolve(role) : mainPosition(d, champ).then((p) => (setRole(p), p))).then((p) => opggBuild(d, champ, p, "ranked", rank))
      : src === "toi" ? myBuild(d, champ, mode === "aram" ? 12 : 11)
      : riotBuild(champ, mode === "aram" ? "NONE" : RIOT_POS[role] ?? "NONE", mode === "aram" ? 12 : 11);
    load.then(
      (x) => (x === null && src === "opgg" ? setSrc("toi") : x === null && src === "toi" ? setSrc("riot") : setB(x)),
      () => (src === "opgg" ? setSrc("toi") : src === "toi" ? setSrc("riot") : setB(null)),
    );
    setSb(0);
  }, [d, champ, src, role, mode, rank]);

  if (!d) return null;

  const apply = async () => {
    if (!b) return;
    const inCs = await lcu<string>("/lol-gameflow/v1/gameflow-phase").catch(() => "");
    applyBuild(d, champ, b, { runes: true, spells: inCs === "ChampSelect", items: true, role, aram: mode === "aram" }).then(
      (done) => setMsg(`Importé : ${done.join(", ")}`),
      (e) => setMsg(`Erreur : ${e}`),
    );
  };
  const c = d.champs[champ];
  const o = b?.source === "opgg" ? (b as OpggBuild) : null;
  const items = (ids: number[], arrows = false) => (
    <div className="row">
      {ids.map((id, i) => (
        <span key={`${id}-${i}`} className="step">
          {arrows && i > 0 && "›"}
          <img className="ic" src={img.item(d, id)} title={d.items[id]?.name} />
        </span>
      ))}
    </div>
  );
  const counters = (list: Counter[], good: boolean) => (
    <div className="counters">
      {list.map((k) => (
        <button key={k.champ} onClick={() => go?.({ page: "champion", champ: k.champ, role })} title={`${k.play} parties`}>
          <img src={img.champ(d, k.champ)} />
          <b className={good ? "good" : "bad"}>{pct(k.myWin)}</b>
          <small>{d.champs[k.champ]?.name ?? k.name}</small>
        </button>
      ))}
    </div>
  );

  return (
    <div className="builds">
      {!compact && (
        <div className="chead" style={{ backgroundImage: `url(https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${c?.id}_0.jpg)` }}>
          <img src={img.champ(d, champ)} />
          <div>
            <h1>{c?.name}</h1>
            <small>{c?.tags.join(" · ")}</small>
          </div>
          {o && (
            <div className="cstats">
              <div><span className={`tier t${o.tier}`}>{TIER_LABEL[o.tier] ?? o.tier}</span><small>Tier</small></div>
              <div><b className={o.winRate >= 0.5 ? "good" : "bad"}>{pct(o.winRate)}</b><small>Winrate</small></div>
              <div><b>{pct(o.pickRate)}</b><small>Pick</small></div>
              <div><b>{pct(o.banRate)}</b><small>Ban</small></div>
              <div><b>{o.games.toLocaleString("fr")}</b><small>Parties</small></div>
            </div>
          )}
          {go && <button className="ghostbtn" onClick={() => go({ page: "tierlist" })}>Tier list</button>}
        </div>
      )}
      <div className="bbar">
        {compact && (
          <button className="cmini" onClick={() => go?.({ page: "champion", champ, role })}>
            <img src={img.champ(d, champ)} />
            <b>{c?.name}</b>
          </button>
        )}
        <div className="seg">
          <button className={src === "opgg" ? "on" : ""} onClick={() => setSrc("opgg")}>Méta OP.GG</button>
          <button className={src === "toi" ? "on" : ""} onClick={() => setSrc("toi")}>Tes parties</button>
          <button className={src === "riot" ? "on" : ""} onClick={() => setSrc("riot")}>Reco Riot</button>
        </div>
        <div className="seg">
          <button className={mode === "ranked" ? "on" : ""} onClick={() => setMode("ranked")}>Faille</button>
          <button className={mode === "aram" ? "on" : ""} onClick={() => setMode("aram")}>ARAM</button>
        </div>
        {src !== "toi" && mode === "ranked" && (
          <div className="seg">
            {Object.entries(POS_LABEL).map(([k, l]) => <button key={k} className={`rolebtn ${role === k ? "on" : ""}`} onClick={() => setRole(k)} title={l}><img src={img.role(k)} />{l}</button>)}
          </div>
        )}
        {src === "opgg" && mode === "ranked" && (
          <div className="seg">
            {RANK_FILTERS.map(([k, l]) => <button key={k} className={rank === k ? "on" : ""} onClick={() => pickRank(k)}>{l}</button>)}
          </div>
        )}
        <span className="grow" />
        {msg && <span className="muted">{msg}</span>}
        <button className="primary" disabled={!b} onClick={apply}>Importer dans le client</button>
      </div>
      {b === undefined ? (
        <Skeleton rows={2} height={220} />
      ) : b === null ? (
        <div className="empty"><p>Aucun build disponible pour ce champion.</p></div>
      ) : (
        <div className="bgrid">
          <section className="card">
            <div className="shead">
              <h3>Runes</h3>
              <Auto k="autoRunes" />
            </div>
            <div className="chips">
              {o ? (
                <>
                  <span className={o.runeWin >= 0.5 ? "good" : "bad"}><b>{pct(o.runeWin)}</b> victoires</span>
                  <span><b>{pct(o.runePick)}</b> des joueurs</span>
                  <span><b>{o.runePlay.toLocaleString("fr")}</b> parties</span>
                </>
              ) : b.games > 0 ? (
                <>
                  <span className={b.wins / b.games >= 0.5 ? "good" : "bad"}><b>{Math.round((b.wins / b.games) * 100)}%</b> victoires</span>
                  <span><b>{b.games}</b> de tes parties</span>
                </>
              ) : <span>Recommandation Riot</span>}
              <span className="pname">Page « {pageName(d, champ, b, role, mode === "aram")} »</span>
            </div>
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
            {b.perks.length >= 9 && (
              <div className="shards">
                {b.perks.slice(6, 9).map((id, i) => <img key={i} src={img.rune(d, id)} title={String(id)} />)}
              </div>
            )}
          </section>
          <section className="card">
            <div className="shead">
              <h3>Sorts d'invocateur</h3>
              <Auto k="autoSpells" />
            </div>
            <div className="spellrow">
              <div className="row">{b.spells.map((s) => <img key={s} className="ic" src={img.spell(d, s)} title={d.spells[s]?.name} />)}</div>
              {o && o.spellPlay > 0 && (
                <div className="chips">
                  <span className={o.spellWin >= 0.5 ? "good" : "bad"}><b>{pct(o.spellWin)}</b> victoires</span>
                  <span><b>{pct(o.spellPick)}</b> des joueurs</span>
                  <span><b>{o.spellPlay.toLocaleString("fr")}</b> parties</span>
                </div>
              )}
            </div>
            <div className="shead sep">
              <h3>Objets</h3>
              <Auto k="autoItems" label="Item set auto" />
            </div>
            {o ? (
              <div className="iopts">
                {o.startOpt && <OptRow label="Départ" opts={[o.startOpt]} d={d} />}
                {o.coreOpt && <OptRow label="Build principal" opts={[o.coreOpt]} d={d} arrows />}
                {o.bootsOpt && <OptRow label="Bottes" opts={[o.bootsOpt]} d={d} />}
                {o.fourth.length > 0 && <OptRow label="4e objet" opts={o.fourth} d={d} />}
                {o.fifth.length > 0 && <OptRow label="5e objet" opts={o.fifth} d={d} />}
                {o.sixth.length > 0 && <OptRow label="6e objet" opts={o.sixth} d={d} />}
              </div>
            ) : (
              <>
                {b.start.length > 0 && (<><h3>Objets de départ</h3>{items(b.start)}</>)}
                {b.order.length > 0 && (<><h3>Ordre d'achat</h3>{items(b.order, true)}</>)}
                {b.core.length > 0 && (<><h3>Objets finis les plus joués</h3>{items(b.core)}</>)}
              </>
            )}
            {b.source === "riot" && <p className="muted">Les objets recommandés par Riot sont déjà dans la boutique en jeu.</p>}
          </section>
          {b.skills.some(Boolean) && (() => {
            const pick = o?.skillBuilds[sb];
            const order = pick ? pick.order.map((k) => ({ Q: 1, W: 2, E: 3, R: 4 })[k] ?? 0) : b.skills;
            return (
            <section className="card wide">
              <h3>
                Ordre des compétences{" "}
                {pick ? <small className="muted">Max {pick.max.join(" › ")}</small> : o && o.maxOrder.length > 0 && <small className="muted">Max {o.maxOrder.join(" › ")}</small>}
              </h3>
              {o && o.skillBuilds.length > 1 && (
                <div className="sbuilds">
                  {o.skillBuilds.map((x, i) => (
                    <button key={i} className={i === sb ? "on" : ""} onClick={() => setSb(i)}>
                      <b>{x.order.slice(0, 3).join(" ")}</b>
                      <span>Max {x.max.join(" › ")}</span>
                      <em className={x.win >= 0.5 ? "good" : "bad"}>{pct(x.win)}</em>
                      <small>{pct(x.pick)}</small>
                    </button>
                  ))}
                </div>
              )}
              <div className="skills">
                {["Q", "W", "E", "R"].map((k, slot) => (
                  <div key={k} className="srow" style={{ gridTemplateColumns: `32px repeat(${order.length}, 1fr)` }}>
                    <img src={img.ability(d, c.id, k)} onError={(e) => (e.currentTarget.style.visibility = "hidden")} />
                    {order.map((s, lvl) => <span key={lvl} className={s === slot + 1 ? "on" : ""}>{s === slot + 1 ? k : ""}</span>)}
                  </div>
                ))}
                <div className="srow lv" style={{ gridTemplateColumns: `32px repeat(${order.length}, 1fr)` }}><span />{order.map((_, i) => <span key={i}>{i + 1}</span>)}</div>
              </div>
            </section>
            );
          })()}
          {mode === "ranked" && role === "jungle" && <JunglePath d={d} />}
          {o && (o.weak.length > 0 || o.strong.length > 0) && (
            <section className="card wide">
              <div className="ctwo">
                <div><h3>Matchups difficiles</h3>{counters(o.weak, false)}</div>
                <div><h3>Matchups faciles</h3>{counters(o.strong, true)}</div>
              </div>
            </section>
          )}
        </div>
      )}
    </div>
  );
}
