import { invoke } from "@tauri-apps/api/core";
import { parseOpgg } from "./opggParse";
import type { Build } from "./buildData";
import type { DD } from "./lcu";

export const OPGG_POS: Record<string, string> = { TOP: "top", JUNGLE: "jungle", MIDDLE: "mid", BOTTOM: "adc", UTILITY: "support" };
export const POS_LABEL: Record<string, string> = { top: "Top", jungle: "Jungle", mid: "Mid", adc: "ADC", support: "Support" };
export const TIER_LABEL = ["", "S", "A", "B", "C", "D"];

const cache = new Map<string, Promise<any>>();
function call(tool: string, args: object) {
  const key = tool + JSON.stringify(args);
  if (!cache.has(key)) {
    const p = invoke<string>("opgg", { tool, args }).then(parseOpgg);
    p.catch(() => cache.delete(key));
    cache.set(key, p);
  }
  return cache.get(key)!;
}

export type MetaRow = { champ: number; name: string; position: string; tier: number; rank: number; rankPrev: number; winRate: number; pickRate: number; banRate: number; play: number; kda: number };

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, "");
export function champId(d: DD, name: string) {
  const k = norm(name);
  const hit = Object.entries(d.champs).find(([, c]) => norm(c.id) === k || norm(c.name) === k) ?? Object.entries(d.champs).find(([, c]) => k.startsWith(norm(c.id)));
  return hit ? +hit[0] : 0;
}

export async function meta(d: DD): Promise<MetaRow[]> {
  const fields = ["top", "jungle", "mid", "adc", "support"].map((p) => `data.positions.${p}[].{ban_rate,champion,kda,pick_rate,play,rank,rank_prev,tier,win_rate}`);
  const r = await call("lol_list_lane_meta_champions", { position: "all", desired_output_fields: fields });
  return Object.entries<any[]>(r.data.positions).flatMap(([position, rows]) =>
    rows.map((x) => ({
      champ: champId(d, x.champion), name: x.champion, position, tier: x.tier, rank: x.rank, rankPrev: x.rank_prev,
      winRate: x.win_rate, pickRate: x.pick_rate, banRate: x.ban_rate, play: x.play, kda: x.kda,
    })),
  );
}

export async function mainPosition(d: DD, champ: number) {
  const rows = (await meta(d)).filter((r) => r.champ === champ).sort((a, b) => b.play - a.play);
  return rows[0]?.position ?? "mid";
}

export type Counter = { champ: number; name: string; play: number; myWin: number };
export type OpggBuild = Build & {
  position: string;
  tier: number;
  rank: number;
  winRate: number;
  pickRate: number;
  banRate: number;
  boots: number[];
  situational: number[];
  maxOrder: string[];
  runeWin: number;
  runePlay: number;
  weak: Counter[];
  strong: Counter[];
};

const SLOT: Record<string, number> = { Q: 1, W: 2, E: 3, R: 4 };

export async function opggBuild(d: DD, champ: number, position?: string): Promise<OpggBuild | null> {
  const c = d.champs[champ];
  if (!c) return null;
  const pos = position && position !== "NONE" ? OPGG_POS[position] ?? position : await mainPosition(d, champ);
  const r = await call("lol_get_champion_analysis", {
    champion: c.id.toUpperCase(), position: pos, game_mode: "ranked",
    desired_output_fields: [
      "data.summary.average_stats", "data.runes", "data.summoner_spells", "data.starter_items", "data.core_items", "data.boots",
      "data.fourth_items", "data.fifth_items", "data.sixth_items", "data.skills", "data.skill_masteries.{ids[]}", "data.weak_counters", "data.strong_counters",
    ],
  });
  const x = r.data;
  if (!x?.runes) return null;
  const avg = x.summary?.average_stats ?? {};
  const ids = (list: any[] | undefined) => (list ?? []).flatMap((b) => b.ids as number[]);
  const counter = (k: any): Counter => ({ champ: k.champion_id, name: k.champion_name, play: k.play, myWin: k.my_win_rate });
  return {
    source: "opgg", games: avg.play ?? 0, wins: Math.round((avg.play ?? 0) * (avg.win_rate ?? 0)),
    primary: x.runes.primary_page_id, sub: x.runes.secondary_page_id,
    perks: [...x.runes.primary_rune_ids, ...x.runes.secondary_rune_ids, ...x.runes.stat_mod_ids],
    spells: x.summoner_spells?.ids ?? [],
    start: x.starter_items?.ids ?? [],
    order: x.core_items?.ids ?? [],
    core: [...new Set(ids([...(x.fourth_items ?? []), ...(x.fifth_items ?? []), ...(x.sixth_items ?? [])]))].slice(0, 8),
    skills: (x.skills?.order ?? []).map((k: string) => SLOT[k] ?? 0),
    position: pos, tier: avg.tier, rank: avg.rank, winRate: avg.win_rate, pickRate: avg.pick_rate, banRate: avg.ban_rate,
    boots: x.boots?.ids ?? [],
    situational: [],
    maxOrder: x.skill_masteries?.ids ?? [],
    runeWin: x.runes.win / Math.max(1, x.runes.play), runePlay: x.runes.play,
    weak: (x.weak_counters ?? []).map(counter).slice(0, 6),
    strong: (x.strong_counters ?? []).map(counter).slice(0, 6),
  };
}
