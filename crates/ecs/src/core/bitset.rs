#[derive(Debug, Clone)]
pub struct Bitset {
    bits: Vec<u64>,
}

impl Bitset {
    pub fn new() -> Self {
        Self { bits: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let len = (capacity + 63) / 64;
        Self { bits: vec![0; len] }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.bits.resize(self.bits.len() + additional, 0);
    }

    pub fn set(&mut self, index: usize) {
        let (word, bit) = (index / 64, index % 64);
        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }
        self.bits[word] |= 1 << bit;
    }

    pub fn clear(&mut self, index: usize) {
        let (word, bit) = (index / 64, index % 64);
        if word < self.bits.len() {
            self.bits[word] &= !(1 << bit);
        }
    }

    pub fn get(&self, index: usize) -> bool {
        let (word, bit) = (index / 64, index % 64);
        word < self.bits.len() && (self.bits[word] & (1 << bit)) != 0
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.iter().all(|index| other.get(index))
    }

    pub fn len(&self) -> usize {
        self.bits.len() * 64
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn clear_all(&mut self) {
        for word in &mut self.bits {
            *word = 0;
        }
    }

    pub fn reset(&mut self) {
        self.bits.clear();
    }

    pub fn iter(&self) -> BitsetIter {
        BitsetIter {
            bits: self,
            word: 0,
            bit: 0,
        }
    }
}

impl Default for Bitset {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BitsetIter<'a> {
    bits: &'a Bitset,
    word: usize,
    bit: usize,
}

impl<'a> Iterator for BitsetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.word < self.bits.bits.len() {
            let word = self.bits.bits[self.word];
            while self.bit < 64 {
                if (word & (1 << self.bit)) != 0 {
                    let index = self.word * 64 + self.bit;
                    self.bit += 1;
                    return Some(index);
                }
                self.bit += 1;
            }
            self.word += 1;
            self.bit = 0;
        }
        None
    }
}
