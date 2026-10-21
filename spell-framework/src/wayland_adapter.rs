//! It provides various widget types for implementing properties
//! across various functionalities for your shell. The most common widget (or
//! window as called by many) is [SpellWin]. You can also implement a lock screen
//! with [`SpellLock`].

mod common;
mod fractional_scaling;
mod lock;
pub(crate) mod viewporter;
mod window;

pub use window::SpellWin;
pub use window::SpellXDGPopup;
pub use window::WinHandle;

pub use lock::LockHandle;
pub use lock::SpellLock;

/// Furture virtual keyboard implementation will be on this type. Currently, it is redundent.
pub struct SpellBoard;
