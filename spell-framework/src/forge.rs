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

use smithay_client_toolkit::reexports::calloop::{
    EventLoop,
    timer::{TimeoutAction, Timer},
};

struct ForgeState;

/// An instance of Forge takes the LoopHandle of your window as input for
/// instance creation. It is currently not usable because of latency issues.
pub struct Forge(EventLoop<'static, ForgeState>);
impl Default for Forge {
    /// Create an instance on forge.
    fn default() -> Self {
        let event_loop: EventLoop<'static, ForgeState> =
            EventLoop::try_new().expect("Failed to initialize the event loop!");
        Forge(event_loop)
    }
}
impl Forge {
    // // Create an instance on forge.
    // pub fn new(handle: WinHandle) -> Self {
    //     Forge(handle)
    // }

    /// Add a recurring event. This function takes [`std::time::Duration`] to specify
    /// time after which it will be polled again with a callback to run. The callback accepts
    /// SpellWin instance of loop handle as argument for updating UI.
    pub fn add_event<F: FnMut() + Send + 'static>(&self, duration: Duration, mut callback: F) {
        self.0
            .handle()
            .insert_source(Timer::from_duration(duration), move |_, _, _| {
                callback();
                TimeoutAction::ToDuration(duration)
            })
            .unwrap();
    }

    // pub fn smith(&mut self) -> std::thread::JoinHandle<_> {
    //     let data = ForgeState;
    //     std::thread::spawn(move || {
    //         loop {
    //             self.0.dispatch(Duration::from_secs(1), &mut data);
    //         }
    //     })
    // }
}
