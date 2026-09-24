pub struct Rng(u64);

impl Rng {
    pub fn from_phrase(phrase: &str, generation: u64) -> Self {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in phrase.bytes().chain(generation.to_le_bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self(hash.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn range(&mut self, end: usize) -> usize {
        (self.next() % end as u64) as usize
    }
}
