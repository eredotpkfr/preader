use preader::PathError;

#[derive(Clone, Copy, Debug)]
pub enum Rule {
    Empty,
    Invalid,
    Escapes,
}

impl Rule {
    pub fn matches(self, error: &PathError, name: &str) -> bool {
        match (self, error) {
            (Self::Empty, PathError::Empty) => true,
            (Self::Invalid, PathError::Invalid(found))
            | (Self::Escapes, PathError::Escapes(found)) => found == name,
            _ => false,
        }
    }
}
