use serde_json::{json, Map, Value};
use std::collections::HashMap;

struct P<'a> {
    src: &'a [u8],
    p: usize,
    classes: &'a HashMap<String, Vec<String>>,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.p < self.src.len() && self.src[self.p].is_ascii_whitespace() {
            self.p += 1;
        }
    }

    fn peek(&self) -> u8 {
        *self.src.get(self.p).unwrap_or(&0)
    }

    fn list(&mut self, close: u8) -> Result<Vec<Value>, String> {
        let mut out = vec![];
        self.ws();
        while self.peek() != close {
            if self.p >= self.src.len() {
                return Err("OPGG: unterminated list".into());
            }
            out.push(self.value()?);
            self.ws();
            if self.peek() == b',' {
                self.p += 1;
            }
            self.ws();
        }
        self.p += 1;
        Ok(out)
    }

    fn string(&mut self) -> Result<Value, String> {
        let mut s = vec![];
        self.p += 1;
        loop {
            if self.p >= self.src.len() {
                return Err("OPGG: unterminated string".into());
            }
            match self.src[self.p] {
                b'"' => break,
                b'\\' => {
                    self.p += 1;
                    match self.peek() {
                        b'n' => s.push(b'\n'),
                        b't' => s.push(b'\t'),
                        c => s.push(c),
                    }
                }
                c => s.push(c),
            }
            self.p += 1;
        }
        self.p += 1;
        Ok(Value::String(String::from_utf8_lossy(&s).into_owned()))
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.p;
        let mut q = self.p;
        let at = |i: usize| *self.src.get(i).unwrap_or(&0);
        if at(q) == b'-' || at(q) == b'+' {
            q += 1;
        }
        if !at(q).is_ascii_digit() {
            return None;
        }
        while at(q).is_ascii_digit() {
            q += 1;
        }
        let mut float = false;
        if at(q) == b'.' && at(q + 1).is_ascii_digit() {
            float = true;
            q += 1;
            while at(q).is_ascii_digit() {
                q += 1;
            }
        }
        if matches!(at(q), b'e' | b'E') {
            let mut r = q + 1;
            if at(r) == b'-' || at(r) == b'+' {
                r += 1;
            }
            if at(r).is_ascii_digit() {
                float = true;
                q = r;
                while at(q).is_ascii_digit() {
                    q += 1;
                }
            }
        }
        let text = std::str::from_utf8(&self.src[start..q]).ok()?;
        self.p = q;
        if !float {
            if let Ok(i) = text.trim_start_matches('+').parse::<i64>() {
                return Some(json!(i));
            }
        }
        Some(json!(text.parse::<f64>().ok()?))
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.peek() {
            b'"' => return self.string(),
            b'[' => {
                self.p += 1;
                return Ok(Value::Array(self.list(b']')?));
            }
            _ => {}
        }
        if let Some(n) = self.number() {
            return Ok(n);
        }
        let start = self.p;
        while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
            self.p += 1;
        }
        if start == self.p {
            let end = (self.p + 20).min(self.src.len());
            return Err(format!("OPGG: unexpected \"{}\"", String::from_utf8_lossy(&self.src[self.p..end])));
        }
        let id = std::str::from_utf8(&self.src[start..self.p]).unwrap_or_default().to_string();
        self.ws();
        if self.peek() != b'(' {
            return Ok(match id.as_str() {
                "true" | "True" => json!(true),
                "false" | "False" => json!(false),
                "null" | "None" => Value::Null,
                _ => Value::String(id),
            });
        }
        self.p += 1;
        let args = self.list(b')')?;
        let fields = self.classes.get(&id);
        let mut out = Map::new();
        for (k, a) in args.into_iter().enumerate() {
            let key = fields.and_then(|f| f.get(k)).cloned().unwrap_or_else(|| k.to_string());
            out.insert(key, a);
        }
        Ok(Value::Object(out))
    }
}

pub fn parse_opgg(text: &str) -> Result<Value, String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut classes = HashMap::new();
    let mut i = 0;
    while i < lines.len() && lines[i].starts_with("class ") {
        if let Some((name, fields)) = lines[i][6..].split_once(':') {
            classes.insert(name.trim().to_string(), fields.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect());
        }
        i += 1;
    }
    let src = lines[i..].join("\n");
    let src = src.trim();
    P { src: src.as_bytes(), p: 0, classes: &classes }.value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check() {
        let t = "class A: name,list,ok,sub\nclass B: x,y\n\nA(\"Zhonya's \\\"H\\\"\",[1,-2.5,3e2],false,B([],None))";
        let r = parse_opgg(t).unwrap();
        assert_eq!(r["name"], "Zhonya's \"H\"");
        assert_eq!(r["list"][0].as_f64(), Some(1.0));
        assert_eq!(r["list"][1].as_f64(), Some(-2.5));
        assert_eq!(r["list"][2].as_f64(), Some(300.0));
        assert_eq!(r["ok"], false);
        assert_eq!(r["sub"]["x"], json!([]));
        assert_eq!(r["sub"]["y"], Value::Null);
    }

    #[test]
    fn broken() {
        assert!(parse_opgg("\"abc").is_err());
        assert!(parse_opgg("[1,").is_err());
        assert!(parse_opgg("@").is_err());
    }
}
