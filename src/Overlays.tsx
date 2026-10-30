import { useState } from "react";
import { settings, Settings as S } from "./lcu";
import { Alert, ICON, Row, SkillHint, Widget } from "./Widget";

const CH = (id: string) => `https://ddragon.leagueoflegends.com/cdn/15.1.1/img/champion/${id}.png`;
const IT = (id: number) => `https://ddragon.leagueoflegends.com/cdn/15.1.1/img/item/${id}.png`;

type Card = { key: keyof S; title: string; desc: string; preview: React.ReactNode };

const CARDS: Card[] = [
  {
    key: "ovTimers", title: "Timers d'objectifs", desc: "Dragon, Nashor et réapparition des inhibiteurs, calculés depuis les événements de la partie.",
    preview: (
      <Widget title="Objectifs">
        <Row icon={ICON.baron} label="Nashor" value="0:48" bar={0.85} />
        <Row icon={ICON.dragon} label="Dragon" value="UP" tone="up" />
        <Row icon={ICON.inhib} label="Inhib mid rouge" value="3:12" bar={0.35} />
      </Widget>
    ),
  },
  {
    key: "ovAlerts", title: "Alertes d'objectifs", desc: "Bandeau en haut de l'écran 30 secondes avant l'apparition d'un objectif.",
    preview: <Alert icon={ICON.dragon} name="Dragon" secs={30} />,
  },
  {
    key: "ovSkill", title: "Ordre des compétences", desc: "Indique la compétence à monter à chaque niveau, selon le build méta OP.GG.",
    preview: <SkillHint skill="Q" level={4} icon="https://cdn.communitydragon.org/latest/champion/Ahri/ability-icon/q" />,
  },
  {
    key: "ovBench", title: "Benchmark de lane", desc: "Ton écart de CS, de valeur d'objets et de niveau avec ton adversaire direct.",
    preview: (
      <Widget title="vs Yasuo">
        <Row label="CS" value="+12" tone="good" />
        <Row label="Objets" value="-0.4k" tone="bad" />
        <Row label="Niveau" value="+1" tone="good" />
      </Widget>
    ),
  },
  {
    key: "ovDeaths", title: "Ennemis morts", desc: "Les adversaires morts et leur temps de réapparition, pour savoir quand forcer un objectif.",
    preview: (
      <Widget title="Ennemis morts">
        <Row icon={CH("Yasuo")} label="Yasuo" value="24s" tone="good" />
        <Row icon={CH("Thresh")} label="Thresh" value="11s" tone="good" />
      </Widget>
    ),
  },
  {
    key: "ovItem", title: "Prochain objet", desc: "Le prochain objet de ton build méta et l'or qu'il te manque pour l'acheter.",
    preview: (
      <Widget title="Prochain objet">
        <Row icon={IT(3089)} label="Coiffe de Rabadon" value="-620" bar={0.82} />
      </Widget>
    ),
  },
  {
    key: "ovGold", title: "Valeur des objets", desc: "Différence de valeur totale des objets entre ton équipe et l'équipe adverse.",
    preview: (
      <Widget title="Valeur des objets">
        <Row label="Équipes" value="+3.8k" tone="good" />
      </Widget>
    ),
  },
  {
    key: "ovJungle", title: "Jungle path", desc: "Ton full clear étape par étape pendant les premières minutes quand tu joues jungle, avec le crabe à 3:30. Le départ se règle sur la page du champion.",
    preview: (
      <Widget title="Jungle path, fin côté top">
        <Row label="1. Buff rouge" value="" />
        <Row label="2. Krugs" value="" />
        <Row label="3. Raptors" value="" />
        <Row label="C. Crabe" value="1:12" />
      </Widget>
    ),
  },
  {
    key: "ovStats", title: "Tes stats", desc: "KDA, CS par minute et or disponible, toujours visibles.",
    preview: (
      <Widget title="Toi">
        <Row label="KDA" value="4 / 1 / 6" />
        <Row label="CS/min" value="7.8" />
        <Row label="Or" value="1 240" />
      </Widget>
    ),
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
