use slint::platform::WindowAdapter;
use smithay_client_toolkit::{
    reexports::{
        client::{
            QueueHandle,
            protocol::{wl_shm, wl_surface::WlSurface},
        },
        protocols::{wp::viewporter, xdg::shell::client::xdg_surface::XdgSurface},
    },
    shell::xdg::{XdgShellSurface, popup::Popup},
    shm::slot::SlotPool,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tracing::info;

use crate::{
    PopupSlint,
    configure::{PopupConf, PopupSettings},
    slint_adapter::{ADAPTERS, SpellSkiaWinAdapter},
    wayland_adapter::{
        SpellWin, SpellXDGPopup,
        fractional_scaling::{FractionalScaleHandler, FractionalScaleState},
        viewporter::{Viewport, ViewporterState},
        widget_impls::popup_impl,
    },
};

pub(crate) struct PopupManager {
    popups: Vec<Box<dyn PopupSlint>>,
    pool: Option<Rc<RefCell<SlotPool>>>,
}

impl PopupManager {
    pub(crate) fn new() -> Self {
        PopupManager {
            popups: Vec::new(),
            pool: None,
        }
    }

    pub(crate) fn return_popup(&self, popup_inner: &Popup) -> Option<&dyn PopupSlint> {
        for popup in self.popups.iter() {
            if popup_inner == popup.inner() {
                return Some(popup.as_ref());
            }
        }
        None
    }

    pub(crate) fn set_pool(&mut self, pool: Rc<RefCell<SlotPool>>) {
        self.pool = Some(pool);
    }

    pub(crate) fn create_popup<T: PopupSlint + 'static>(
        &mut self,
        popup: Popup,
        popup_conf: PopupConf,
        fractional_scale_state: &FractionalScaleState,
        viewporter_state: &ViewporterState,
        qh: &QueueHandle<SpellWin>,
    ) {
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

        let popup = T::create_new(PopupSettings {
            pool: self.pool.as_ref().unwrap().clone(),
            popup,
            popup_conf,
            buffer,
            // viewport,
        });
        self.popups.push(Box::new(popup));
    }

    pub(crate) fn redraw_popups(&self, qh: &QueueHandle<SpellWin>) {
        for popup in self.popups.iter() {
            popup.converter_popup(popup.inner().wl_surface(), qh);
        }
    }

    pub(crate) fn return_adapter(
        &self,
        surface: &WlSurface,
    ) -> Option<&std::rc::Rc<SpellSkiaWinAdapter>> {
        for popup in self.popups.iter() {
            if popup.inner().wl_surface() == surface {
                return Some(popup.adapter());
            }
        }
        None
    }

    pub(crate) fn call_ack(&self, xdg_surface: &XdgSurface, serial: u32) {
        for popup in self.popups.iter() {
            if popup.inner().xdg_surface() == xdg_surface {
                popup.inner().xdg_surface().ack_configure(serial);
            }
        }
    }
}

impl SpellXDGPopup {
    pub fn new(popup_settings: PopupSettings) -> Self {
        let adapter_value: Rc<SpellSkiaWinAdapter> = SpellSkiaWinAdapter::new(
            popup_settings.pool,
            RefCell::new(popup_settings.buffer.slot()),
            popup_settings.popup_conf.width,
            popup_settings.popup_conf.height,
        );
        ADAPTERS.with_borrow_mut(|v| v.push(adapter_value.clone()));
        SpellXDGPopup {
            adapter: adapter_value,
            popup: popup_settings.popup,
            buffer: popup_settings.buffer,
            first_configure: Cell::new(true),
            // viewport: popup_settings.viewport,
        }
    }

    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    pub fn first_configure(&self) -> bool {
        if self.first_configure.get() {
            self.first_configure.set(false);
            true
        } else {
            false
        }
    }

    pub fn adapter(&self) -> &std::rc::Rc<SpellSkiaWinAdapter> {
        &self.adapter
    }

    pub fn converter_popup<'a>(&self, wl_surface: &'a WlSurface, qh: &'a QueueHandle<SpellWin>) {
        slint::platform::update_timers_and_animations();
        let width: u32 = self.adapter.as_ref().size.get().width;
        let height: u32 = self.adapter.as_ref().size.get().height;
        let window_adapter = self.adapter.clone();

        let redraw_val: bool = window_adapter.draw_if_needed();
        let buffer = &self.buffer;
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
            wl_surface.attach(Some(buffer.wl_buffer()), 0, 0);
            wl_surface.frame(qh, wl_surface.clone());
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
        let width_old = self.adapter.size_original.get().width;
        let height_old = self.adapter.size_original.get().height;
        self.popup.wl_surface().damage_buffer(
            0,
            0,
            self.adapter.size.get().width as i32,
            self.adapter.size.get().height as i32,
        );
        let (buffer, width, height, scale_factor) = self.adapter.changed_scale_factor(scale);
        // self.width = width;
        // self.height = height;
        self.buffer = buffer;
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
        self.popup.wl_surface().commit();
    }
}
