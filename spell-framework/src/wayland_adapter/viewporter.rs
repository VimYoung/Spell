// Courtesy DrepDays
// Implementaion is taken and modified from here.
// https://github.com/DerpDays/draw/blob/main/platform%2Fwayland%2Fsrc%2Fviewporter.rs
use crate::wayland_adapter::fractional_scaling::FractionalScale;
use smithay_client_toolkit::{
    globals::GlobalData,
    reexports::{
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            globals::{BindError, GlobalList},
            protocol::wl_surface::WlSurface,
        },
        protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    },
};

#[derive(Debug)]
pub struct ViewporterState {
    viewporter: WpViewporter,
}

/// An owned instance of WpViewport, when this is dropped, the underlying interface is
/// destroyed.
#[derive(Debug)]
pub struct Viewport {
    viewport: WpViewport,
    // This is not required but yet stored so that it doesn't get distroyed.
    fractional_scale: FractionalScale,
}

impl ViewporterState {
    pub fn bind<State>(
        globals: &GlobalList,
        queue_handle: &QueueHandle<State>,
    ) -> Result<Self, BindError>
    where
        State: Dispatch<WpViewporter, GlobalData, State> + 'static,
    {
        let viewporter = globals.bind(queue_handle, 1..=1, GlobalData)?;
        Ok(ViewporterState { viewporter })
    }

    pub fn get_viewport<State>(
        &self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<State>,
        fractional_scale: FractionalScale,
    ) -> Viewport
    where
        State: Dispatch<WpViewport, GlobalData> + 'static,
    {
        Viewport {
            viewport: self
                .viewporter
                .get_viewport(surface, queue_handle, GlobalData),
            fractional_scale,
        }
    }
}

impl Viewport {
    pub fn set_source(&self, x: f64, y: f64, width: f64, height: f64) {
        self.viewport.set_source(x, y, width, height);
    }
    pub fn set_destination(&self, width: i32, height: i32) {
        self.viewport.set_destination(width, height);
    }
}

impl Drop for Viewport {
    fn drop(&mut self) {
        self.viewport.destroy();
    }
}

impl<D> Dispatch<WpViewporter, GlobalData, D> for ViewporterState
where
    D: Dispatch<WpViewporter, GlobalData> + 'static,
{
    fn event(
        _: &mut D,
        _: &WpViewporter,
        _: <WpViewporter as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("WpViewporter has no events")
    }
}

impl<D> Dispatch<WpViewport, GlobalData, D> for ViewporterState
where
    D: Dispatch<WpViewport, GlobalData> + 'static,
{
    fn event(
        _: &mut D,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!("WpViewport has no events")
    }
}

#[macro_export]
macro_rules! delegate_viewporter {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport: smithay_client_toolkit::globals::GlobalData
        ] => $crate::wayland_adapter::viewporter::ViewporterState);
        smithay_client_toolkit::reexports::client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter: smithay_client_toolkit::globals::GlobalData
        ] => $crate::wayland_adapter::viewporter::ViewporterState);
    };
}
