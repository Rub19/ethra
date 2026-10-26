import { parseOpgg } from "./opggParse.ts";
import assert from "node:assert";

const t = `class A: name,list,ok,sub
class B: x,y

A("Zhonya's \\"H\\"",[1,-2.5,3e2],false,B([],None))`;
const r = parseOpgg(t);
assert.deepStrictEqual(r, { name: 'Zhonya\'s "H"', list: [1, -2.5, 300], ok: false, sub: { x: [], y: null } });

if (process.argv.includes("--live")) {
  const call = async (name: string, args: object) => {
    const res = await fetch("https://mcp-api.op.gg/mcp", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json, text/event-stream" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/call", params: { name, arguments: args } }),
    });
    return parseOpgg((await res.json()).result.content[0].text);
  };
  const a = await call("lol_get_champion_analysis", { champion: "AHRI", position: "mid", game_mode: "ranked", desired_output_fields: ["data.summary", "data.runes", "data.skills", "data.weak_counters", "data.starter_items", "data.fourth_items"] });
  assert.ok(a.data.runes.primary_rune_ids.length === 4 && a.data.skills.order.length > 10);
  console.log("analysis ok", a.data.runes.primary_page_name, a.data.weak_counters?.[0], a.data.fourth_items?.[0]);
  const m = await call("lol_list_lane_meta_champions", { position: "all", desired_output_fields: ["data.positions.top[].{champion,tier,rank,win_rate,play}"] });
  console.log("meta ok", Object.keys(m.data.positions), m.data.positions.top?.length, m.data.positions.top?.[0]);
  for (const c of ["KAISA", "KAI_SA", "TWISTED_FATE", "TWISTEDFATE", "DR_MUNDO", "WUKONG", "MONKEY_KING", "NUNU_WILLUMP", "NUNU", "LEBLANC", "CHOGATH", "RENATA_GLASC", "RENATA"]) {
    const x = await call("lol_get_champion_analysis", { champion: c, position: "mid", game_mode: "ranked", desired_output_fields: ["data.summary.{id}"] }).catch((e) => String(e).slice(0, 60));
    console.log(c, JSON.stringify(x).slice(0, 80));
  }
}
console.log("parse ok");
