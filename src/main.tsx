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
import AccountMenu from "./Account";
import Overlays from "./Overlays";
import { ValoAgents, ValoLive, ValoMatch, ValoProfile, ValoSearch } from "./Valo";
import { VPhase, vPhase, vSession } from "./valoApi";
import GamePicker, { GameKey } from "./GamePicker";
import TftDecks from "./Tft";
import UpdateCorner from "./Updater";
export type Route =
  | { page: "v-profil"; puuid?: string }
  | { page: "v-match"; id: string; puuid: string }
  | { page: "v-partie" }
  | { page: "v-agents" }
  | { page: "t-decks" }
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
    ...champs.map(([id, c]) => ({ key: `c${id}`, label: c.name, sub: "Build méta du champion", icon: d ? img.champ(d, +id) : undefined, run: () => go({ page: "champion", champ: +id }) })),
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
      <svg className="sicon" width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" strokeWidth="1.7" /><path d="m11 11 3.5 3.5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" /></svg>
      <input
        ref={ref}
        value={q}
        aria-label="Rechercher un joueur ou un champion"
        placeholder="Joueur#TAG ou champion"
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
  const [game, setGame] = useState<GameKey>(() => {
    try {
      const g = localStorage.getItem("rw-game");
      return g === "valo" || g === "tft" ? g : "lol";
    } catch {
      return "lol";
    }
  });
  const [vSelf, setVSelf] = useState<string | null>(null);
  const [vph, setVph] = useState<VPhase>(null);

  const homeOf = (g: GameKey): Route => ({ page: g === "valo" ? "v-profil" : g === "tft" ? "t-decks" : "profil" });
  const savedRoute = (g: GameKey): Route | null => {
    try {
      const r = JSON.parse(localStorage.getItem(`rw-route-${g}`) || "null");
      return r?.page ? r : null;
    } catch {
      return null;
    }
  };
  const switchGame = (g: GameKey) => {
    if (g === game) return;
    setGame(g);
    try {
      localStorage.setItem("rw-game", g);
    } catch {}
    go(savedRoute(g) ?? homeOf(g));
  };

  useEffect(() => {
    const owner: GameKey | null = route.page.startsWith("v-") ? "valo" : route.page.startsWith("t-") ? "tft" : route.page === "reglages" ? null : "lol";
    if (owner !== game) return;
    try {
      localStorage.setItem(`rw-route-${game}`, JSON.stringify(route));
    } catch {}
  }, [route, game]);

  useEffect(() => {
    const onToast = (e: Event) => setToast((e as CustomEvent<string>).detail);
    window.addEventListener("rw-toast", onToast);
    return () => window.removeEventListener("rw-toast", onToast);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && ["1", "2", "3"].includes(e.key)) {
        e.preventDefault();
        switchGame((["lol", "valo", "tft"] as GameKey[])[+e.key - 1]);
      }
      if (e.altKey && e.key === "ArrowLeft") (e.preventDefault(), back());
      if (e.altKey && e.key === "ArrowRight") (e.preventDefault(), fwd());
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  useEffect(() => {
    if (game === "valo" && route.page === "profil" && !route.puuid) go({ page: "v-profil" });
    if (game === "tft" && route.page === "profil" && !route.puuid) go({ page: "t-decks" });
  }, []);

  useEffect(() => {
    document.documentElement.dataset.game = game;
  }, [game]);

  useEffect(() => {
    ddragon().then(setD, () => {});
  }, []);

  useEffect(() => {
    if (game !== "valo") return;
    const tick = async () => {
      const s = await vSession().catch(() => null);
      setVSelf(s?.puuid || null);
      const p = s?.puuid ? await vPhase(s.puuid).catch(() => "MENUS" as VPhase) : null;
      setVph((old) => {
        if (p === "PREGAME" && old !== "PREGAME" && settings().autoOpenLive) go({ page: "v-partie" });
        return p;
      });
    };
    tick();
    const id = setInterval(tick, 3000);
    return () => clearInterval(id);
  }, [game]);

  useEffect(() => {
    const tick = async () => {
      const p = await lcu<string>("/lol-gameflow/v1/gameflow-phase").catch(() => null);
      setPhase((old) => {
        if (p === "ChampSelect" && old !== "ChampSelect" && settings().autoOpenLive) go({ page: "partie" });
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
        const done = await applyBuild(dd, champ, b, { runes: s.autoRunes, spells: s.autoSpells, items: s.autoItems, role: POSITIONS[mine?.assignedPosition] ?? "", aram: map === 12 });
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

  const valoGame = game === "valo";
  const tftGame = game === "tft";
  const inGame = valoGame ? vph === "PREGAME" || vph === "INGAME" : !!phase && LIVE.includes(phase);
  const tabs: [Route["page"], string][] = tftGame
    ? [["t-decks", "Decks méta"], ["reglages", "Réglages"]]
    : valoGame
    ? [["v-profil", "Profil"], ["v-agents", "Agents"], ["v-partie", "Partie en cours"], ["reglages", "Réglages"]]
    : [["profil", "Profil"], ["tierlist", "Tier list"], ["champions", "Champions"], ["partie", "Partie en cours"], ["overlays", "Overlays"], ["reglages", "Réglages"]];
  const active =
    route.page === "champion" ? "champions"
    : (route.page === "profil" && route.puuid && route.puuid !== self?.puuid) || (route.page === "v-profil" && route.puuid && route.puuid !== vSelf) ? ""
    : route.page;
  const needsClient = phase === null && !["reglages", "overlays", "champions", "champion", "tierlist", "v-profil", "v-match", "v-partie"].includes(route.page);
  const needsValo = !vSelf && ["v-profil", "v-match", "v-partie"].includes(route.page);
  const connected = tftGame ? true : valoGame ? !!vSelf : phase !== null;
  const liveRoute = valoGame ? "v-partie" : "partie";

  return (
    <div className="app">
      <header>
        <div className="bar">
          <GamePicker game={game} onPick={switchGame} />
          <div className="brand"><svg width="22" height="22" viewBox="0 0 32 32" aria-hidden="true"><path d="M25.3 14.5A9.4 9.4 0 1 1 17.5 6.7" fill="none" stroke="#2fd3b5" strokeWidth="3.6" strokeLinecap="round" /><circle cx="22.6" cy="9.4" r="2.2" fill="#f5f7fc" /></svg>Kairo</div>
          <div className="hist">
            <button disabled={!canBack} onClick={back} aria-label="Retour"><svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M9 2.5 4.5 7 9 11.5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg></button>
            <button disabled={!canFwd} onClick={fwd} aria-label="Suivant"><svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"><path d="M5 2.5 9.5 7 5 11.5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" /></svg></button>
          </div>
          <nav aria-label="Pages">
            {tabs.map(([k, l]) => (
              <button key={k} className={active === k ? "on" : ""} aria-current={active === k ? "page" : undefined} onClick={() => go({ page: k } as Route)}>
                {l}
                {k === liveRoute && inGame && <i className="dot" />}
              </button>
            ))}
          </nav>
          {tftGame ? <div className="bar-spacer" /> : valoGame ? <ValoSearch go={go} /> : <Search d={d} go={go} toast={setToast} />}
          <div className={`client ${connected ? "" : "off"}`} title={connected ? "Client connecté" : "Client fermé"}>
            {!valoGame && self && d ? (
              <AccountMenu self={self} d={d} go={go} />
            ) : (
              <span><i />{tftGame ? "Méta OP.GG" : valoGame ? (vSelf ? "VALORANT connecté" : "VALORANT fermé") : phase === null ? "Client fermé" : "Connexion…"}</span>
            )}
          </div>
        </div>
      </header>
      <main>
        {route.page === "t-decks" ? (
          <TftDecks />
        ) : route.page === "v-agents" ? (
          <ValoAgents />
        ) : needsValo ? (
          <>
            <div className="notice">
              <i />
              <div>
                <b>VALORANT est fermé</b>
                <small>Lance le jeu pour voir ton profil, tes parties et les rangs en direct. En attendant, voici la méta des agents.</small>
              </div>
            </div>
            <ValoAgents />
          </>
        ) : route.page === "v-profil" ? (
          <ValoProfile key={route.puuid ?? "me"} puuid={route.puuid} self={vSelf!} go={go} />
        ) : route.page === "v-match" ? (
          <ValoMatch key={route.id} id={route.id} puuid={route.puuid} go={go} />
        ) : route.page === "v-partie" ? (
          <ValoLive phase={vph} self={vSelf!} go={go} />
        ) : needsClient ? (
          <>
            <div className="notice">
              <i />
              <div>
                <b>Le client League of Legends est fermé</b>
                <small>Lance-le pour voir ton profil, ton historique et ta partie en cours. En attendant, voici la tier list du patch.</small>
              </div>
            </div>
            <TierList go={go} />
          </>
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
          route.page === "overlays" ? <Overlays /> : <Settings only="general" />
        )}
      </main>
      {inGame && route.page !== liveRoute && (
        <button className="livebanner" onClick={() => go({ page: liveRoute })}>
          <i className="dot" />
          <div>
            <b>{valoGame ? (vph === "PREGAME" ? "Sélection des agents" : "Partie en cours") : phase === "ChampSelect" ? "Sélection des champions" : "Partie en cours"}</b>
            <small>{valoGame ? "Voir les rangs des joueurs" : "Voir les équipes, rangs et builds"}</small>
          </div>
          <span className="primary">Ouvrir</span>
        </button>
      )}
      {toast && <div className="toast">{toast}</div>}
      <UpdateCorner />
    </div>
  );
}

document.addEventListener("contextmenu", (e) => {
  if (!(e.target as HTMLElement).closest("input, textarea")) e.preventDefault();
});

createRoot(document.getElementById("root")!).render(<App />);
