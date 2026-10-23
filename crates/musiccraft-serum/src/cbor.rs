//! A small CBOR decoder (RFC 8949): enough for Serum 2's state, which is a
//! map of maps of numbers, strings, arrays and byte strings. Tags are read
//! through (their content is kept), and indefinite lengths are supported.

/// Nesting deeper than this is refused.
const MAX_DEPTH: usize = 64;

/// A decoded CBOR item.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
    /// Entries in file order; non-text keys are written as text.
    Map(Vec<(String, Value)>),
}

impl Value {
    /// The value under `key`, if this is a map that has it.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A number (booleans count as 0 and 1).
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::Int(i) => Some(i as f64),
            Self::Float(f) => Some(f),
            Self::Bool(b) => Some(f64::from(u8::from(b))),
            _ => None,
        }
    }

    /// Text.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// An array's items.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// A map's entries.
    #[must_use]
    pub fn as_map(&self) -> Option<&[(String, Value)]> {
        match self {
            Self::Map(entries) => Some(entries),
            _ => None,
        }
    }
}

/// Decodes one item from the start of `data`.
///
/// # Errors
///
/// Truncated or malformed data, or nesting deeper than 64.
pub fn decode(data: &[u8]) -> Result<Value, String> {
    let mut reader = Reader { data, pos: 0 };
    reader.item(0)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

/// What an initial byte announces.
enum Head {
    /// Major type, additional information, argument.
    Item(u8, u8, u64),
    Indefinite(u8),
    Break,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&end| end <= self.data.len())
            .ok_or("the CBOR data is cut off")?;
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn uint(&mut self, n: usize) -> Result<u64, String> {
        Ok(self
            .take(n)?
            .iter()
            .fold(0_u64, |acc, &b| (acc << 8) | u64::from(b)))
    }

    fn head(&mut self) -> Result<Head, String> {
        let initial = self.byte()?;
        let major = initial >> 5;
        let info = initial & 0x1F;
        let arg = match info {
            0..=23 => u64::from(info),
            24 => self.uint(1)?,
            25 => self.uint(2)?,
            26 => self.uint(4)?,
            27 => self.uint(8)?,
            31 if major == 7 => return Ok(Head::Break),
            31 if (2..=5).contains(&major) => return Ok(Head::Indefinite(major)),
            _ => return Err(format!("bad CBOR item (major type {major}, info {info})")),
        };
        Ok(Head::Item(major, info, arg))
    }

    fn length(&self, n: u64) -> Result<usize, String> {
        usize::try_from(n)
            .ok()
            .filter(|&n| n <= self.data.len() - self.pos.min(self.data.len()))
            .ok_or_else(|| "a CBOR length runs past the end of the data".to_owned())
    }

    fn item(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("the CBOR data is nested too deeply".to_owned());
        }
        match self.head()? {
            Head::Break => Err("unexpected CBOR break".to_owned()),
            Head::Indefinite(major) => self.indefinite(major, depth),
            Head::Item(major, info, arg) => self.definite(major, info, arg, depth),
        }
    }

    fn definite(&mut self, major: u8, info: u8, arg: u64, depth: usize) -> Result<Value, String> {
        Ok(match major {
            0 => Value::Int(i128::from(arg)),
            1 => Value::Int(-1 - i128::from(arg)),
            2 => {
                let n = self.length(arg)?;
                Value::Bytes(self.take(n)?.to_vec())
            }
            3 => {
                let n = self.length(arg)?;
                Value::Text(String::from_utf8_lossy(self.take(n)?).into_owned())
            }
            4 => {
                let n = self.length(arg)?;
                let mut items = Vec::with_capacity(n.min(4_096));
                for _ in 0..n {
                    items.push(self.item(depth + 1)?);
                }
                Value::Array(items)
            }
            5 => {
                let n = self.length(arg)?;
                let mut entries = Vec::with_capacity(n.min(4_096));
                for _ in 0..n {
                    let key = key_text(self.item(depth + 1)?);
                    entries.push((key, self.item(depth + 1)?));
                }
                Value::Map(entries)
            }
            // A tag: keep what it tags.
            6 => self.item(depth + 1)?,
            _ => simple(info, arg)?,
        })
    }

    fn indefinite(&mut self, major: u8, depth: usize) -> Result<Value, String> {
        let mut items = Vec::new();
        let mut entries = Vec::new();
        let mut bytes = Vec::new();
        loop {
            if self.data.get(self.pos) == Some(&0xFF) {
                self.pos += 1;
                break;
            }
            match major {
                2 | 3 => match self.item(depth + 1)? {
                    Value::Bytes(b) => bytes.extend(b),
                    Value::Text(t) => bytes.extend(t.into_bytes()),
                    _ => return Err("bad chunk in an indefinite CBOR string".to_owned()),
                },
                4 => items.push(self.item(depth + 1)?),
                _ => {
                    let key = key_text(self.item(depth + 1)?);
                    entries.push((key, self.item(depth + 1)?));
                }
            }
        }
        Ok(match major {
            2 => Value::Bytes(bytes),
            3 => Value::Text(String::from_utf8_lossy(&bytes).into_owned()),
            4 => Value::Array(items),
            _ => Value::Map(entries),
        })
    }
}

/// Major type 7: simple values, and floats whose raw bits are in `arg`.
fn simple(info: u8, arg: u64) -> Result<Value, String> {
    Ok(match (info, arg) {
        (25, bits) => Value::Float(f16(bits as u16)),
        (26, bits) => Value::Float(f64::from(f32::from_bits(bits as u32))),
        (27, bits) => Value::Float(f64::from_bits(bits)),
        (_, 20) => Value::Bool(false),
        (_, 21) => Value::Bool(true),
        (_, 22 | 23) => Value::Null,
        _ => return Err(format!("unsupported CBOR simple value {arg}")),
    })
}

/// A half-precision float.
fn f16(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1F);
    let mantissa = f64::from(bits & 0x3FF);
    sign * match exponent {
        0 => mantissa * 2f64.powi(-24),
        31 if mantissa == 0.0 => f64::INFINITY,
        31 => f64::NAN,
        e => (1.0 + mantissa / 1_024.0) * 2f64.powi(e - 15),
    }
}

fn key_text(key: Value) -> String {
    match key {
        Value::Text(t) => t,
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

/// Encodes `value` (for tests: Serum 2 files are built from their parts).
#[must_use]
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write(value, &mut out);
    out
}

fn write_head(major: u8, arg: u64, out: &mut Vec<u8>) {
    let m = major << 5;
    match arg {
        0..=23 => out.push(m | arg as u8),
        24..=0xFF => out.extend([m | 24, arg as u8]),
        0x100..=0xFFFF => {
            out.push(m | 25);
            out.extend((arg as u16).to_be_bytes());
        }
        0x1_0000..=0xFFFF_FFFF => {
            out.push(m | 26);
            out.extend((arg as u32).to_be_bytes());
        }
        _ => {
            out.push(m | 27);
            out.extend(arg.to_be_bytes());
        }
    }
}

fn write(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.push(0xF6),
        Value::Bool(b) => out.push(if *b { 0xF5 } else { 0xF4 }),
        Value::Int(i) if *i >= 0 => write_head(0, *i as u64, out),
        Value::Int(i) => write_head(1, (-1 - *i) as u64, out),
        Value::Float(f) => {
            out.push(0xFB);
            out.extend(f.to_bits().to_be_bytes());
        }
        Value::Bytes(b) => {
            write_head(2, b.len() as u64, out);
            out.extend(b);
        }
        Value::Text(t) => {
            write_head(3, t.len() as u64, out);
            out.extend(t.as_bytes());
        }
        Value::Array(items) => {
            write_head(4, items.len() as u64, out);
            for item in items {
                write(item, out);
            }
        }
        Value::Map(entries) => {
            write_head(5, entries.len() as u64, out);
            for (k, v) in entries {
                write(&Value::Text(k.clone()), out);
                write(v, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_rfc_examples() {
        let cases: [(&[u8], Value); 10] = [
            (&[0x00], Value::Int(0)),
            (&[0x18, 0x64], Value::Int(100)),
            (&[0x38, 0x63], Value::Int(-100)),
            (&[0xF9, 0x3C, 0x00], Value::Float(1.0)),
            (&[0xF9, 0xC4, 0x00], Value::Float(-4.0)),
            (&[0xFA, 0x47, 0xC3, 0x50, 0x00], Value::Float(100_000.0)),
            (
                &[0xFB, 0x3F, 0xF1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9A],
                Value::Float(1.1),
            ),
            (&[0xF5], Value::Bool(true)),
            (&[0x64, 0x49, 0x45, 0x54, 0x46], Value::Text("IETF".into())),
            (
                &[0x9F, 0x01, 0x82, 0x02, 0x03, 0xFF],
                Value::Array(vec![
                    Value::Int(1),
                    Value::Array(vec![Value::Int(2), Value::Int(3)]),
                ]),
            ),
        ];
        for (bytes, want) in cases {
            assert_eq!(decode(bytes).unwrap(), want, "{bytes:02x?}");
        }
        // {_ "a": 1, "b": [_ 2, 3]}
        let map = decode(&[
            0xBF, 0x61, 0x61, 0x01, 0x61, 0x62, 0x9F, 0x02, 0x03, 0xFF, 0xFF,
        ])
        .unwrap();
        assert_eq!(map.get("a"), Some(&Value::Int(1)));
        assert_eq!(
            map.get("b").and_then(Value::as_array).map(<[_]>::len),
            Some(2)
        );
    }

    #[test]
    fn round_trips_and_refuses_bad_data() {
        let value = Value::Map(vec![
            ("x".into(), Value::Float(0.25)),
            ("n".into(), Value::Int(-70_000)),
            ("b".into(), Value::Bytes(vec![1, 2, 3])),
        ]);
        assert_eq!(decode(&encode(&value)).unwrap(), value);
        assert!(decode(&[0x62, 0x61]).is_err());
        assert!(decode(&[0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).is_err());
        assert!(decode(&[0x81; 100]).is_err());
    }
}
