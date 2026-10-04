use crate::common::constants::TEST_UNICODE_TEXT;

const WORDS_PER_LINE: u64 = 6;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub fn seeded_bytes(seed: u64, size: usize) -> Vec<u8> {
    let mut rng = Rng::new(seed);

    (0..size).map(|_| (rng.next() & 0xFF) as u8).collect()
}

pub fn seeded_text(seed: u64, size: usize) -> String {
    let words: Vec<&str> =
        TEST_UNICODE_TEXT.split(' ').chain(["preader", "state", "a;b", ","]).collect();
    let mut rng = Rng::new(seed);
    let mut text = String::new();

    while text.len() < size {
        let count = rng.next() % WORDS_PER_LINE;

        for word in 0..count {
            if word > 0 {
                text.push(' ');
            }

            text.push_str(words[(rng.next() % words.len() as u64) as usize]);
        }

        text.push('\n');
    }

    text
}
