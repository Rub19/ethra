import assert from "node:assert";
import { champsIn, counted, filterGames, NO_FILTER, parseTftGames, queuesIn, tftQueue, tftSummary } from "./history.ts";

const game = (gameId: number, queueId: number, championId: number, win: boolean, gameDuration = 1500) =>
  ({ gameId, queueId, gameDuration, participants: [{ championId, stats: { win } }] }) as any;
const games = [game(1, 420, 7, true), game(2, 420, 8, false), game(3, 450, 7, false), game(4, 420, 7, false, 200)];
const ids = (list: any[]) => list.map((g) => g.gameId);

assert.deepStrictEqual(ids(counted(games)), [1, 2, 3]);
assert.deepStrictEqual(ids(filterGames(games, NO_FILTER)), [1, 2, 3, 4]);
assert.deepStrictEqual(ids(filterGames(games, { ...NO_FILTER, result: "win" })), [1]);
assert.deepStrictEqual(ids(filterGames(games, { ...NO_FILTER, result: "loss" })), [2, 3]);
assert.deepStrictEqual(ids(filterGames(games, { ...NO_FILTER, queue: 450 })), [3]);
assert.deepStrictEqual(ids(filterGames(games, { result: "loss", queue: 420, champ: 8 })), [2]);
assert.deepStrictEqual(queuesIn(games), [420, 450]);
assert.deepStrictEqual(champsIn(games), [7, 8]);

const raw = {
  games: [
    { metadata: { match_id: "A" }, json: { game_datetime: 100, game_length: 1800, queue_id: 1100, participants: [{ puuid: "me", placement: 3, level: 8, units: [{ characterId: "TFT_X", tier: 2, rarity: 1 }], traits: [{ name: "T1", num_units: 3, style: 1 }, { name: "T2", num_units: 1, style: 0 }] }, { puuid: "other", placement: 1 }] } },
    { metadata: { match_id: "B" }, json: { game_datetime: 200, game_length: 1500, queue_id: 1090, participants: [{ puuid: "me", placement: 1, level: 9 }] } },
    { metadata: { match_id: "C" }, json: { game_datetime: 300, participants: [{ puuid: "other", placement: 2 }] } },
    { broken: true },
  ],
};
const tft = parseTftGames(raw, "me");
assert.deepStrictEqual(tft.map((g) => g.id), ["B", "A"]);
assert.deepStrictEqual(tft[1].traits.map((t) => t.name), ["T1"]);
assert.deepStrictEqual(tft[0].units, []);
assert.deepStrictEqual(parseTftGames(null, "me"), []);
const sum = tftSummary(tft);
assert.deepStrictEqual([sum.games, sum.average, sum.top4, sum.wins], [2, 2, 1, 1]);
assert.strictEqual(tftSummary([]).average, 0);
assert.strictEqual(tftQueue(1100), "Classée");
assert.strictEqual(tftQueue(9), "TFT");
console.log("history ok");
