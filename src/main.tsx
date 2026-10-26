import { createRoot } from "react-dom/client";
import { useEffect, useRef, useState } from "react";
import "./style.css";
import { ddragon, lcu, settings, Settings } from "./lcu";
import { applyBuild, bestBuild, POSITIONS } from "./buildData";
import Profile from "./Profile";
import Live from "./Live";
import Builds from "./Builds";

type Tab = "profil" | "partie" | "builds" | "reglages";

function App() {
  const [tab, setTab] = useState<Tab>("profil");
  const [phase, setPhase] = useState<string | null>(null);
  const [puuid, setPuuid] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [toast, setToast] = useState("");
  const [cfg, setCfg] = useState<Settings>(settings);
  const imported = useRef(0);

  useEffect(() => {
    const tick = async () => {
      const p = await lcu<string>("/lol-gameflow/v1/gameflow-phase").catch(() => null);
      setPhase((old) => {
        if (p === "ChampSelect" && old !== "ChampSelect") setTab("partie");
        return p;
      });
      const s = settings();
      if (p === "ReadyCheck" && s.autoAccept) lcu("/lol-matchmaking/v1/ready-check/accept", "POST").catch(() => {});
      if (p !== "ChampSelect") imported.current = 0;
      if (p !== "ChampSelect" || !(s.autoRunes || s.autoSpells || s.autoItems)) return;
      const champ = await lcu<number>("/lol-champ-select/v1/current-champion").catch(() => 0);
      if (!champ || champ === imported.current) return;
      imported.current = champ;
      try {
        const d = await ddragon();
        const sess = await lcu<any>("/lol-champ-select/v1/session");
        const mine = sess.myTeam.find((m: any) => m.cellId === sess.localPlayerCellId);
        const flow = await lcu<any>("/lol-gameflow/v1/session").catch(() => null);
        const map = flow?.map?.id ?? flow?.gameData?.queue?.mapId ?? 11;
        const b = await bestBuild(d, champ, POSITIONS[mine?.assignedPosition] ?? "NONE", map, s.source);
        if (!b) return setToast(`Pas de build trouvé pour ${d.champs[champ]?.name}`);
        const done = await applyBuild(d, champ, b, { runes: s.autoRunes, spells: s.autoSpells, items: s.autoItems });
        setToast(`${d.champs[champ]?.name} : ${done.join(", ")} importés (${b.source === "toi" ? `tes ${b.games} parties` : "reco Riot"})`);
      } catch (e) {
        setToast(`Import impossible : ${e}`);
      }
    };
    tick();
    const id = setInterval(tick, 2000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(id);
  }, [toast]);

  const save = (patch: Partial<Settings>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
  };

  const go = (e: React.FormEvent) => {
    e.preventDefault();
    if (!search.includes("#")) return setToast("Format : Nom#TAG");
    import("./lcu").then(({ findPlayer }) =>
      findPlayer(search).then((p) => {
        setPuuid(p);
        setTab("profil");
      }, (err) => setToast(String(err))),
    );
  };

  const closed = phase === null;
  const tabs: [Tab, string][] = [["profil", "Profil"], ["partie", "Partie en cours"], ["builds", "Builds"], ["reglages", "Réglages"]];

  return (
    <div className="app">
      <header>
        <div className="brand">ETHRA</div>
        <span className="game">League of Legends</span>
        <nav>
          {tabs.map(([k, l]) => (
            <button key={k} className={tab === k ? "on" : ""} onClick={() => (k === "profil" && setPuuid(null), setTab(k))}>
              {l}
              {k === "partie" && phase && ["ChampSelect", "InProgress"].includes(phase) && <i className="dot" />}
            </button>
          ))}
        </nav>
        <form onSubmit={go}>
          <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Rechercher un joueur (Nom#TAG)" />
        </form>
      </header>
      <main>
        {closed && tab !== "reglages" ? (
          <div className="empty">
            <h2>Lance le client League of Legends</h2>
            <p>Ethra lit tes données directement depuis le client, aucune clé ni compte requis.</p>
          </div>
        ) : tab === "profil" ? (
          <Profile puuid={puuid} onOpen={setPuuid} />
        ) : tab === "partie" ? (
          <Live phase={phase!} onOpen={(p) => (setPuuid(p), setTab("profil"))} />
        ) : tab === "builds" ? (
          <Builds />
        ) : (
          <section className="card settings">
            <h3>Champ select</h3>
            {([["autoRunes", "Importer les runes"], ["autoSpells", "Importer les sorts d'invocateur"], ["autoItems", "Créer l'item set"], ["autoAccept", "Accepter la partie automatiquement"]] as const).map(([k, l]) => (
              <label key={k}><input type="checkbox" checked={cfg[k]} onChange={(e) => save({ [k]: e.target.checked })} /> {l}</label>
            ))}
            <label>
              Source des builds{" "}
              <select value={cfg.source} onChange={(e) => save({ source: e.target.value })}>
                <option value="auto">Auto (tes parties si 3+, sinon Riot)</option>
                <option value="toi">Toujours tes parties</option>
                <option value="riot">Toujours la reco Riot</option>
              </select>
            </label>
            <h3>Overlay en jeu</h3>
            {([["ovEnabled", "Afficher l'overlay"], ["ovStats", "KDA, CS/min, or"], ["ovGold", "Différence d'or des équipes"], ["ovTimers", "Timers dragon / nashor"]] as const).map(([k, l]) => (
              <label key={k}><input type="checkbox" checked={cfg[k]} onChange={(e) => save({ [k]: e.target.checked })} /> {l}</label>
            ))}
            <p className="muted">L'overlay demande le mode fenêtré sans bordure dans les options vidéo du jeu.</p>
          </section>
        )}
      </main>
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
