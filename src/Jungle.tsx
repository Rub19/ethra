import { useState } from "react";
import { DD, settings } from "./lcu";

type Camp = "blue" | "gromp" | "wolves" | "raptors" | "red" | "krugs";
export type Team = "blue" | "red";
export type Start = "red" | "blue";

const SIZE = 14870;
const CAMPS: Record<Camp, { name: string; at: [number, number] }> = {
  blue: { name: "Buff bleu", at: [3821, 8101] },
  gromp: { name: "Gromp", at: [2288, 8448] },
  wolves: { name: "Loups", at: [3783, 6495] },
  raptors: { name: "Raptors", at: [6823, 5508] },
  red: { name: "Buff rouge", at: [7765, 4020] },
  krugs: { name: "Krugs", at: [8394, 2641] },
};
const SCUTTLES: [number, number][] = [[4400, 9600], [10500, 5200]];
const ROUTES: Record<Start, Camp[]> = {
  red: ["red", "krugs", "raptors", "wolves", "blue", "gromp"],
  blue: ["blue", "gromp", "wolves", "raptors", "red", "krugs"],
};

const mirror = ([x, y]: [number, number], team: Team): [number, number] => (team === "blue" ? [x, y] : [SIZE - x, SIZE - y]);
const dist = (a: [number, number], b: [number, number]) => Math.hypot(a[0] - b[0], a[1] - b[1]);

export function route(team: Team, start: Start) {
  const steps = ROUTES[start].map((k) => ({ name: CAMPS[k].name, at: mirror(CAMPS[k].at, team) }));
  const last = steps[steps.length - 1].at;
  const crab = [...SCUTTLES].sort((a, b) => dist(a, last) - dist(b, last))[0];
  const side: "top" | "bot" = crab[1] > crab[0] ? "top" : "bot";
  return { steps: [...steps, { name: "Crabe", at: crab }], side };
}

const pt = ([x, y]: [number, number]) => [(x / SIZE) * 100, 100 - (y / SIZE) * 100] as const;

export default function JunglePath({ d, team: initialTeam = "blue" }: { d: DD; team?: Team }) {
  const [team, setTeam] = useState<Team>(initialTeam);
  const [start, setStart] = useState<Start>(() => (settings().jungleRoute === "blue" ? "blue" : "red"));
  const pickStart = (s: Start) => {
    setStart(s);
    try {
      localStorage.setItem("rw-settings", JSON.stringify({ ...settings(), jungleRoute: s }));
    } catch {}
  };
  const r = route(team, start);
  const pts = r.steps.map((s) => pt(s.at));

  return (
    <section className="card wide jpath">
      <div className="shead">
        <h3>Jungle path</h3>
        <div className="jctrl">
          <div className="seg">
            <button className={team === "blue" ? "on" : ""} onClick={() => setTeam("blue")}>Équipe bleue</button>
            <button className={team === "red" ? "on" : ""} onClick={() => setTeam("red")}>Équipe rouge</button>
          </div>
          <div className="seg">
            <button className={start === "red" ? "on" : ""} onClick={() => pickStart("red")}>Départ rouge</button>
            <button className={start === "blue" ? "on" : ""} onClick={() => pickStart("blue")}>Départ bleu</button>
          </div>
        </div>
      </div>
      <div className="jbody">
        <div className="jmap" style={{ backgroundImage: `url(https://ddragon.leagueoflegends.com/cdn/${d.v}/img/map/map11.png)` }}>
          <svg viewBox="0 0 100 100" aria-hidden="true">
            <polyline points={pts.map(([x, y]) => `${x},${y}`).join(" ")} />
            {pts.map(([x, y], i) => (
              <g key={i} className={i === pts.length - 1 ? "crab" : ""}>
                <circle cx={x} cy={y} r="3.1" />
                <text x={x} y={y + 1.1}>{i === pts.length - 1 ? "C" : i + 1}</text>
              </g>
            ))}
          </svg>
        </div>
        <div className="jsteps">
          <ol>
            {r.steps.map((s, i) => (
              <li key={i} className={i === r.steps.length - 1 ? "crab" : ""}>
                <span>{i === r.steps.length - 1 ? "C" : i + 1}</span>
                <b>{s.name}</b>
                {i === 0 && <small>apparition 1:30</small>}
                {i === r.steps.length - 1 && <small>apparition 3:30</small>}
              </li>
            ))}
          </ol>
          <p className="muted">
            Full clear classique qui finit côté {r.side === "top" ? "top" : "bot"} : prépare ton premier gank {r.side === "top" ? "sur le top ou le mid" : "sur le bot ou le mid"}, puis prends le crabe de cette rivière.
            Le départ {start === "red" ? "rouge" : "bleu"} demande en général un leash de ta lane {(team === "blue") === (start === "red") ? "bot" : "top"}.
          </p>
        </div>
      </div>
    </section>
  );
}
