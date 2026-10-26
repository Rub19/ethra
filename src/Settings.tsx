import { useState } from "react";
import { settings, Settings as S } from "./lcu";
import pkg from "../package.json";

const GENERAL = [
  ["autoRunes", "Importer les runes", "Crée une page « Ethra » dès que tu verrouilles ton champion."],
  ["autoSpells", "Importer les sorts d'invocateur", "Applique les sorts les plus gagnants."],
  ["autoItems", "Créer l'item set", "Ajoute ton build dans la boutique en jeu."],
  ["autoAccept", "Accepter la partie automatiquement", "Clique sur « Accepter » à ta place quand une partie est trouvée."],
] as const;

export default function Settings(_: { only?: string }) {
  const [cfg, setCfg] = useState<S>(settings);
  const save = (patch: Partial<S>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
  };
  const toggle = ([k, l, help]: (typeof GENERAL)[number]) => (
    <label key={k} className="toggle">
      <div><b>{l}</b><small>{help}</small></div>
      <input type="checkbox" checked={cfg[k]} onChange={(e) => save({ [k]: e.target.checked })} />
    </label>
  );
  return (
    <div className="settings">
      <div className="pagehead center">
        <h1>Réglages</h1>
        <p className="muted">Tout est gratuit. Ethra ne parle qu'à ton client League, à OP.GG pour la méta et à Data Dragon pour les images.</p>
      </div>
      <div className="setgrid">
        <section className="card">
          <h3>Sélection des champions</h3>
          {GENERAL.slice(0, 3).map(toggle)}
          <label className="toggle col">
            <div><b>Source des builds</b><small>« Auto » prend le build méta OP.GG, sinon tes parties, sinon la reco Riot.</small></div>
            <select value={cfg.source} onChange={(e) => save({ source: e.target.value })}>
              <option value="auto">Auto (OP.GG, sinon tes parties, sinon Riot)</option>
              <option value="opgg">Toujours la méta OP.GG</option>
              <option value="toi">Toujours tes parties</option>
              <option value="riot">Toujours la reco Riot</option>
            </select>
          </label>
        </section>
        <div className="setside">
          <section className="card">
            <h3>File d'attente</h3>
            {GENERAL.slice(3).map(toggle)}
          </section>
          <section className="card about">
            <h3>À propos</h3>
            <div className="aboutrow"><span>Version</span><b>{pkg.version}</b></div>
            <div className="aboutrow"><span>Code source</span><b>github.com/Rub19/ethra</b></div>
            <div className="aboutrow"><span>Données</span><b>Client LoL · OP.GG · Data Dragon</b></div>
            <small className="muted">Ethra n'est pas approuvé par Riot Games.</small>
          </section>
        </div>
      </div>
    </div>
  );
}
