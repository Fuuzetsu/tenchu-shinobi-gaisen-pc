use std::{cell::RefCell, rc::Rc};
use tenchu_assets::{ReadAt, Result, cache::Cache};

#[derive(Clone)]
pub struct DiscSource(pub Rc<RefCell<Cache>>);

impl ReadAt for DiscSource {
    fn byte_len(&self) -> u64 {
        self.0.borrow().byte_len()
    }
    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        self.0.borrow().read_exact_at(offset, output)
    }
}
