pub trait Item {
    fn bytes(self) -> Vec<u8>;
}

impl Item for u8 {
    fn bytes(self) -> Vec<u8> {
        vec![self]
    }
}

impl Item for Vec<u8> {
    fn bytes(self) -> Vec<u8> {
        self
    }
}

impl Item for String {
    fn bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}
