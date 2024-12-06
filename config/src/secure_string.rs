use alloc::string::String;
use serde::{Serialize, Deserialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Zeroize, ZeroizeOnDrop, Serialize, Deserialize, Default)]
pub struct SecureString(String);

impl SecureString {
    pub fn map(&self, f: impl FnOnce(&str) -> String) -> Self {
        Self(f(self.0.as_str()))
    }
    /// Get a reference to the insecure inner string. Prefer `map` for constructing new `SecureString`s
    /// since it avoids creating intermediate non-zeroed-on-drop `String` copies.
    pub fn insecure(&self) -> &str {
        self.0.as_str()
    }
}
impl core::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("\"<REDACTED>\"")
    }
}
