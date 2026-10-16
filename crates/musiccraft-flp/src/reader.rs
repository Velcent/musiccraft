// Ported from PyFLP (https://github.com/demberto/PyFLP),
// Copyright (C) 2022 demberto. GPL-3.0-or-later; see `LICENSE`.

//! A little-endian byte reader with the semantics of the `construct`
//! primitives PyFLP's event structs use: an `Optional` field that does not
//! fit reads nothing and leaves the position where it was, so a later,
//! smaller field can still be read.

/// A position in a byte slice.
#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    /// Bytes not read yet.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }

    /// `n` bytes, or `None` (reading nothing) if fewer are left.
    pub fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let out = self.data.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(out)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.bytes(N)?.try_into().ok()
    }

    pub fn u8(&mut self) -> Option<u8> {
        self.array::<1>().map(|b| b[0])
    }

    pub fn i8(&mut self) -> Option<i8> {
        self.array::<1>().map(i8::from_le_bytes)
    }

    /// `Flag`: one byte, anything but zero is true.
    pub fn flag(&mut self) -> Option<bool> {
        self.u8().map(|b| b != 0)
    }

    pub fn u16(&mut self) -> Option<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub fn i16(&mut self) -> Option<i16> {
        self.array().map(i16::from_le_bytes)
    }

    pub fn u32(&mut self) -> Option<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub fn i32(&mut self) -> Option<i32> {
        self.array().map(i32::from_le_bytes)
    }

    pub fn u64(&mut self) -> Option<u64> {
        self.array().map(u64::from_le_bytes)
    }

    pub fn f32(&mut self) -> Option<f32> {
        self.array().map(f32::from_le_bytes)
    }

    pub fn f64(&mut self) -> Option<f64> {
        self.array().map(f64::from_le_bytes)
    }

    /// `FourByteBool`: a `u32`, anything but zero is true.
    pub fn bool32(&mut self) -> Option<bool> {
        self.u32().map(|v| v != 0)
    }

    /// `VarInt`: 7 bits per byte, low bits first, top bit set on every byte
    /// but the last.
    pub fn varint(&mut self) -> Option<u64> {
        let start = self.at;
        let mut value: u64 = 0;
        let mut shift = 0;
        loop {
            let Some(byte) = self.u8() else {
                self.at = start;
                return None;
            };
            if shift < 64 {
                value |= u64::from(byte & 0x7F) << shift;
            }
            if byte & 0x80 == 0 {
                return Some(value);
            }
            shift += 7;
        }
    }

    /// Runs `read`; if it returns `None`, the position goes back to where it
    /// was (like `construct`'s `Optional` and `GreedyRange`).
    pub fn attempt<T>(&mut self, read: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let start = self.at;
        let out = read(self);
        if out.is_none() {
            self.at = start;
        }
        out
    }

    /// `GreedyRange`: items read with `item` until one does not fit.
    pub fn greedy<T>(&mut self, mut item: impl FnMut(&mut Self) -> Option<T>) -> Vec<T> {
        let mut out = Vec::new();
        while self.remaining() > 0 {
            match self.attempt(&mut item) {
                Some(value) => out.push(value),
                None => break,
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_reads_leave_the_position_on_failure() {
        let mut r = Reader::new(&[1, 0, 0, 0, 7]);
        assert_eq!(r.u32(), Some(1));
        assert_eq!(r.u32(), None, "one byte left");
        assert_eq!(r.u8(), Some(7), "the byte is still there");
        assert_eq!(r.u8(), None);
    }

    #[test]
    fn varints_and_greedy_ranges() {
        let mut r = Reader::new(&[0xAC, 0x02, 5]);
        assert_eq!(r.varint(), Some(300));
        assert_eq!(r.varint(), Some(5));
        assert_eq!(Reader::new(&[0x80]).varint(), None, "unterminated");
        let mut r = Reader::new(&[1, 0, 2, 0, 3]);
        assert_eq!(r.greedy(Reader::u16), [1, 2]);
        assert_eq!(r.remaining(), 1, "a partial item is left unread");
    }
}
