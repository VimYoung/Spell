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

/// Future implementation of Forge for timed events will happen here.
pub struct Forge;
