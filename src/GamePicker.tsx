import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type GameKey = "lol" | "valo" | "tft";

const icons = new Map<GameKey, Promise<string | null>>();
const loadIcon = (g: GameKey) => {
  if (!icons.has(g))
    icons.set(
      g,
      invoke<ArrayBuffer>("game_icon", { game: g }).then(
        (b) => URL.createObjectURL(new Blob([b], { type: "image/x-icon" })),
        () => (icons.delete(g), null),
      ),
    );
  return icons.get(g)!;
};

const LOL_MARK = "m1.912 0 1.212 2.474v19.053L1.912 24h14.73l1.337-4.682H8.33V0ZM12 1.516c-.913 0-1.798.112-2.648.312v1.74a9.738 9.738 0 0 1 2.648-.368c5.267 0 9.536 4.184 9.536 9.348a9.203 9.203 0 0 1-2.3 6.086l-.273.954-.602 2.112c2.952-1.993 4.89-5.335 4.89-9.122C23.25 6.468 18.213 1.516 12 1.516Zm0 2.673c-.924 0-1.814.148-2.648.414v13.713h8.817a8.246 8.246 0 0 0 2.36-5.768c0-4.617-3.818-8.359-8.529-8.359zM2.104 7.312A10.858 10.858 0 0 0 .75 12.576c0 1.906.492 3.7 1.355 5.266z";
const VALO_MARK = "M23.792 2.152a.25.25 0 0 0-.098.083q-5.077 6.345-10.15 12.69c-.107.093-.025.288.119.265c2.439.003 4.877 0 7.316.001a.66.66 0 0 0 .552-.25l2.324-2.903a.72.72 0 0 0 .144-.49c-.002-3.077 0-6.153-.003-9.23c.016-.11-.1-.206-.204-.167zM.077 2.166c-.077.038-.074.132-.076.205q.002 4.612.001 9.225a.68.68 0 0 0 .158.463l7.64 9.55c.12.152.308.25.505.247c2.455 0 4.91.003 7.365 0c.142.02.222-.174.116-.265C10.661 15.176 5.526 8.766.4 2.35c-.08-.094-.174-.272-.322-.184z";

export function GameMark({ g, size = 30 }: { g: GameKey; size?: number }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (g === "valo") return;
    let on = true;
    loadIcon(g).then((x) => on && setSrc(x));
    return () => {
      on = false;
    };
  }, [g]);
  return (
    <span className={`gmark ${g}`} style={{ width: size, height: size }} aria-hidden="true">
      {g === "lol" && (src ? <img src={src} alt="" /> : <svg viewBox="0 0 24 24"><path d={LOL_MARK} /></svg>)}
      {g === "valo" && <svg viewBox="0 0 24 24"><path d={VALO_MARK} /></svg>}
      {g === "tft" && (src ? <img src={src} alt="" /> : (
        <svg viewBox="0 0 24 24">
          <path d="M4 3.5h16l-1.7 4.2h-4.2v11.2L12 21l-2.1-2.1V7.7H5.7Z" />
          <path d="M6.2 9.5a8 8 0 0 0 0 7.6M17.8 9.5a8 8 0 0 1 0 7.6" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
        </svg>
      ))}
    </span>
  );
}

const GAMES: { key: GameKey; name: string; sub: string; art: string }[] = [
  { key: "lol", name: "League of Legends", sub: "Profil, builds, tier list, overlays", art: "https://ddragon.leagueoflegends.com/cdn/img/champion/loading/Jinx_0.jpg" },
  { key: "valo", name: "VALORANT", sub: "Profil, matchs, rangs en direct", art: "https://media.valorant-api.com/agents/add6443a-41bd-e414-f6ad-e58d267f4e95/fullportrait.png" },
  { key: "tft", name: "Teamfight Tactics", sub: "Decks méta du set en cours", art: "https://ddragon.leagueoflegends.com/cdn/img/champion/loading/Teemo_0.jpg" },
];

export default function GamePicker({ game, onPick }: { game: GameKey; onPick: (g: GameKey) => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => !ref.current?.contains(e.target as Node) && setOpen(false);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("pointerdown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const enter = () => (clearTimeout(timer.current), setOpen(true));
  const leave = () => (timer.current = window.setTimeout(() => setOpen(false), 180));

  return (
    <div className="gpick" ref={ref} onPointerEnter={enter} onPointerLeave={leave}>
      <button className="gbtn" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)} title="Changer de jeu">
        <GameMark g={game} />
        <svg className="chev" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 3.5 5 6.5 8 3.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" /></svg>
      </button>
      <div className={`gmenu ${open ? "open" : ""}`} role="menu" aria-hidden={!open}>
        {GAMES.map((g) => (
          <button
            key={g.key}
            role="menuitemradio"
            aria-checked={game === g.key}
            tabIndex={open ? 0 : -1}
            className={`gcard ${g.key} ${game === g.key ? "on" : ""}`}
            onClick={() => (onPick(g.key), setOpen(false))}
          >
            <span className="gart" style={{ backgroundImage: `url(${g.art})` }} />
            <span className="ginfo">
              <GameMark g={g.key} size={34} />
              <b>{g.name}</b>
              <small>{g.sub}</small>
            </span>
          </button>
        ))}
        <p className="glegal">Kairo n'est pas approuvé par Riot Games et ne reflète pas ses opinions. League of Legends, VALORANT et Teamfight Tactics sont des marques de Riot Games, Inc.</p>
      </div>
    </div>
  );
}
