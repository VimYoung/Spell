use crate::wayland_adapter::{fractional_scaling::FractionalScaleHandler, window::SpellWin};
use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, Dispatch, QueueHandle,
            protocol::{wl_output, wl_seat, wl_surface},
        },
        protocols::xdg::shell::client::xdg_surface::XdgSurface,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState, pointer::PointerData},
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        xdg::{popup::PopupHandler, window::WindowHandler},
    },
    shm::{Shm, ShmHandler},
};
use tracing::{info, trace, warn};

impl WindowHandler for SpellWin {
    fn request_close(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::window::Window,
    ) {
        todo!()
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::window::Window,
        _: smithay_client_toolkit::shell::xdg::window::WindowConfigure,
        _: u32,
    ) {
        todo!()
    }
}

impl SeatHandler for SpellWin {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.states.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.states.keyboard_state.is_none() {
            info!("Setting keyboard capability");
            let keyboard = self
                .states
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.states.keyboard_state = Some(keyboard);
        }
        if capability == Capability::Touch && self.states.touch_state.is_none() {
            info!("Setting touch Capability");
            let touch = self
                .states
                .seat_state
                .get_touch(qh, &seat)
                .expect("Failed to create touch");
            self.states.touch_state = Some(touch);
        }
        if capability == Capability::Pointer && self.states.pointer_state.pointer.is_none() {
            info!("Setting pointer capability");
            let pointer = self
                .states
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            let pointer_data = PointerData::new(seat);
            self.states.pointer_state.pointer = Some(pointer);
            self.states.pointer_state.pointer_data = Some(pointer_data);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.states.keyboard_state.is_some() {
            info!("Unsetting keyboard capability");
            self.states.keyboard_state.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.states.pointer_state.pointer.is_some() {
            info!("Unsetting pointer capability");
            self.states.pointer_state.pointer.take().unwrap().release();
        }
        if capability == Capability::Touch && self.states.touch_state.is_some() {
            info!("Unsetting pointer capability");
            self.states.touch_state.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PopupHandler for SpellWin {
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &smithay_client_toolkit::shell::xdg::popup::Popup,
        _: smithay_client_toolkit::shell::xdg::popup::PopupConfigure,
    ) {
        let x = self.popup_manager.return_popup(popup);
        if let Some(current_popup) = x {
            // FIXME: Is this commit required?
            current_popup.inner().wl_surface().commit();
            if current_popup.first_configure() {
                current_popup.converter_popup(current_popup.inner().wl_surface(), &self.queue);
            }
        } else {
            warn!("Popup configured but not pushed to the manager");
        }
    }

    fn done(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &smithay_client_toolkit::shell::xdg::popup::Popup,
    ) {
        info!("[Popup Manager]: A popup is closed");
    }
}

// TODO: FIND What is the use of registery_handlers here?
impl ProvidesRegistryState for SpellWin {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.states.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for SpellWin {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.states.shm
    }
}

impl OutputHandler for SpellWin {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.states.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        trace!("New output Source Added");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        trace!("Existing output is updated");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        trace!("Output is destroyed");
    }
}

impl CompositorHandler for SpellWin {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
        info!("Scale factor changed, compositor msg");
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        trace!("Compositor transformation changed");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.converter(qh);
        self.popup_manager.redraw_popups(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!("Surface entered");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!("Surface left");
    }
}

impl FractionalScaleHandler for SpellWin {
    fn preferred_scale(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        scale: u32,
    ) {
        info!("Scale factor changed, invoked from custom trait: {}", scale);
        let width_old = self.adapter.as_ref().unwrap().size_original.get().width;
        let height_old = self.adapter.as_ref().unwrap().size_original.get().height;
        self.layer.as_ref().unwrap().wl_surface().damage_buffer(
            0,
            0,
            self.adapter.as_ref().unwrap().size.get().width as i32,
            self.adapter.as_ref().unwrap().size.get().height as i32,
        );
        let (buffer, width, height, scale_factor) =
            self.adapter.as_ref().unwrap().changed_scale_factor(scale);
        self.config.evaluated_width = width;
        self.config.evaluated_height = height;
        self.buffer = Some(buffer);
        self.adapter
            .as_ref()
            .unwrap()
            .try_dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor })
            .unwrap();
        self.viewport.as_ref().unwrap().set_source(
            0.,
            0.,
            self.adapter.as_ref().unwrap().size.get().width.into(),
            self.adapter.as_ref().unwrap().size.get().height.into(),
        );

        self.viewport
            .as_ref()
            .unwrap()
            .set_destination(width_old as i32, height_old as i32);
        self.adapter.as_ref().unwrap().request_redraw();
        self.layer.as_ref().unwrap().commit();
    }
}

impl LayerShellHandler for SpellWin {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        trace!("Closure of layer called");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.converter(qh);
    }
}

impl Dispatch<XdgSurface, ()> for SpellWin {
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: <XdgSurface as smithay_client_toolkit::reexports::client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_surface::Event::Configure { serial } => {
                info!("[Popup Manager]: ack called with a serial");
                 state.popup_manager.call_ack(xdg_surface, serial);
            },
            event_branch => warn!("Unprocessed event branch from popup configure: {:?}", event_branch),
        }
    }
}
