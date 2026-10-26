import { invoke } from "@tauri-apps/api/core";

export const lcu = <T = any>(path: string, method = "GET", body?: unknown) =>
  invoke<T>("lcu", { method, path, body: body ?? null });

export const live = <T = any>(path: string) => invoke<T>("live", { path });

export type Item = { name: string; gold: { total: number }; into?: string[]; tags: string[] };
export type Rune = { id: number; name: string; icon: string };
export type Style = Rune & { slots: { runes: Rune[] }[] };
export type DD = {
  v: string;
  champs: Record<number, { id: string; name: string; tags: string[]; info: { attack: number; magic: number } }>;
  items: Record<string, Item>;
  styles: Style[];
  runes: Record<number, Rune>;
  spells: Record<number, { id: string; name: string }>;
  perkIcons: Record<number, string>;
};

const CDN = "https://ddragon.leagueoflegends.com/cdn";
let dd: Promise<DD> | undefined;

export function ddragon(): Promise<DD> {
  return (dd ??= (async () => {
    const v: string = (await (await fetch("https://ddragon.leagueoflegends.com/api/versions.json")).json())[0];
    const get = (f: string) => fetch(`${CDN}/${v}/data/fr_FR/${f}.json`).then((r) => r.json());
    const CD = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/";
    const [c, i, styles, s, perks] = await Promise.all([
      get("champion"), get("item"), get("runesReforged"), get("summoner"),
      fetch(`${CD}v1/perks.json`).then((r) => r.json()).catch(() => []),
    ]);
    const perkIcons: Record<number, string> = {};
    for (const p of perks as { id: number; iconPath: string }[]) perkIcons[p.id] = CD + p.iconPath.replace("/lol-game-data/assets/", "").toLowerCase();
    const champs: DD["champs"] = {};
    for (const x of Object.values<any>(c.data)) champs[+x.key] = { id: x.id, name: x.name, tags: x.tags, info: x.info };
    const spells: DD["spells"] = {};
    for (const x of Object.values<any>(s.data)) spells[+x.key] = { id: x.id, name: x.name };
    const runes: DD["runes"] = {};
    for (const st of styles as Style[]) {
      runes[st.id] = st;
      for (const sl of st.slots) for (const r of sl.runes) runes[r.id] = r;
    }
    return { v, champs, items: i.data, styles, runes, spells, perkIcons };
  })());
}

export const img = {
  champ: (d: DD, id: number) => `${CDN}/${d.v}/img/champion/${d.champs[id]?.id}.png`,
  item: (d: DD, id: number | string) => `${CDN}/${d.v}/img/item/${id}.png`,
  spell: (d: DD, id: number) => `${CDN}/${d.v}/img/spell/${d.spells[id]?.id}.png`,
  icon: (d: DD, id: number) => `${CDN}/${d.v}/img/profileicon/${id}.png`,
  rune: (d: DD, id: number) => d.perkIcons[id] ?? `https://ddragon.leagueoflegends.com/cdn/img/${d.runes[id]?.icon}`,
  emblem: (tier: string) => `https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests/${tier.toLowerCase()}.svg`,
  role: (pos: string) => `https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-${({ adc: "bottom", mid: "middle", support: "utility" } as Record<string, string>)[pos] ?? pos}.png`,
  ability: (d: DD, champ: string, key: string) =>
    `https://cdn.communitydragon.org/latest/champion/${champ}/ability-icon/${key.toLowerCase()}`,
};

export const isCompleted = (d: DD, id: number) => {
  const it = d.items[id];
  return !!it && !it.into?.length && it.gold.total >= 900 && !it.tags.includes("Consumable") && !it.tags.includes("Trinket");
};

export type Stats = Record<string, number | boolean>;
export type Participant = {
  participantId: number;
  teamId: number;
  championId: number;
  spell1Id: number;
  spell2Id: number;
  stats: Stats & { win: boolean };
};
export type Identity = { participantId: number; player: { puuid: string; gameName: string; tagLine: string; summonerName: string } };
export type Game = {
  gameId: number;
  gameCreation: number;
  gameDuration: number;
  queueId: number;
  mapId: number;
  participants: Participant[];
  participantIdentities: Identity[];
};
export type Summoner = { puuid: string; gameName: string; tagLine: string; summonerLevel: number; profileIconId: number; summonerId: number };

export const me = () => lcu<Summoner>("/lol-summoner/v1/current-summoner");
export const summoner = (puuid: string) => lcu<Summoner>(`/lol-summoner/v2/summoners/puuid/${puuid}`);

export async function findPlayer(riotId: string): Promise<string> {
  const [gameName, tagLine = ""] = riotId.split("#");
  const r = await lcu<{ puuid: string }>(
    `/lol-summoner/v1/alias/lookup?gameName=${encodeURIComponent(gameName.trim())}&tagLine=${encodeURIComponent(tagLine.trim())}`,
  ).catch(() => null);
  if (!r?.puuid) throw new Error("Joueur introuvable");
  return r.puuid;
}

export const history = (puuid: string, count = 20) =>
  lcu<{ games: { games: Game[] } }>(`/lol-match-history/v1/products/lol/${puuid}/matches?begIndex=0&endIndex=${count - 1}`).then(
    (r) => r.games.games,
  );

const gameCache = new Map<number, Promise<Game>>();
export const gameDetails = (id: number) => {
  if (!gameCache.has(id)) gameCache.set(id, lcu<Game>(`/lol-match-history/v1/games/${id}`));
  return gameCache.get(id)!;
};

export type Queue = { queueType: string; tier: string; division: string; leaguePoints: number; wins: number; losses: number };
export const ranked = (puuid: string) =>
  lcu<{ queueMap: Record<string, Queue> }>(`/lol-ranked/v1/ranked-stats/${puuid}`).then((r) => r.queueMap);

let queues: Promise<Record<number, string>> | undefined;
export const queueNames = () =>
  (queues ??= lcu<{ id: number; description: string; name: string }[]>("/lol-game-queues/v1/queues").then((qs) =>
    Object.fromEntries(qs.map((q) => [q.id, q.description || q.name])),
  ));

export const TIERS = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND", "MASTER", "GRANDMASTER", "CHALLENGER"];
const DIVS = ["IV", "III", "II", "I"];
export const rankScore = (q: Queue) =>
  TIERS.indexOf(q.tier) * 400 + (TIERS.indexOf(q.tier) >= 7 ? 0 : DIVS.indexOf(q.division) * 100) + q.leaguePoints;
export const rankLabel = (q?: Queue) =>
  !q || !q.tier || q.tier === "NONE" || q.tier === "UNRANKED"
    ? "Non classé"
    : `${q.tier[0]}${q.tier.slice(1).toLowerCase()} ${TIERS.indexOf(q.tier) >= 7 ? "" : q.division} ${q.leaguePoints} LP`;

export const n = (s: Stats, k: string) => Number(s[k] ?? 0);
export const kda = (s: Stats) => (n(s, "kills") + n(s, "assists")) / Math.max(1, n(s, "deaths"));
export const cs = (s: Stats) => n(s, "totalMinionsKilled") + n(s, "neutralMinionsKilled");
export const mmss = (sec: number) => `${Math.floor(sec / 60)}:${String(Math.floor(sec % 60)).padStart(2, "0")}`;
export const ago = (t: number) => {
  const m = Math.round((Date.now() - t) / 60000);
  return m < 60 ? `il y a ${m} min` : m < 1440 ? `il y a ${Math.round(m / 60)} h` : `il y a ${Math.round(m / 1440)} j`;
};

export function settings() {
  const def = { autoRunes: true, autoSpells: true, autoItems: true, autoAccept: false, source: "auto", ovTimers: true, ovStats: true, ovGold: true, ovEnabled: true, ovAlerts: true, ovBench: true, ovSkill: true };
  try {
    return { ...def, ...JSON.parse(localStorage.getItem("rw-settings") || "{}") } as typeof def;
  } catch {
    return def;
  }
}
export type Settings = ReturnType<typeof settings>;
