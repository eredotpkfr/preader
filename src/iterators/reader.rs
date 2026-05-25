use crate::{PReaderState, types::config::PReaderIteratorConfig};

pub trait PReaderIterator {
    fn config(&self) -> &PReaderIteratorConfig;
    fn state(&self) -> PReaderState;
}

impl dyn PReaderIterator {
    pub fn percent(&self) -> f64 {
        self.state().percent()
    }
}
