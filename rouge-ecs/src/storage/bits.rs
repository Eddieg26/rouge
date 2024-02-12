use std::mem::size_of;

pub struct BitSet {
    bits: Vec<u8>,
}

impl BitSet {
    pub fn new() -> Self {
        Self { bits: vec![] }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: Vec::with_capacity(capacity),
        }
    }

    pub fn set(&mut self, index: usize) {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }

        self.bits[word] |= 1 << bit;
    }

    pub fn unset(&mut self, index: usize) {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }

        self.bits[word] &= !(1 << bit);
    }

    pub fn get(&self, index: usize) -> bool {
        let (word, bit) = self.index(index);

        if word >= self.bits.len() {
            return false;
        }

        self.bits[word] & (1 << bit) != 0
    }

    pub fn index(&self, index: usize) -> (usize, usize) {
        let byte = index / 8;
        let bit = index % 8;

        (byte, bit)
    }

    pub fn or(&self, other: &Self) -> BitSet {
        let len = self.len().max(other.len());
        let mut result = BitSet::with_capacity(len);

        for i in 0..len {
            if self.get(i) || other.get(i) {
                result.set(i);
            }
        }

        result
    }

    pub fn all_off(&self) -> bool {
        for word in self.bits.iter() {
            if *word != 0 {
                return false;
            }
        }

        true
    }

    pub fn len(&self) -> usize {
        self.bits.len() * 8
    }

    pub fn iter(&self) -> BitSetIter {
        BitSetIter::new(self)
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn clear(&mut self) {
        self.bits.clear();
    }
}

pub struct BitSetIter<'a> {
    set: &'a BitSet,
    index: usize,
}

impl<'a> BitSetIter<'a> {
    pub fn new(set: &'a BitSet) -> Self {
        Self { set, index: 0 }
    }
}

impl<'a> Iterator for BitSetIter<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.set.len() {
            return None;
        }

        let result = self.set.get(self.index);
        self.index += 1;

        Some(result)
    }
}

pub trait AsBytes: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl AsBytes for u8 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let byte: [u8; 1] = bytes.try_into().ok()?;
        Some(u8::from_le_bytes(byte))
    }
}

impl AsBytes for usize {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; size_of::<usize>()] = bytes.try_into().ok()?;
        Some(usize::from_le_bytes(bytes))
    }
}

impl AsBytes for u16 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 2] = bytes.try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }
}

impl AsBytes for u32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 4] = bytes.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }
}

impl AsBytes for u64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 8] = bytes.try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }
}

impl AsBytes for i8 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let byte: [u8; 1] = bytes.try_into().ok()?;
        Some(i8::from_le_bytes(byte))
    }
}

impl AsBytes for i16 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 2] = bytes.try_into().ok()?;
        Some(i16::from_le_bytes(bytes))
    }
}

impl AsBytes for i32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 4] = bytes.try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }
}

impl AsBytes for i64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 8] = bytes.try_into().ok()?;
        Some(i64::from_le_bytes(bytes))
    }
}

impl AsBytes for f32 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bits().to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 4] = bytes.try_into().ok()?;
        Some(f32::from_bits(u32::from_le_bytes(bytes)))
    }
}

impl AsBytes for f64 {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bits().to_le_bytes().iter().copied().collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let bytes: [u8; 8] = bytes.try_into().ok()?;
        Some(f64::from_bits(u64::from_le_bytes(bytes)))
    }
}

impl AsBytes for bool {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(bytes[0] != 0)
    }
}

impl AsBytes for char {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = [0; 4];
        self.encode_utf8(&mut bytes)
            .as_bytes()
            .iter()
            .copied()
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let code = u32::from_bytes(bytes)?;
        Some(std::char::from_u32(code)?)
    }
}

impl AsBytes for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(String::from_utf8(bytes.to_vec()).ok()?)
    }
}

impl AsBytes for () {
    fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }

    fn from_bytes(_: &[u8]) -> Option<Self> {
        Some(())
    }
}

impl<T: AsBytes> AsBytes for Vec<T> {
    fn to_bytes(&self) -> Vec<u8> {
        let size = std::mem::size_of::<T>();
        let mut bytes = Vec::with_capacity(size * self.len() + self.len() * size_of::<usize>());

        bytes.extend_from_slice(&(self.len() as u64).to_bytes());

        for item in self {
            let item = item.to_bytes();
            let len = bytes.len();
            bytes.extend_from_slice(&len.to_bytes());
            bytes.extend_from_slice(&item);
        }

        bytes
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let size = std::mem::size_of::<T>();
        let len = usize::from_bytes(&bytes[0..size])?;
        let mut result = Vec::with_capacity(len);

        let mut offset = size_of::<usize>();
        for _ in 0..bytes.len() {
            let item_len = usize::from_bytes(&bytes[offset..offset + size])?;
            offset += size;
            let item = T::from_bytes(&bytes[offset..offset + item_len])?;
            offset += item_len;
            result.push(item);
        }

        Some(result)
    }
}
