import type { Game } from "./lcu";

export const REMAKE_SECONDS = 300;
export const isRemake = (g: Pick<Game, "gameDuration">) => g.gameDuration < REMAKE_SECONDS;
export const counted = <T extends Pick<Game, "gameDuration">>(games: T[]) => games.filter((g) => !isRemake(g));

export type HistoryFilter = { result: "" | "win" | "loss"; queue: number; champ: number };
export const NO_FILTER: HistoryFilter = { result: "", queue: 0, champ: 0 };

export function filterGames(games: Game[], f: HistoryFilter): Game[] {
  return games.filter((g) => {
    const p = g.participants[0];
    if (f.queue && g.queueId !== f.queue) return false;
    if (f.champ && p.championId !== f.champ) return false;
    if (f.result && (isRemake(g) || (f.result === "win") !== p.stats.win)) return false;
    return true;
  });
}

export function mergeGames<T extends { gameId: number; gameCreation: number }>(fresh: T[], kept: T[], limit: number): T[] {
  const known = new Set(fresh.map((g) => g.gameId));
  return [...fresh, ...kept.filter((g) => !known.has(g.gameId))].sort((a, b) => b.gameCreation - a.gameCreation).slice(0, limit);
}

const distinctBy = (values: number[]) => [...new Set(values)];
export const queuesIn = (games: Game[]) => distinctBy(games.map((g) => g.queueId));
export const champsIn = (games: Game[]) => distinctBy(games.map((g) => g.participants[0].championId));

export type Form = { streak: { win: boolean; n: number } | null; strip: boolean[]; kdaNow: number | null; kdaBefore: number | null };

export function formOf(games: Game[], kdaOf: (g: Game) => number): Form {
  const list = counted(games);
  const avg = (part: Game[]) => part.reduce((sum, g) => sum + kdaOf(g), 0) / part.length;
  let n = 0;
  while (n < list.length && list[n].participants[0].stats.win === list[0].participants[0].stats.win) n++;
  return {
    streak: list.length ? { win: list[0].participants[0].stats.win, n } : null,
    strip: list.slice(0, 20).reverse().map((g) => g.participants[0].stats.win),
    kdaNow: list.length >= 10 ? avg(list.slice(0, 5)) : null,
    kdaBefore: list.length >= 10 ? avg(list.slice(5, 10)) : null,
  };
}

export type TftUnit = { characterId: string; tier: number; rarity: number };
export type TftTrait = { name: string; num_units: number; style: number };
export type TftGame = { id: string; at: number; seconds: number; queueId: number; placement: number; level: number; units: TftUnit[]; traits: TftTrait[] };

export function parseTftGames(raw: any, puuid: string): TftGame[] {
  const out: TftGame[] = [];
  for (const g of raw?.games ?? []) {
    const info = g?.json;
    const me = info?.participants?.find((p: any) => p.puuid === puuid);
    if (!me) continue;
    out.push({
      id: g.metadata?.match_id ?? String(info.game_datetime),
      at: info.game_datetime,
      seconds: info.game_length,
      queueId: info.queue_id,
      placement: me.placement,
      level: me.level,
      units: me.units ?? [],
      traits: (me.traits ?? []).filter((t: TftTrait) => t.style > 0),
    });
  }
  return out.sort((a, b) => b.at - a.at);
}

export function tftSummary(games: TftGame[]) {
  if (!games.length) return { games: 0, average: 0, top4: 0, wins: 0 };
  return {
    games: games.length,
    average: games.reduce((sum, g) => sum + g.placement, 0) / games.length,
    top4: games.filter((g) => g.placement <= 4).length / games.length,
    wins: games.filter((g) => g.placement === 1).length,
  };
}

const TFT_QUEUES: Record<number, string> = { 1090: "Normale", 1100: "Classée", 1130: "Hyper Roll", 1160: "Double Up", 1170: "Fortune's Favor", 1180: "Soul Brawl" };
export const tftQueue = (id: number) => TFT_QUEUES[id] ?? "TFT";
