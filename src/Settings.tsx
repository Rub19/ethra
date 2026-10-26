import { useState } from "react";
import { settings, Settings as S } from "./lcu";

const GENERAL = [
  ["autoRunes", "Importer les runes", "Crée une page « Ethra » dès que tu verrouilles ton champion."],
  ["autoSpells", "Importer les sorts d'invocateur", "Applique les sorts les plus gagnants."],
  ["autoItems", "Créer l'item set", "Ajoute ton build dans la boutique en jeu."],
  ["autoAccept", "Accepter la partie automatiquement", "Clique sur « Accepter » à ta place quand une partie est trouvée."],
] as const;
const OVERLAY = [
  ["ovEnabled", "Afficher l'overlay", "Widgets par-dessus le jeu, transparents aux clics."],
  ["ovStats", "KDA, CS/min, or", "Tes stats en direct."],
  ["ovGold", "Différence d'or", "Valeur des objets de ton équipe contre l'équipe adverse."],
  ["ovTimers", "Timers dragon / nashor", "Calculés depuis les événements de la partie."],
] as const;

export default function Settings({ only }: { only: "general" | "overlay" }) {
  const [cfg, setCfg] = useState<S>(settings);
  const save = (patch: Partial<S>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
  };
  const rows = only === "overlay" ? OVERLAY : GENERAL;
  return (
    <div className="settings">
      <h1>{only === "overlay" ? "Overlays" : "Réglages"}</h1>
      <p className="muted">
        {only === "overlay"
          ? "L'overlay s'affiche quand une partie est en cours. Mets le jeu en mode fenêtré sans bordure (Options › Vidéo)."
          : "Tout est gratuit, rien n'est envoyé ailleurs que vers ton client League."}
      </p>
      <section className="card">
        {rows.map(([k, l, help]) => (
          <label key={k} className="toggle">
            <div><b>{l}</b><small>{help}</small></div>
            <input type="checkbox" checked={cfg[k]} onChange={(e) => save({ [k]: e.target.checked })} />
          </label>
        ))}
        {only === "general" && (
          <label className="toggle">
            <div><b>Source des builds</b><small>« Auto » prend le build méta OP.GG ; si OP.GG ne répond pas, tes propres parties, puis la reco Riot.</small></div>
            <select value={cfg.source} onChange={(e) => save({ source: e.target.value })}>
              <option value="auto">Auto (OP.GG, sinon tes parties, sinon Riot)</option>
              <option value="opgg">Toujours la méta OP.GG</option>
              <option value="toi">Toujours tes parties</option>
              <option value="riot">Toujours la reco Riot</option>
            </select>
          </label>
        )}
      </section>
    </div>
  );
}
