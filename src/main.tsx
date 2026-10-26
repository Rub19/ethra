import { createRoot } from "react-dom/client";
import { useEffect, useRef, useState } from "react";
import "./style.css";
import { DD, ddragon, findPlayer, img, lcu, me, settings, Summoner } from "./lcu";
import { applyBuild, bestBuild, POSITIONS } from "./buildData";
import Profile from "./Profile";
import Live from "./Live";
import Builds from "./Builds";
import Champions from "./Champions";
import Settings from "./Settings";
import TierList from "./TierList";
import Match from "./Match";

export type Route =
  | { page: "profil"; puuid?: string }
  | { page: "champions" }
  | { page: "tierlist" }
  | { page: "match"; gameId: number; puuid: string }
  | { page: "champion"; champ: number; role?: string }
  | { page: "partie" }
  | { page: "overlays" }
  | { page: "reglages" };
export type Go = (r: Route) => void;

const LIVE = ["ChampSelect", "InProgress", "Reconnect", "GameStart"];

function useRouter() {
  const [stack, setStack] = useState<{ list: Route[]; i: number }>({ list: [{ page: "profil" }], i: 0 });
  const go: Go = (r) =>
    setStack((s) => (JSON.stringify(s.list[s.i]) === JSON.stringify(r) ? s : { list: [...s.list.slice(0, s.i + 1), r], i: s.i + 1 }));
  const back = () => setStack((s) => ({ ...s, i: Math.max(0, s.i - 1) }));
  const fwd = () => setStack((s) => ({ ...s, i: Math.min(s.list.length - 1, s.i + 1) }));
  useEffect(() => {
    const onMouse = (e: MouseEvent) => (e.button === 3 ? back() : e.button === 4 ? fwd() : null);
    window.addEventListener("mouseup", onMouse);
    return () => window.removeEventListener("mouseup", onMouse);
  }, []);
  return { route: stack.list[stack.i], go, back, fwd, canBack: stack.i > 0, canFwd: stack.i < stack.list.length - 1 };
}

function recent(): { puuid: string; name: string }[] {
  try {
    return JSON.parse(localStorage.getItem("rw-recent") || "[]");
  } catch {
    return [];
  }
}

function Search({ d, go, toast }: { d: DD | null; go: Go; toast: (s: string) => void }) {
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
  const champs = d && t ? Object.entries(d.champs).filter(([, c]) => c.name.toLowerCase().includes(t)).slice(0, 6) : [];
  const players = recent().filter((p) => !t || p.name.toLowerCase().includes(t)).slice(0, 4);
  type Opt = { key: string; label: string; sub: string; icon?: string; run: () => void };
  const opts: Opt[] = [
    ...(q.includes("#") ? [{ key: "find", label: q.trim(), sub: "Chercher ce joueur", run: () => lookup(q.trim()) }] : []),
    ...champs.map(([id, c]) => ({ key: `c${id}`, label: c.name, sub: "Champion · build méta", icon: d ? img.champ(d, +id) : undefined, run: () => go({ page: "champion", champ: +id }) })),
    ...players.map((p) => ({ key: p.puuid, label: p.name, sub: "Joueur récent", run: () => go({ page: "profil", puuid: p.puuid }) })),
  ];

  function lookup(name: string) {
    findPlayer(name).then((puuid) => {
      try {
        localStorage.setItem("rw-recent", JSON.stringify([{ puuid, name }, ...recent().filter((p) => p.puuid !== puuid)].slice(0, 8)));
      } catch {}
      go({ page: "profil", puuid });
    }, () => toast(`Joueur introuvable : ${name}`));
  }

  const pick = (o?: Opt) => {
    if (!o) return;
    o.run();
    setQ("");
    setOpen(false);
    ref.current?.blur();
  };

  return (
    <div className="search">
      <span className="sicon">⌕</span>
      <input
        ref={ref}
        value={q}
        placeholder="Rechercher un joueur (Nom#TAG) ou un champion"
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
      {open && opts.length > 0 && (
        <div className="sresults">
          {opts.map((o, i) => (
            <button key={o.key} className={i === sel ? "on" : ""} onMouseEnter={() => setSel(i)} onMouseDown={() => pick(o)}>
              {o.icon ? <img src={o.icon} /> : <span className="ph">{o.label[0]}</span>}
              <b>{o.label}</b>
              <small>{o.sub}</small>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function App() {
  const { route, go, back, fwd, canBack, canFwd } = useRouter();
  const [phase, setPhase] = useState<string | null>(null);
  const [self, setSelf] = useState<Summoner | null>(null);
  const [d, setD] = useState<DD | null>(null);
  const [toast, setToast] = useState("");
  const imported = useRef(0);

  useEffect(() => {
    ddragon().then(setD, () => {});
  }, []);

  useEffect(() => {
    const tick = async () => {
      const p = await lcu<string>("/lol-gameflow/v1/gameflow-phase").catch(() => null);
      setPhase((old) => {
        if (p === "ChampSelect" && old !== "ChampSelect") go({ page: "partie" });
        return p;
      });
      if (p !== null && !self) me().then(setSelf, () => {});
      const s = settings();
      if (p === "ReadyCheck" && s.autoAccept) lcu("/lol-matchmaking/v1/ready-check/accept", "POST").catch(() => {});
      if (p !== "ChampSelect") imported.current = 0;
      if (p !== "ChampSelect" || !(s.autoRunes || s.autoSpells || s.autoItems)) return;
      const champ = await lcu<number>("/lol-champ-select/v1/current-champion").catch(() => 0);
      if (!champ || champ === imported.current) return;
      imported.current = champ;
      try {
        const dd = await ddragon();
        const sess = await lcu<any>("/lol-champ-select/v1/session");
        const mine = sess.myTeam.find((m: any) => m.cellId === sess.localPlayerCellId);
        const flow = await lcu<any>("/lol-gameflow/v1/session").catch(() => null);
        const map = flow?.map?.id ?? flow?.gameData?.queue?.mapId ?? 11;
        const b = await bestBuild(dd, champ, POSITIONS[mine?.assignedPosition] ?? "NONE", map, s.source);
        if (!b) return setToast(`Pas de build trouvé pour ${dd.champs[champ]?.name}`);
        const done = await applyBuild(dd, champ, b, { runes: s.autoRunes, spells: s.autoSpells, items: s.autoItems });
        setToast(`${dd.champs[champ]?.name} : ${done.join(", ")} importés (${b.source === "toi" ? `tes ${b.games} parties` : b.source === "opgg" ? "méta OP.GG" : "reco Riot"})`);
      } catch (e) {
        setToast(`Import impossible : ${e}`);
      }
    };
    tick();
    const id = setInterval(tick, 2000);
    return () => clearInterval(id);
  }, [self]);

  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(id);
  }, [toast]);

  const inGame = !!phase && LIVE.includes(phase);
  const tabs: [Route["page"], string][] = [["profil", "Profil"], ["tierlist", "Tier list"], ["champions", "Champions"], ["partie", "Partie en cours"], ["overlays", "Overlays"], ["reglages", "Réglages"]];
  const active = route.page === "champion" ? "champions" : route.page === "profil" && route.puuid && route.puuid !== self?.puuid ? "" : route.page;
  const needsClient = phase === null && !["reglages", "overlays", "champions", "champion", "tierlist"].includes(route.page);

  return (
    <div className="app">
      <header>
        <div className="top">
          <div className="brand">ETHRA</div>
          <div className="hist">
            <button disabled={!canBack} onClick={back} title="Retour">‹</button>
            <button disabled={!canFwd} onClick={fwd} title="Suivant">›</button>
          </div>
          <Search d={d} go={go} toast={setToast} />
          <div className={`client ${phase === null ? "off" : ""}`} title={phase === null ? "Client LoL fermé" : "Client LoL connecté"}>
            <i />
            {self && d ? (
              <button className="mechip" onClick={() => go({ page: "profil" })}>
                <img src={img.icon(d, self.profileIconId)} />
                {self.gameName}
                <span>#{self.tagLine}</span>
              </button>
            ) : (
              <span>{phase === null ? "Client fermé" : "Connexion…"}</span>
            )}
          </div>
        </div>
        <nav>
          <span className="game">League of Legends</span>
          {tabs.map(([k, l]) => (
            <button key={k} className={active === k ? "on" : ""} onClick={() => go({ page: k } as Route)}>
              {l}
              {k === "partie" && inGame && <i className="dot" />}
            </button>
          ))}
        </nav>
      </header>
      <main>
        {needsClient ? (
          <div className="empty">
            <h2>Lance le client League of Legends</h2>
            <p>Ethra lit tes données directement depuis le client, aucune clé ni compte requis.</p>
            <button className="primary" onClick={() => go({ page: "champions" })}>Voir les champions en attendant</button>
          </div>
        ) : route.page === "profil" ? (
          <Profile key={route.puuid ?? "me"} puuid={route.puuid ?? null} go={go} />
        ) : route.page === "match" ? (
          <Match key={route.gameId} gameId={route.gameId} puuid={route.puuid} go={go} />
        ) : route.page === "tierlist" ? (
          <TierList go={go} />
        ) : route.page === "champions" ? (
          <Champions go={go} />
        ) : route.page === "champion" ? (
          <Builds key={`${route.champ}-${route.role}`} champ={route.champ} role={route.role} go={go} />
        ) : route.page === "partie" ? (
          <Live phase={phase ?? ""} go={go} />
        ) : (
          <Settings only={route.page === "overlays" ? "overlay" : "general"} />
        )}
      </main>
      {inGame && route.page !== "partie" && (
        <button className="livebanner" onClick={() => go({ page: "partie" })}>
          <i className="dot" />
          <div>
            <b>{phase === "ChampSelect" ? "Sélection des champions" : "Partie en cours"}</b>
            <small>Voir les équipes, rangs et builds</small>
          </div>
          <span className="primary">Ouvrir</span>
        </button>
      )}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
