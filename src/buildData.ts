import { DD, Participant, history, isCompleted, lcu, me, n, settings } from "./lcu";
import { opggBuild } from "./opgg";

export type Build = {
  source: "toi" | "riot" | "opgg";
  games: number;
  wins: number;
  primary: number;
  sub: number;
  perks: number[];
  spells: number[];
  start: number[];
  order: number[];
  core: number[];
  skills: number[];
};

const top = <T>(xs: T[], key = (x: T) => String(x)) => {
  const c = new Map<string, { x: T; n: number }>();
  for (const x of xs) {
    const k = key(x);
    c.set(k, { x, n: (c.get(k)?.n ?? 0) + 1 });
  }
  return [...c.values()].sort((a, b) => b.n - a.n);
};

export const POSITIONS: Record<string, string> = { top: "TOP", jungle: "JUNGLE", middle: "MIDDLE", bottom: "BOTTOM", utility: "UTILITY" };

export async function riotBuild(champ: number, position = "NONE", map = 11): Promise<Build | null> {
  const pages = await lcu<any[]>(`/lol-perks/v1/recommended-pages/champion/${champ}/position/${position}/map/${map}`).catch(() => []);
  const p = pages?.[0];
  if (!p) return null;
  return {
    source: "riot", games: 0, wins: 0,
    primary: p.primaryPerkStyleId, sub: p.secondaryPerkStyleId,
    perks: p.perks.map((x: any) => x.id), spells: p.summonerSpellIds ?? [],
    start: [], order: [], core: [], skills: [],
  };
}

export async function myBuild(d: DD, champ: number, map = 11): Promise<Build | null> {
  const self = await me();
  const games = (await history(self.puuid, 100)).filter((g) => g.participants[0].championId === champ && (g.mapId ?? map) === map);
  if (!games.length) return null;
  const ps = games.map((g) => g.participants[0]);
  const wins = ps.filter((p) => p.stats.win).length;

  const pageKey = (p: Participant) => [0, 1, 2, 3, 4, 5, "PrimaryStyle", "SubStyle"].map((k) => n(p.stats, `perk${k}`)).join(",");
  const score = (k: string) => ps.filter((p) => pageKey(p) === k).reduce((s, p) => s + (p.stats.win ? 2 : 1), 0);
  const best = [...ps].sort((a, b) => score(pageKey(b)) - score(pageKey(a)))[0].stats;
  const perks = [0, 1, 2, 3, 4, 5].map((k) => n(best, `perk${k}`));
  const shards = [0, 1, 2].map((k) => n(best, `statPerk${k}`)).filter(Boolean);

  const spells = top(ps, (p) => [p.spell1Id, p.spell2Id].sort().join(","))[0].x;

  const orders = ps.map((p) => [0, 1, 2, 3, 4, 5].map((k) => n(p.stats, `item${k}`)).filter((id) => isCompleted(d, id)));

  const rank = new Map<number, number[]>();
  for (const o of orders) o.forEach((id, i) => rank.set(id, [...(rank.get(id) ?? []), i]));
  const order = [...rank.entries()]
    .filter(([, r]) => r.length >= Math.max(1, orders.length * 0.3))
    .sort((a, b) => a[1].reduce((s, x) => s + x, 0) / a[1].length - b[1].reduce((s, x) => s + x, 0) / b[1].length)
    .map(([id]) => id);
  const core = top(ps.flatMap((p) => [0, 1, 2, 3, 4, 5].map((k) => n(p.stats, `item${k}`)).filter((id) => isCompleted(d, id)))).map((x) => x.x);

  return {
    source: "toi", games: games.length, wins,
    primary: n(best, "perkPrimaryStyle"), sub: n(best, "perkSubStyle"),
    perks: shards.length === 3 ? [...perks, ...shards] : perks,
    spells: spells ? [spells.spell1Id, spells.spell2Id] : [],
    start: [],
    order: order.slice(0, 6),
    core: core.slice(0, 9),
    skills: [],
  };
}

export async function bestBuild(d: DD, champ: number, position: string, map: number, source: string): Promise<Build | null> {
  const riot = await riotBuild(champ, position, map);
  const fill = (b: Build | null) => {
    if (b && b.perks.length < 9 && riot) b.perks = [...b.perks, ...riot.perks.slice(6)];
    return b;
  };
  const opgg = () => (map === 11 || map === 12 ? opggBuild(d, champ, position, map === 12 ? "aram" : "ranked", settings().rankFilter).catch(() => null) : Promise.resolve(null));
  const mine = () => myBuild(d, champ, map).catch(() => null).then(fill);
  if (source === "riot") return riot;
  if (source === "toi") return (await mine()) ?? riot;
  if (source === "opgg") return (await opgg()) ?? riot;
  return (await opgg()) ?? (await mine()) ?? riot;
}

const ROLE_LABEL: Record<string, string> = { top: "Top", jungle: "Jungle", mid: "Mid", adc: "ADC", support: "Support", TOP: "Top", JUNGLE: "Jungle", MIDDLE: "Mid", BOTTOM: "ADC", UTILITY: "Support" };

export function pageName(d: DD, champ: number, b: Build, role = "", aram = false) {
  const pos = aram ? "ARAM" : ROLE_LABEL[(b as any).position ?? ""] ?? ROLE_LABEL[role] ?? "";
  return `Ethra : ${d.champs[champ]?.name ?? champ}${pos ? ` ${pos}` : ""}`;
}

export async function applyBuild(d: DD, champ: number, b: Build, opts: { runes: boolean; spells: boolean; items: boolean; role?: string; aram?: boolean }) {
  const name = pageName(d, champ, b, opts.role, opts.aram);
  const done: string[] = [];
  if (opts.runes && b.perks.length >= 9) {
    const pages = await lcu<any[]>("/lol-perks/v1/pages");
    const old = pages.find((p) => p.name.startsWith("Ethra"));
    if (old) await lcu(`/lol-perks/v1/pages/${old.id}`, "DELETE");
    await lcu("/lol-perks/v1/pages", "POST", { name, primaryStyleId: b.primary, subStyleId: b.sub, selectedPerkIds: b.perks, current: true });
    done.push("runes");
  }
  if (opts.spells && b.spells.length === 2) {
    await lcu("/lol-champ-select/v1/session/my-selection", "PATCH", { spell1Id: b.spells[0], spell2Id: b.spells[1] });
    done.push("sorts");
  }
  const items = [...b.order, ...b.core.filter((x) => !b.order.includes(x))];
  if (opts.items && items.length) {
    const self = await me();
    const path = `/lol-item-sets/v1/item-sets/${self.summonerId}/sets`;
    const sets = await lcu<any>(path);
    const block = (type: string, ids: number[]) => ({ type, items: ids.map((id) => ({ id: String(id), count: 1 })) });
    sets.itemSets = (sets.itemSets ?? []).filter((s: any) => !s.title.startsWith("Ethra"));
    sets.itemSets.push({
      title: name, associatedChampions: [champ], associatedMaps: [], map: "any", mode: "any", type: "custom", sortrank: 0,
      uid: crypto.randomUUID(),
      blocks: [
        b.start.length && block("Départ", b.start),
        block(b.source === "opgg" ? `Build méta OP.GG (${b.games.toLocaleString("fr")} parties)` : `Ton build (${b.games} parties)`, items),
        "boots" in b && (b as any).boots.length && block("Bottes", (b as any).boots),
      ].filter(Boolean),
    });
    await lcu(path, "PUT", sets);
    done.push("items");
  }
  return done;
}
