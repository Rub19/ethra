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

export function GameMark({ g, size = 30 }: { g: GameKey; size?: number }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let on = true;
    loadIcon(g).then((x) => on && setSrc(x));
    return () => {
      on = false;
    };
  }, [g]);
  if (src) return <img className="gmark official" src={src} width={size} height={size} alt="" />;
  if (g === "tft")
    return (
      <svg className="gmark tft" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
        <rect width="32" height="32" rx="8" />
        <path d="M16 6 23.5 10.5v11L16 26 8.5 21.5v-11Z" fill="none" strokeWidth="2.2" />
        <path d="M16 11.2 17.6 14.6l3.6.4-2.7 2.4.8 3.6L16 19.2 12.7 21l.8-3.6-2.7-2.4 3.6-.4Z" />
      </svg>
    );
  return g === "lol" ? (
    <svg className="gmark lol" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <rect width="32" height="32" rx="8" />
      <path d="M16 6.5 24.2 11v10L16 25.5 7.8 21V11Z" fill="none" strokeWidth="2.2" />
      <path d="M13.2 11.5v9h6.2" fill="none" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  ) : (
    <svg className="gmark valo" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <rect width="32" height="32" rx="8" />
      <path d="M7 9h4.6l6.2 9.4-2.4 3.7Z" />
      <path d="M19.6 9H25l-6.6 10.3h-4.6Z" />
    </svg>
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
        <p className="glegal">Ethra n'est pas approuvé par Riot Games et ne reflète pas ses opinions. League of Legends, VALORANT et Teamfight Tactics sont des marques de Riot Games, Inc.</p>
      </div>
    </div>
  );
}
