use crate::{PReaderState, types::config::PReaderIteratorConfig};

pub trait PReaderIterator {
    fn config(&self) -> &PReaderIteratorConfig;
    fn state(&mut self) -> &mut PReaderState;

    fn percent(&mut self) -> f64 {
        self.state().percent()
    }
}
