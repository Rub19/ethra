export function parseOpgg(text: string): any {
  const classes: Record<string, string[]> = {};
  const lines = text.split("\n");
  let i = 0;
  for (; i < lines.length && lines[i].startsWith("class "); i++) {
    const [name, fields] = lines[i].slice(6).split(":");
    classes[name.trim()] = fields.split(",").map((f) => f.trim()).filter(Boolean);
  }
  const src = lines.slice(i).join("\n").trim();
  let p = 0;
  const ws = () => {
    while (/\s/.test(src[p] ?? "")) p++;
  };
  const value = (): any => {
    ws();
    const c = src[p];
    if (c === '"') {
      let s = "";
      for (p++; src[p] !== '"'; p++) {
        if (p >= src.length) throw new Error("OPGG: unterminated string");
        if (src[p] === "\\") {
          p++;
          s += ({ n: "\n", t: "\t" } as Record<string, string>)[src[p]] ?? src[p];
        } else s += src[p];
      }
      p++;
      return s;
    }
    if (c === "[") {
      p++;
      const out = [];
      for (ws(); src[p] !== "]"; ws()) {
        out.push(value());
        ws();
        if (src[p] === ",") p++;
      }
      p++;
      return out;
    }
    const m = /^[-+]?\d+(\.\d+)?([eE][-+]?\d+)?/.exec(src.slice(p));
    if (m) {
      p += m[0].length;
      return Number(m[0]);
    }
    const id = /^[A-Za-z_]\w*/.exec(src.slice(p))?.[0];
    if (!id) throw new Error(`OPGG: unexpected "${src.slice(p, p + 20)}"`);
    p += id.length;
    ws();
    const lit: Record<string, unknown> = { true: true, false: false, True: true, False: false, null: null, None: null };
    if (src[p] !== "(") return id in lit ? lit[id] : id;
    p++;
    const args = [];
    for (ws(); src[p] !== ")"; ws()) {
      args.push(value());
      ws();
      if (src[p] === ",") p++;
    }
    p++;
    const fields = classes[id] ?? [];
    return Object.fromEntries(args.map((a, k) => [fields[k] ?? k, a]));
  };
  return value();
}
