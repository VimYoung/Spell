use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    compositor::FrameCallbackData,
    reexports::{
        client::{
            QueueHandle,
            protocol::{wl_shm, wl_surface::WlSurface},
        },
        protocols::xdg::shell::client::xdg_surface::XdgSurface,
    },
    shell::xdg::popup::Popup,
    shm::slot::{Buffer, SlotPool},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use tracing::{info, warn};

use crate::{
    PopupSlint,
    configure::{PopupConf, PopupCore},
    slint_adapter::{ADAPTERS, SpellSkiaWinAdapter},
    wayland_adapter::{
        SpellWin,
        fractional_scaling::{
            FractionalScaleHandler, FractionalScaleState, delegate_fractional_scale,
        },
        viewporter::{ViewporterState, delegate_viewporter},
    },
};

pub(super) struct PopupManager {
    id_gen: u32,
    popups: HashMap<u32, Box<dyn PopupSlint>>,
    pool: Option<Rc<RefCell<SlotPool>>>,
}

impl PopupManager {
    pub(super) fn new() -> Self {
        PopupManager {
            id_gen: 0,
            popups: HashMap::new(),
            pool: None,
        }
    }

    pub(super) fn return_popup(&self, popup_inner: &Popup) -> Option<&dyn PopupSlint> {
        for popup in self.popups.values() {
            if popup_inner == popup.inner() {
                return Some(popup.as_ref());
            }
        }
        None
    }

    pub(super) fn set_pool(&mut self, pool: Rc<RefCell<SlotPool>>) {
        self.pool = Some(pool);
    }

    pub(super) fn create_popup_core(
        &mut self,
        popup: Popup,
        popup_conf: PopupConf,
        _fractional_scale_state: &FractionalScaleState,
        _viewporter_state: &ViewporterState,
        _qh: &QueueHandle<SpellWin>,
    ) -> PopupCore {
        // let fractional_scale = fractional_scale_state.get_scale(popup.wl_surface(), qh);
        // let viewport = viewporter_state.get_viewport(popup.wl_surface(), qh, fractional_scale);
        let stride = popup_conf.width as i32 * 4;
        let (buffer, _) = self
            .pool
            .as_ref()
            .unwrap()
            .borrow_mut()
            .create_buffer(
                popup_conf.width as i32,
                popup_conf.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("failed to create buffer for popup");
        // viewport.set_destination(popup_conf.width as i32, popup_conf.height as i32);
        // popup.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
        // popup
        //     .wl_surface()
        //     .damage(0, 0, popup_conf.width as i32, popup_conf.height as i32);
        // popup.xdg_surface().set_window_geometry(
        //     0,
        //     0,
        //     popup_conf.width as i32,
        //     popup_conf.height as i32,
        // );
        popup.wl_surface().commit();
        // popup.xdg_surface().config
        // popup.wl_surface().set_buffer_transform(
        //     smithay_client_toolkit::reexports::client::protocol::wl_output::Transform::Normal,
        // );
        // popup.wl_surface().set_buffer_scale(1);
        // popup.wl_surface().commit();

        PopupCore {
            pool: self.pool.as_ref().unwrap().clone(),
            popup,
            popup_conf,
            buffer,
            // viewport,
        }
    }

    pub(super) fn add_popup<T: PopupSlint + 'static>(&mut self, popup_instance: T) -> u32 {
        self.popups.insert(self.id_gen, Box::new(popup_instance));
        info!("[Popup Manager]: Popup added for rendering");
        self.id_gen = self.id_gen.wrapping_add(1);
        self.id_gen - 1
    }

    pub(super) fn redraw_popups(&self, qh: &QueueHandle<SpellWin>) {
        for popup in self.popups.values() {
            popup.converter_popup(popup.inner().wl_surface(), qh);
        }
    }

    pub(super) fn return_adapter(
        &self,
        surface: &WlSurface,
    ) -> Option<&std::rc::Rc<SpellSkiaWinAdapter>> {
        for popup in self.popups.values() {
            if popup.inner().wl_surface() == surface {
                return Some(popup.adapter());
            }
        }
        None
    }

    pub(super) fn call_ack(&self, xdg_surface: &XdgSurface, serial: u32) {
        for popup in self.popups.values() {
            if popup.inner().xdg_surface() == xdg_surface {
                popup.inner().xdg_surface().ack_configure(serial);
            }
        }
    }

    pub(super) fn close_popup(&mut self, id: &u32) {
        if let Some(rem_popup) = self.popups.remove(id) {
            rem_popup.inner().xdg_popup().destroy();
            info!("Removed Popup with id: {}", id);
        } else {
            warn!(
                "[PopupManager]: trying to remove a non-existant popup with id: {}",
                id
            );
        };
    }
}

/// This struct holds the backend information for creating and managing a XDG
/// popup in spell. It needs a [`PopupCore`] instance for initialisation
/// and it needs to be initialsed before the corresponding slint frontend. It is
/// better to wrap it in an external wrapper object along with frontend to satisfy
/// trait requirements of [`PopupSlint`]. For example, refer to popup example in
/// spell-demo.
pub struct SpellXDGPopup {
    adapter: Rc<SpellSkiaWinAdapter>,
    popup: Popup,
    first_configure: Cell<bool>,
    // viewport: Viewport,
}

delegate_fractional_scale!(SpellXDGPopup);
delegate_viewporter!(SpellXDGPopup);

impl SpellXDGPopup {
    /// Creates an instance provided [`PopupCore`].
    pub fn new(popup_settings: PopupCore) -> Self {
        let adapter_value: Rc<SpellSkiaWinAdapter> = SpellSkiaWinAdapter::new(
            popup_settings.pool,
            RefCell::new(popup_settings.buffer.slot()),
            popup_settings.popup_conf.width,
            popup_settings.popup_conf.height,
        );
        ADAPTERS.with_borrow_mut(|v| v.push(adapter_value.clone()));
        adapter_value
            .buffer
            .borrow_mut()
            .replace(popup_settings.buffer);
        SpellXDGPopup {
            adapter: adapter_value,
            popup: popup_settings.popup,
            first_configure: Cell::new(true),
            // viewport: popup_settings.viewport,
        }
    }

    /// Method necessary for a [`PopupSlint`] implementation.
    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    /// Method necessary for a [`PopupSlint`] implementation.
    pub fn first_configure(&self) -> bool {
        if self.first_configure.get() {
            self.first_configure.set(false);
            true
        } else {
            false
        }
    }

    /// Method necessary for a [`PopupSlint`] implementation.
    pub fn adapter(&self) -> &std::rc::Rc<SpellSkiaWinAdapter> {
        &self.adapter
    }

    /// Method necessary for a [`PopupSlint`] implementation.
    pub fn converter_popup<'a>(&self, wl_surface: &'a WlSurface, qh: &'a QueueHandle<SpellWin>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.adapter.as_ref().size.get().width;
        let height: u32 = self.adapter.as_ref().size.get().height;
        let window_adapter = self.adapter.clone();

        let redraw_val: bool = window_adapter.draw_if_needed();
        if self.first_configure.get() || redraw_val {
            // if self.first_configure {
            // self.first_configure.set(false);
            wl_surface.damage_buffer(0, 0, width as i32, height as i32);
            // } else {
            //     for (position, size) in self.damaged_part.as_ref().unwrap().iter() {
            //         // println!(
            //         //     "{}, {}, {}, {}",
            //         //     position.x, position.y, size.width as i32, size.height as i32,
            //         // );
            //         // if size.width != width && size.height != height {
            //         self.layer.wl_surface().damage_buffer(
            //             position.x,
            //             position.y,
            //             size.width as i32,
            //             size.height as i32,
            //         );
            //         //}
            //     }
            // }
            // Request our next frame
            if let Some(buffer) = self.adapter.buffer.borrow().as_ref() {
                wl_surface.attach(Some(buffer.wl_buffer()), 0, 0);
            }
            wl_surface.frame(qh, FrameCallbackData(wl_surface.clone()));
            wl_surface.commit();
        } else {
            wl_surface.commit();
        }
    }
}

impl FractionalScaleHandler for SpellXDGPopup {
    fn preferred_scale(
        &mut self,
        _: &smithay_client_toolkit::reexports::client::Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        scale: u32,
    ) {
        info!(
            "Scale factor of popup changed, invoked from custom trait: {}",
            scale
        );
        // FIXME: Make use of this for proper scaling implementation.
        let _width_old = self.adapter.size_original.get().width;
        let _height_old = self.adapter.size_original.get().height;
        self.popup.wl_surface().damage_buffer(
            0,
            0,
            self.adapter.size.get().width as i32,
            self.adapter.size.get().height as i32,
        );
        // FIXME: Make use of this for proper scaling implementation.
        let (_width, _height, scale_factor) = self.adapter.changed_scale_factor(scale);
        // self.width = width;
        // self.height = height;
        self.adapter
            .try_dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor })
            .unwrap();
        // self.viewport.set_source(
        //     0.,
        //     0.,
        //     self.adapter.size.get().width.into(),
        //     self.adapter.size.get().height.into(),
        // );
        //
        // self.viewport
        //     .set_destination(width_old as i32, height_old as i32);
        self.adapter.request_redraw();
    }
}
