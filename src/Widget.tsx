import type { ReactNode } from "react";

const CD = "https://raw.communitydragon.org/latest/";
export const ICON = {
  dragon: `${CD}game/assets/ux/minimap/icons/dragon.png`,
  baron: `${CD}game/assets/ux/minimap/icons/baron.png`,
  inhib: `${CD}plugins/rcp-fe-lol-match-history/global/default/inhibitor-100.png`,
};

export function Widget({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="w">
      <div className="wh">{title}</div>
      {children}
    </div>
  );
}

export function Row({ icon, label, value, tone, bar }: { icon?: string; label: string; value: ReactNode; tone?: "good" | "bad" | "up"; bar?: number }) {
  return (
    <div className={`wrow ${icon ? "" : "noi"}`}>
      {icon && <img src={icon} alt="" />}
      <span className="wl">{label}</span>
      <b className={tone ?? ""}>{value}</b>
      {bar !== undefined && <i className="wbar"><i style={{ width: `${Math.max(0, Math.min(1, bar)) * 100}%` }} /></i>}
    </div>
  );
}

export function Alert({ icon, name, secs }: { icon?: string; name: string; secs: number }) {
  return (
    <div className="walert">
      {icon && <img src={icon} alt="" />}
      <div>
        <b>{name}</b>
        <span>apparaît bientôt</span>
      </div>
      <em>{secs}s</em>
    </div>
  );
}

export function SkillHint({ skill, icon, level }: { skill: string; icon?: string; level: number }) {
  return (
    <div className="wskill">
      <div className="wsk">
        {icon ? <img src={icon} alt="" /> : null}
        <b>{skill}</b>
      </div>
      <div>
        <span>Niveau {level}</span>
        <b>Monte {skill}</b>
      </div>
    </div>
  );
}
