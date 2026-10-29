import { invoke } from "@tauri-apps/api/core";

type Host = "local" | "pd" | "glz" | "shared" | "session";
export const valo = <T = any>(host: Host, path = "", method = "GET", body?: unknown) =>
  invoke<T>("valo", { host, method, path, body: body ?? null });

export type VSession = { puuid: string; region: string; shard: string };
export const vSession = () => valo<VSession>("session");

export type Agent = { name: string; icon: string; portrait: string; bg: string; role: string; roleIcon: string };
export type VD = {
  agents: Record<string, Agent>;
  maps: Record<string, { name: string; icon: string; splash: string }>;
  tiers: Record<number, { name: string; icon: string; color: string }>;
  weapons: Record<string, { name: string; icon: string; category: string }>;
};

const API = "https://valorant-api.com/v1";
const get = (p: string) => fetch(`${API}/${p}`).then((r) => r.json()).then((j) => j.data);
const title = (s: string) => s.toLowerCase().replace(/(^|\s)\S/g, (c) => c.toUpperCase());
let vd: Promise<VD> | undefined;

export function vdata(): Promise<VD> {
  return (vd ??= (async () => {
    const [a, m, t, w] = await Promise.all([
      get("agents?isPlayableCharacter=true&language=fr-FR"), get("maps?language=fr-FR"), get("competitivetiers?language=fr-FR"), get("weapons?language=fr-FR").catch(() => []),
    ]);
    const agents: VD["agents"] = {};
    for (const x of a) agents[x.uuid.toLowerCase()] = { name: x.displayName, icon: x.displayIcon, portrait: x.fullPortrait, bg: x.background, role: x.role?.displayName ?? "", roleIcon: x.role?.displayIcon ?? "" };
    const maps: VD["maps"] = {};
    for (const x of m) maps[x.mapUrl] = { name: x.displayName, icon: x.listViewIcon, splash: x.splash };
    const tiers: VD["tiers"] = {};
    for (const x of t.at(-1).tiers) tiers[x.tier] = { name: title(x.tierName), icon: x.smallIcon ?? x.largeIcon, color: `#${x.color.slice(0, 6)}` };
    const weapons: VD["weapons"] = {};
    for (const x of w) weapons[x.uuid.toLowerCase()] = { name: x.displayName, icon: x.killStreamIcon ?? x.displayIcon, category: String(x.category ?? "").split("::").pop() ?? "" };
    return { agents, maps, tiers, weapons };
  })());
}

export const QUEUE: Record<string, string> = {
  competitive: "Compétition", unrated: "Non classé", swiftplay: "Vélocité", spikerush: "Spike Rush", deathmatch: "Combat à mort",
  hurm: "Combat à mort en équipe", ggteam: "Escalade", onefa: "Réplication", premier: "Premier", newmap: "Nouvelle carte", "": "Personnalisée",
};
export const queueLabel = (q: string) => QUEUE[q] ?? q;

let act: Promise<string> | undefined;
export const currentAct = () =>
  (act ??= valo<{ Seasons: { ID: string; Type: string; IsActive: boolean }[] }>("shared", "/content-service/v3/content").then(
    (c) => c.Seasons.find((s) => s.Type === "act" && s.IsActive)?.ID ?? "",
  ));

export type VRank = { tier: number; rr: number; wins: number; games: number; peak: number };
const rankCache = new Map<string, Promise<VRank>>();
export const vRank = (puuid: string) => {
  if (!rankCache.has(puuid))
    rankCache.set(puuid, (async () => {
      const [r, a] = await Promise.all([valo("pd", `/mmr/v1/players/${puuid}`), currentAct()]);
      const seasons: Record<string, any> = r.QueueSkills?.competitive?.SeasonalInfoBySeasonID ?? {};
      const s = seasons[a];
      const peak = Math.max(0, ...Object.values(seasons).flatMap((x) => Object.keys(x.WinsByTier ?? {}).map(Number)));
      return { tier: s?.CompetitiveTier ?? 0, rr: s?.RankedRating ?? 0, wins: s?.NumberOfWinsWithPlacements ?? 0, games: s?.NumberOfGames ?? 0, peak };
    })().catch(() => ({ tier: 0, rr: 0, wins: 0, games: 0, peak: 0 })));
  return rankCache.get(puuid)!;
};

export type VName = { Subject: string; GameName: string; TagLine: string };
export type Seen = { puuid: string; name: string; tag: string; t: number };
export function seenPlayers(): Seen[] {
  try {
    return JSON.parse(localStorage.getItem("rw-vseen") || "[]");
  } catch {
    return [];
  }
}
function remember(list: VName[]) {
  const fresh = list.filter((n) => n.GameName).map((n) => ({ puuid: n.Subject, name: n.GameName, tag: n.TagLine, t: Date.now() }));
  if (!fresh.length) return;
  const ids = new Set(fresh.map((x) => x.puuid));
  try {
    localStorage.setItem("rw-vseen", JSON.stringify([...fresh, ...seenPlayers().filter((x) => !ids.has(x.puuid))].slice(0, 400)));
  } catch {}
}
export const vNames = (puuids: string[]) =>
  valo<VName[]>("pd", "/name-service/v2/players", "PUT", puuids).then((list) => (remember(list), list));

const nameCache = new Map<string, Promise<VName | undefined>>();
const waiters = new Map<string, (n?: VName) => void>();
let queue: string[] = [];
let flushT: number | undefined;
function flush() {
  const batch = queue;
  queue = [];
  flushT = undefined;
  vNames(batch).then(
    (list) => {
      const by = new Map(list.map((n) => [n.Subject, n]));
      batch.forEach((p) => waiters.get(p)?.(by.get(p)));
    },
    () => batch.forEach((p) => (waiters.get(p)?.(undefined), nameCache.delete(p))),
  );
}
export function vName(puuid: string) {
  if (!nameCache.has(puuid)) {
    nameCache.set(puuid, new Promise((r) => waiters.set(puuid, r)));
    queue.push(puuid);
    flushT ??= window.setTimeout(flush, 60);
  }
  return nameCache.get(puuid)!;
}

export type VPlayer = {
  subject: string; gameName: string; tagLine: string; teamId: string; characterId: string; competitiveTier: number; accountLevel: number;
  stats: { score: number; roundsPlayed: number; kills: number; deaths: number; assists: number } | null;
};
export type VMatch = {
  matchInfo: { matchId: string; mapId: string; gameLengthMillis: number; gameStartMillis: number; queueID: string; isRanked: boolean };
  players: VPlayer[];
  teams: { teamId: string; won: boolean; roundsWon: number; roundsPlayed: number }[] | null;
  roundResults: {
    roundNum: number; winningTeam: string; roundResult: string;
    plantRoundTime?: number; bombPlanter?: string; plantSite?: string; defuseRoundTime?: number; bombDefuser?: string;
    playerStats: {
      subject: string;
      kills: { killer: string; victim: string; roundTime: number; finishingDamage?: { damageType: string; damageItem: string } }[];
      damage: { damage: number; headshots: number; bodyshots: number; legshots: number }[];
      economy?: { loadoutValue: number; spent: number; remaining: number };
    }[];
    playerEconomies?: { subject: string; loadoutValue: number; spent: number; remaining: number }[];
  }[] | null;
};

export const vHistory = (puuid: string, count = 20, queue = "", start = 0) =>
  valo<{ History: { MatchID: string; GameStartTime: number; QueueID: string }[] }>(
    "pd",
    `/match-history/v1/history/${puuid}?startIndex=${start}&endIndex=${start + count}${queue ? `&queue=${queue}` : ""}`,
  ).then((r) => r.History);

export type VRR = { earned: number; after: number; tierBefore: number; tierAfter: number; placement: boolean };
const rrCache = new Map<string, Promise<Record<string, VRR>>>();
export const vRRUpdates = (puuid: string) => {
  if (!rrCache.has(puuid))
    rrCache.set(
      puuid,
      valo<{ Matches: any[] }>("pd", `/mmr/v1/players/${puuid}/competitiveupdates?startIndex=0&endIndex=20&queue=competitive`).then(
        (r) =>
          Object.fromEntries(
            r.Matches.map((x) => [
              x.MatchID,
              { earned: x.RankedRatingEarned, after: x.RankedRatingAfterUpdate, tierBefore: x.TierBeforeUpdate, tierAfter: x.TierAfterUpdate, placement: x.IsPlacementMatch },
            ]),
          ),
        () => ({}),
      ),
    );
  return rrCache.get(puuid)!;
};

const matchCache = new Map<string, Promise<VMatch>>();
let chain: Promise<unknown> = Promise.resolve();
// ponytail: one match-details call at a time to stay under Riot's PD rate limit, parallelize with a small pool if history feels slow
export const vMatch = (id: string) => {
  if (!matchCache.has(id)) {
    const get = async (tries: number): Promise<VMatch> => {
      try {
        return await valo<VMatch>("pd", `/match-details/v1/matches/${id}`);
      } catch (e) {
        if (tries > 0 && /429|Too Many|rate/i.test(String(e))) {
          await new Promise((r) => setTimeout(r, (4 - tries) * 2500));
          return get(tries - 1);
        }
        throw e;
      }
    };
    const p = chain.then(() => get(3));
    chain = p.catch(() => {});
    matchCache.set(id, p);
    p.catch(() => matchCache.delete(id));
  }
  return matchCache.get(id)!;
};

export type VStat = { acs: number; adr: number; hs: number; fk: number; fd: number; kd: number; mk: number[] };
export function vStats(m: VMatch): Record<string, VStat> {
  const acc: Record<string, { dmg: number; head: number; shots: number; fk: number; fd: number; mk: number[] }> = {};
  const at = (s: string) => (acc[s] ??= { dmg: 0, head: 0, shots: 0, fk: 0, fd: 0, mk: [0, 0, 0, 0, 0, 0] });
  for (const r of m.roundResults ?? []) {
    const kills = r.playerStats.flatMap((p) => p.kills ?? []);
    const first = [...kills].sort((a, b) => a.roundTime - b.roundTime)[0];
    if (first) (at(first.killer).fk++, at(first.victim).fd++);
    const per: Record<string, number> = {};
    for (const k of kills) per[k.killer] = (per[k.killer] ?? 0) + 1;
    for (const [who, c] of Object.entries(per)) if (c >= 3) at(who).mk[Math.min(c, 5)]++;
    for (const p of r.playerStats)
      for (const d of p.damage ?? []) {
        const a = at(p.subject);
        a.dmg += d.damage;
        a.head += d.headshots;
        a.shots += d.headshots + d.bodyshots + d.legshots;
      }
  }
  const out: Record<string, VStat> = {};
  for (const p of m.players) {
    const a = at(p.subject);
    const rounds = Math.max(1, p.stats?.roundsPlayed ?? 0);
    out[p.subject] = {
      acs: (p.stats?.score ?? 0) / rounds, adr: a.dmg / rounds, hs: a.shots ? a.head / a.shots : 0, fk: a.fk, fd: a.fd, mk: a.mk,
      kd: (p.stats?.kills ?? 0) / Math.max(1, p.stats?.deaths ?? 0),
    };
  }
  return out;
}

export const vTeamWon = (m: VMatch, team: string) => !!m.teams?.find((t) => t.teamId === team)?.won;
export const vScore = (m: VMatch, team: string) => {
  const mine = m.teams?.find((t) => t.teamId === team);
  const other = m.teams?.find((t) => t.teamId !== team);
  return mine && other ? `${mine.roundsWon} - ${other.roundsWon}` : "";
};

const decode = (b64: string) => {
  try {
    return JSON.parse(atob(b64));
  } catch {
    return null;
  }
};

export async function vParties(puuids: string[], self: string): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  const inMatch = new Set(puuids);
  const pres = await valo<{ presences: { puuid: string; product: string; private: string }[] }>("local", "/chat/v4/presences").catch(() => ({ presences: [] }));
  for (const p of pres.presences) {
    if (p.product !== "valorant" || !inMatch.has(p.puuid) || !p.private) continue;
    const j = decode(p.private);
    const id = j?.partyPresenceData?.partyId ?? j?.partyId;
    if (id) out[p.puuid] = id;
  }
  const mine = await valo<{ CurrentPartyID: string }>("glz", `/parties/v1/players/${self}`).catch(() => null);
  if (mine?.CurrentPartyID) {
    const party = await valo<{ Members: { Subject: string }[] }>("glz", `/parties/v1/parties/${mine.CurrentPartyID}`).catch(() => null);
    for (const m of party?.Members ?? []) if (inMatch.has(m.Subject)) out[m.Subject] = mine.CurrentPartyID;
  }
  return out;
}

export const PARTY_LABEL = ["", "", "Duo", "Trio", "Quatuor", "Five stack"];

export type VPhase = "MENUS" | "PREGAME" | "INGAME" | null;
export async function vPhase(puuid: string): Promise<VPhase> {
  const r = await valo<{ presences: { puuid: string; product: string; private: string }[] }>("local", "/chat/v4/presences");
  const p = r.presences.find((x) => x.puuid === puuid && x.product === "valorant");
  if (!p?.private) return "MENUS";
  try {
    const j = JSON.parse(atob(p.private));
    return (j.matchPresenceData?.sessionLoopState ?? j.sessionLoopState ?? "MENUS") as VPhase;
  } catch {
    return "MENUS";
  }
}

export const mkLabel = (mk: number[]) =>
  [[5, "ACE"], [4, "4K"], [3, "3K"]].filter(([n]) => mk[n as number] > 0).map(([n, l]) => (mk[n as number] > 1 ? `${l} ×${mk[n as number]}` : String(l)));

export function vWeaponKills(m: VMatch, puuid: string) {
  const out: Record<string, number> = {};
  for (const r of m.roundResults ?? [])
    for (const p of r.playerStats)
      for (const k of p.kills ?? []) {
        if (k.killer !== puuid) continue;
        const key = k.finishingDamage?.damageType === "Weapon" ? k.finishingDamage.damageItem.toLowerCase() : k.finishingDamage?.damageType === "Melee" ? "melee" : "ability";
        out[key] = (out[key] ?? 0) + 1;
      }
  return out;
}

export function vEconomy(m: VMatch, team: string) {
  const members = new Set(m.players.filter((p) => p.teamId === team).map((p) => p.subject));
  return (m.roundResults ?? []).map((r) => {
    const eco = r.playerEconomies ?? r.playerStats.map((p) => ({ subject: p.subject, ...(p.economy ?? { loadoutValue: 0, spent: 0, remaining: 0 }) }));
    const v = eco.filter((e) => members.has(e.subject)).reduce((a, e) => a + (e.loadoutValue ?? 0), 0);
    const pistol = r.roundNum === 0 || r.roundNum === 12;
    return { value: v, kind: pistol ? "Pistolet" : v < 5000 ? "Éco" : v < 10000 ? "Semi-éco" : v < 20000 ? "Semi" : "Full" };
  });
}
