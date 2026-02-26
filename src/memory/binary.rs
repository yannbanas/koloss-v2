// Compact binary serialization for knowledge graph and terms.
// No external dependencies — pure Rust little-endian format.
//
// Format:
//   [magic: u32 = 0x4B4F4C53 "KOLS"]
//   [version: u8]
//   [section_count: u16]
//   [sections...]
//
// Section:
//   [type: u8] [len: u32] [data: [u8; len]]

use crate::core::{Term, OrderedFloat};

const MAGIC: u32 = 0x4B4F4C53; // "KOLS"
const VERSION: u8 = 1;

// Term tags
const TAG_VAR: u8 = 0;
const TAG_ATOM: u8 = 1;
const TAG_INT: u8 = 2;
const TAG_FLOAT: u8 = 3;
const TAG_STR: u8 = 4;
const TAG_BOOL: u8 = 5;
const TAG_COMPOUND: u8 = 6;
const TAG_LIST: u8 = 7;
const TAG_NIL: u8 = 8;

pub struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(4096) }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_bytes(&mut self, data: &[u8]) {
        self.write_u32(data.len() as u32);
        self.buf.extend_from_slice(data);
    }

    fn write_str(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
    }

    pub fn write_term(&mut self, term: &Term) {
        match term {
            Term::Var(v) => {
                self.write_u8(TAG_VAR);
                self.write_u32(*v);
            }
            Term::Atom(a) => {
                self.write_u8(TAG_ATOM);
                self.write_u32(*a);
            }
            Term::Int(n) => {
                self.write_u8(TAG_INT);
                self.write_i64(*n);
            }
            Term::Float(f) => {
                self.write_u8(TAG_FLOAT);
                self.write_u64(f.0);
            }
            Term::Str(s) => {
                self.write_u8(TAG_STR);
                self.write_str(s);
            }
            Term::Bool(b) => {
                self.write_u8(TAG_BOOL);
                self.write_u8(if *b { 1 } else { 0 });
            }
            Term::Compound(f, args) => {
                self.write_u8(TAG_COMPOUND);
                self.write_u32(*f);
                self.write_u16(args.len() as u16);
                for arg in args {
                    self.write_term(arg);
                }
            }
            Term::List(items) => {
                self.write_u8(TAG_LIST);
                self.write_u16(items.len() as u16);
                for item in items {
                    self.write_term(item);
                }
            }
            Term::Nil => {
                self.write_u8(TAG_NIL);
            }
        }
    }

    pub fn write_terms(&mut self, terms: &[Term]) {
        self.write_u32(terms.len() as u32);
        for t in terms {
            self.write_term(t);
        }
    }

    pub fn write_header(&mut self) {
        self.write_u32(MAGIC);
        self.write_u8(VERSION);
    }

    pub fn write_symbol_table(&mut self, symbols: &[&str]) {
        self.write_u32(symbols.len() as u32);
        for s in symbols {
            self.write_str(s);
        }
    }
}

pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.pos >= self.data.len() { return None; }
        let v = self.data[self.pos];
        self.pos += 1;
        Some(v)
    }

    fn read_u16(&mut self) -> Option<u16> {
        if self.pos + 2 > self.data.len() { return None; }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Some(v)
    }

    fn read_u32(&mut self) -> Option<u32> {
        if self.pos + 4 > self.data.len() { return None; }
        let v = u32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().ok()?);
        self.pos += 4;
        Some(v)
    }

    fn read_u64(&mut self) -> Option<u64> {
        if self.pos + 8 > self.data.len() { return None; }
        let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().ok()?);
        self.pos += 8;
        Some(v)
    }

    fn read_i64(&mut self) -> Option<i64> {
        if self.pos + 8 > self.data.len() { return None; }
        let v = i64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().ok()?);
        self.pos += 8;
        Some(v)
    }

    fn read_bytes(&mut self) -> Option<Vec<u8>> {
        let len = self.read_u32()? as usize;
        if self.pos + len > self.data.len() { return None; }
        let v = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(v)
    }

    fn read_str(&mut self) -> Option<String> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).ok()
    }

    pub fn read_term(&mut self) -> Option<Term> {
        let tag = self.read_u8()?;
        match tag {
            TAG_VAR => Some(Term::Var(self.read_u32()?)),
            TAG_ATOM => Some(Term::Atom(self.read_u32()?)),
            TAG_INT => Some(Term::Int(self.read_i64()?)),
            TAG_FLOAT => Some(Term::Float(OrderedFloat(self.read_u64()?))),
            TAG_STR => Some(Term::Str(self.read_str()?.into())),
            TAG_BOOL => Some(Term::Bool(self.read_u8()? != 0)),
            TAG_COMPOUND => {
                let f = self.read_u32()?;
                let n = self.read_u16()? as usize;
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(self.read_term()?);
                }
                Some(Term::Compound(f, args))
            }
            TAG_LIST => {
                let n = self.read_u16()? as usize;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.read_term()?);
                }
                Some(Term::List(items))
            }
            TAG_NIL => Some(Term::Nil),
            _ => None,
        }
    }

    pub fn read_terms(&mut self) -> Option<Vec<Term>> {
        let count = self.read_u32()? as usize;
        let mut terms = Vec::with_capacity(count);
        for _ in 0..count {
            terms.push(self.read_term()?);
        }
        Some(terms)
    }

    pub fn read_header(&mut self) -> Option<u8> {
        let magic = self.read_u32()?;
        if magic != MAGIC { return None; }
        self.read_u8()
    }

    pub fn read_symbol_table(&mut self) -> Option<Vec<String>> {
        let count = self.read_u32()? as usize;
        let mut syms = Vec::with_capacity(count);
        for _ in 0..count {
            syms.push(self.read_str()?);
        }
        Some(syms)
    }
}

// Compact bitfield operations for grid storage
pub fn pack_grid(grid: &[Vec<u8>]) -> Vec<u8> {
    if grid.is_empty() { return vec![0, 0]; }
    let rows = grid.len() as u16;
    let cols = grid[0].len() as u16;
    let mut buf = Vec::with_capacity(4 + grid.len() * grid[0].len());
    buf.extend_from_slice(&rows.to_le_bytes());
    buf.extend_from_slice(&cols.to_le_bytes());

    // Check if all values fit in 4 bits (0-15) → pack 2 per byte
    let all_small = grid.iter().flat_map(|r| r.iter()).all(|&v| v < 16);
    buf.push(if all_small { 1 } else { 0 });

    if all_small {
        let mut pairs = grid.iter().flat_map(|r| r.iter());
        loop {
            match (pairs.next(), pairs.next()) {
                (Some(&a), Some(&b)) => buf.push(a | (b << 4)),
                (Some(&a), None) => buf.push(a),
                _ => break,
            }
        }
    } else {
        for row in grid {
            buf.extend_from_slice(row);
        }
    }
    buf
}

pub fn unpack_grid(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    if data.len() < 5 { return None; }
    let rows = u16::from_le_bytes([data[0], data[1]]) as usize;
    let cols = u16::from_le_bytes([data[2], data[3]]) as usize;
    let packed = data[4] == 1;

    if rows == 0 || cols == 0 { return Some(Vec::new()); }

    let mut grid = vec![vec![0u8; cols]; rows];
    let payload = &data[5..];

    if packed {
        let total = rows * cols;
        let mut idx = 0;
        for byte_idx in 0..payload.len() {
            if idx >= total { break; }
            grid[idx / cols][idx % cols] = payload[byte_idx] & 0x0F;
            idx += 1;
            if idx >= total { break; }
            grid[idx / cols][idx % cols] = payload[byte_idx] >> 4;
            idx += 1;
        }
    } else {
        let mut idx = 0;
        for r in 0..rows {
            for c in 0..cols {
                if idx < payload.len() {
                    grid[r][c] = payload[idx];
                    idx += 1;
                }
            }
        }
    }

    Some(grid)
}
