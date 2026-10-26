import { useState } from "react";
import { settings, Settings as S } from "./lcu";

type Card = { key: keyof S; title: string; desc: string; preview: React.ReactNode };

const CARDS: Card[] = [
  {
    key: "ovTimers", title: "Timers d'objectifs", desc: "Dragon, Nashor et réapparition des inhibiteurs, calculés depuis les événements de la partie.",
    preview: <div className="pvbox"><div className="ovt"><span>Nashor</span><b>0:48</b></div><div className="ovt"><span>Dragon</span><b>0:31</b></div></div>,
  },
  {
    key: "ovAlerts", title: "Alertes d'objectifs", desc: "Bandeau en haut de l'écran 30 secondes avant l'apparition d'un objectif.",
    preview: <div className="pvalert"><b>Dragon</b> apparaît dans <b>30 s</b></div>,
  },
  {
    key: "ovSkill", title: "Ordre des compétences", desc: "Te dit quelle compétence monter à chaque niveau, selon le build méta OP.GG de ton champion.",
    preview: <div className="pvskill"><span>Monte</span><b>Q</b></div>,
  },
  {
    key: "ovBench", title: "Benchmark de lane", desc: "Ton écart de CS, de valeur d'objets et de niveau avec ton adversaire direct, en direct.",
    preview: <div className="pvbox"><div className="ovh">vs Yasuo</div><div className="ovt"><span>CS</span><b className="good">+12</b></div><div className="ovt"><span>Objets</span><b className="bad">-0.4k</b></div></div>,
  },
  {
    key: "ovGold", title: "Valeur des objets", desc: "Différence de valeur totale des objets entre ton équipe et l'équipe adverse.",
    preview: <div className="pvbox"><div className="ovt"><span>Valeur objets</span><b className="good">+3.8k</b></div></div>,
  },
  {
    key: "ovStats", title: "Tes stats", desc: "KDA, CS par minute et or disponible, toujours visibles.",
    preview: <div className="pvbox"><div className="ovt"><span>KDA</span><b>4/1/6</b></div><div className="ovt"><span>CS/min</span><b>7.8</b></div></div>,
  },
];

export default function Overlays() {
  const [cfg, setCfg] = useState<S>(settings);
  const save = (patch: Partial<S>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);
    try {
      localStorage.setItem("rw-settings", JSON.stringify(next));
    } catch {}
  };
  return (
    <div className="overlays">
      <div className="pagehead ovhead">
        <div>
          <h1>Overlays</h1>
          <p className="muted">Widgets affichés par-dessus le jeu, transparents aux clics. Mets le jeu en mode fenêtré sans bordure (Options › Vidéo).</p>
        </div>
        <button className={`toggleall ${cfg.ovEnabled ? "on" : ""}`} onClick={() => save({ ovEnabled: !cfg.ovEnabled })}>
          {cfg.ovEnabled ? "Overlays activés" : "Overlays désactivés"}
        </button>
      </div>
      <div className={`ovgrid ${cfg.ovEnabled ? "" : "off"}`}>
        {CARDS.map((c) => (
          <div key={c.key} className="ovcard">
            <div className="ovprev">{c.preview}</div>
            <div className="ovbody">
              <h3>{c.title}</h3>
              <p className="muted">{c.desc}</p>
              <button className={`state ${cfg[c.key] ? "on" : ""}`} onClick={() => save({ [c.key]: !cfg[c.key] })}>
                {cfg[c.key] ? "Activé" : "Désactivé"}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
