//! The number reader LesIniNumeri (0x100079e0) installs.

use super::super::common::Tok;
use super::Portuguese;

impl Portuguese {
    pub(super) fn leggi_numero(&self, _toks: &mut [Tok], _i: usize, _kind: u32, _dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        None
    }
}
