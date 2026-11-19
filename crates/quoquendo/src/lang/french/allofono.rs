//! `FonAllofonoLingua` (0x1000e690)
use super::French;
use crate::lang::common::{Phone, Word};
impl French {
    pub(crate) fn allofono(&self, _w: &[Word], _p: &mut Vec<Phone>, i: usize) -> usize { i }
}
