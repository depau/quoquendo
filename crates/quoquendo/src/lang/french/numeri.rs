//! Numbers (`LesIniNumeri` 0x10006480)
use super::French;
pub(crate) fn number_words(_f: &French, digits: &str) -> Vec<Vec<u8>> { vec![digits.as_bytes().to_vec()] }
