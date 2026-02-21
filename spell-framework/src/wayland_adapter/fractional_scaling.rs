// Courtesy DrepDays
// Implementaion is taken and modified from here.
// https://github.com/DerpDays/draw/blob/main/platform%2Fwayland%2Fsrc%2Ffractional_scale.rs
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::reexports::{
    client::{
        Connection, Dispatch, Proxy, QueueHandle,
        globals::{BindError, GlobalList},
        protocol::wl_surface::WlSurface,
    },
    protocols::wp::fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{Event::PreferredScale, WpFractionalScaleV1},
    },
};

#[derive(Debug)]
#[allow(unused)]
pub struct FractionalScaleState {
    manager: WpFractionalScaleManagerV1,
}

/// An owned instance of WpFractionalScaleV1, when this is dropped, the underlying interface is
/// destroyed, and hence events for the corresponding surface are no longer emitted.
#[derive(Debug)]
pub struct FractionalScale {
    fractional_scale: WpFractionalScaleV1,
}

impl FractionalScaleState {
    pub(crate) fn bind<State>(
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<WpFractionalScaleManagerV1, GlobalData, State>
            + FractionalScaleHandler
            + 'static,
    {
        let manager = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(FractionalScaleState { manager })
    }

    pub(crate) fn get_scale<State>(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<State>,
    ) -> FractionalScale
    where
        State: Dispatch<WpFractionalScaleV1, WlSurface> + 'static,
    {
        FractionalScale {
            fractional_scale: self.manager.get_fractional_scale(
                surface,
                queue_handle,
                surface.clone(),
            ),
        }
    }
}

impl Drop for FractionalScale {
    fn drop(&mut self) {
        self.fractional_scale.destroy();
    }
}

impl<D> Dispatch<WpFractionalScaleManagerV1, GlobalData, D> for FractionalScaleState
where
    D: Dispatch<WpFractionalScaleManagerV1, GlobalData> + FractionalScaleHandler + 'static,
{
    fn event(
        _: &mut D,
        _: &WpFractionalScaleManagerV1,
        _: <WpFractionalScaleManagerV1 as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("WpFractionalScaleManagerV1 has no events")
    }
}

impl<D> Dispatch<WpFractionalScaleV1, WlSurface, D> for FractionalScaleState
where
    D: Dispatch<WpFractionalScaleV1, WlSurface> + FractionalScaleHandler + 'static,
{
    fn event(
        state: &mut D,
        _: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        surface: &WlSurface,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            PreferredScale { scale } => {
                state.preferred_scale(conn, qh, surface, scale);
            }
            _ => unreachable!("WpFractionalScaleV1 should only have a preferred_scale event"),
        }
    }
}

pub trait FractionalScaleHandler: Sized {
    /// When this function is called, the compositor is indicating the preferred fractional scale
    /// for the given surface.
    ///
    /// The scale received in this event has a denominator of 120, so the true fractional scale
    /// would be (scale.to_f64().unwrap() / 120.)
    fn preferred_scale(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        scale: u32,
    );
}

macro_rules! delegate_fractional_scale {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
            smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1: smithay_client_toolkit::globals::GlobalData
        ] => $crate::wayland_adapter::fractional_scaling::FractionalScaleState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay_client_toolkit::reexports::protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1: smithay_client_toolkit::reexports::client::protocol::wl_surface::WlSurface
        ] => $crate::wayland_adapter::fractional_scaling::FractionalScaleState);
    };
}
pub(crate) use delegate_fractional_scale;
