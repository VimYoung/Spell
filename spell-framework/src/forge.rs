//! `forge` is a mini submodule which provides alternative method to create and run events
//! after a certain duration of time. Obvious approach to tackle such events is to use
//! [Timer](https://docs.slint.dev/latest/docs/slint/reference/timer/). Alternatively,
//! if you want a more rust facing interface (where timed events are not managed inside
//! `.slint` files and rather directly created in rust code), you can use
//! [`Forge`]
//!
//! ## Use Cases
//!
//! This module can easily be used in creating various timed events which are very common
//! while making shells. For example, it can be used for retriving:-
//! - Battery Percentage after certain time.
//! - Possible PowerProfiles and changes.
//! - CPU and Temperature Analytics etc.

use std::time::Duration;

use crate::wayland_adapter::SpellWin;
use smithay_client_toolkit::reexports::calloop::{
    LoopHandle,
    timer::{TimeoutAction, Timer},
};

/// An instance of Forge takes the LoopHandle of your window as input for
/// instance creation
pub struct Forge(LoopHandle<'static, SpellWin>);

impl Forge {
    // Create an instance on forge.
    pub fn new(handle: LoopHandle<'static, SpellWin>) -> Self {
        Forge(handle)
    }

    /// Add a recurring event. This function takes [`std::time::Duration`] to specify
    /// time after which it will be polled again with a callback to run. The callback accepts
    /// SpellWin instance of loop handle as argument for updating UI.
    pub fn add_event<F: FnMut(&mut SpellWin) + 'static>(
        &self,
        duration: Duration,
        mut callback: F,
    ) {
        self.0
            .insert_source(Timer::from_duration(duration), move |_, _, data| {
                callback(data);
                TimeoutAction::ToDuration(duration)
            })
            .unwrap();
    }
}
