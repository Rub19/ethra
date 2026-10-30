import { call } from "./opgg";

type Asset = { name: string; icon: string };
export type TftData = { set: string; units: Record<string, Asset & { cost: number }>; items: Record<string, Asset>; traits: Record<string, Asset> };

const CD = "https://raw.communitydragon.org/latest/";
const png = (p?: string) => (p ? `${CD}game/${p.toLowerCase().replace(/\.(tex|dds)$/, ".png")}` : "");
let data: Promise<TftData> | undefined;

export function tftData(): Promise<TftData> {
  return (data ??= (async () => {
    try {
      const c = JSON.parse(localStorage.getItem("rw-tft") || "null");
      if (c && Date.now() - c.t < 864e5) return c.data as TftData;
    } catch {}
    const j = await (await fetch(`${CD}cdragon/tft/fr_fr.json`)).json();
    const set = Object.keys(j.sets).sort((a, b) => +a - +b).at(-1)!;
    const s = j.sets[set];
    const units: TftData["units"] = {};
    for (const c of s.champions ?? []) units[c.apiName] = { name: c.name, cost: c.cost, icon: png(c.tileIcon ?? c.squareIcon ?? c.icon) };
    const traits: TftData["traits"] = {};
    for (const t of s.traits ?? []) traits[t.apiName] = { name: t.name, icon: png(t.icon) };
    const items: TftData["items"] = {};
    for (const i of j.items ?? []) if (String(i.apiName).startsWith("DA_")) items[i.apiName] = { name: i.name, icon: png(i.icon) };
    const out = { set, units, items, traits };
    try {
      localStorage.setItem("rw-tft", JSON.stringify({ t: Date.now(), data: out }));
    } catch {}
    return out;
  })());
}

export type TftDeck = {
  id: string;
  name: Record<string, string>;
  units: { key: string; items: string[]; tier: number; isCore: boolean }[];
  traits: { key: string; style: number; numUnits: number }[];
  badge: { key: string; value: any }[];
  stat: { opTier: string; deck: { avgPlacement: number; top4Rate: number; winRate: number; pickRate: number; compsCount: number } };
};

export const tftDecks = () => call("tft_list_meta_decks", {}).then((r: any) => ({ decks: r.data as TftDeck[], games: r.metadata?.gameStatCounts as number, at: r.metadata?.gameStatDateTime as string }));
