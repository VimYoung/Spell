use smithay_client_toolkit::{
    reexports::{
        client::{QueueHandle, protocol::wl_shm},
        protocols::xdg::shell::client::{xdg_positioner::XdgPositioner, xdg_surface::XdgSurface},
    },
    shell::xdg::popup::Popup,
    shm::slot::{Buffer, SlotPool},
};
use std::{cell::RefCell, rc::Rc};

use crate::{
    PopupSlint,
    configure::{PopupConf, PopupSettings},
    slint_adapter::{ADAPTERS, SpellSkiaWinAdapter},
    wayland_adapter::SpellXDGPopup,
};

pub(crate) struct PopupManager {
    popups: Vec<Box<dyn PopupSlint>>,
    xdg_surface: Option<XdgSurface>,
    pool: Option<Rc<RefCell<SlotPool>>>,
}

impl PopupManager {
    pub(crate) fn new() -> Self {
        PopupManager {
            popups: Vec::new(),
            xdg_surface: None,
            pool: None,
        }
    }

    pub(crate) fn return_popup(&mut self, popup_inner: &Popup) -> Option<&mut dyn PopupSlint> {
        for popup in self.popups.iter_mut() {
            if popup_inner == popup.inner() {
                return Some(popup.as_mut());
            }
        }
        None
    }

    pub(crate) fn set_pool(&mut self, pool: Rc<RefCell<SlotPool>>) {
        self.pool = Some(pool);
    }
    pub(crate) fn xdg_surface(&self) -> &XdgSurface {
        self.xdg_surface.as_ref().unwrap()
    }

    pub(crate) fn set_surface(&mut self, surface: XdgSurface) {
        self.xdg_surface = Some(surface);
    }
    pub(crate) fn create_popup<T: PopupSlint + 'static>(
        &mut self,
        popup: Popup,
        popup_conf: PopupConf,
    ) {
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
        let popup = T::create_new(PopupSettings {
            pool: self.pool.as_ref().unwrap().clone(),
            popup,
            popup_conf,
            buffer,
        });
        // let popup = SpellXDGPopup::new(
        //     self.pool.as_ref().unwrap().clone(),
        //     popup,
        //     popup_conf,
        //     buffer,
        // );
        self.popups.push(Box::new(popup));
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
            // frontend: Box::new(T::create_new()),
            adapter: adapter_value,
            // evaluated_width: popup_conf.width,
            // evaluated_height: popup_conf.height,
            popup: popup_settings.popup,
            buffer: popup_settings.buffer,
        }
    }

    pub fn popup(&self) -> &Popup {
        &self.popup
    }

    pub fn converter_popup(&mut self) {
        slint::platform::update_timers_and_animations();
        // let width: u32 = self.adapter.as_ref().size.get().width;
        // let height: u32 = self.adapter.as_ref().size.get().height;
        let window_adapter = self.adapter.clone();

        // let skia_now = std::time::Instant::now();
        let redraw_val: bool = window_adapter.draw_if_needed();
        // let elasped_time = skia_now.elapsed().as_millis();
        // if elasped_time != 0 {
        //     debug!("Skia Elapsed Time: {}", skia_now.elapsed().as_millis());
        // }

        // self.states
        //     .pointer_state
        //     .update_cursor(self.adapter.as_ref().current_cursor.get(), &qh);

        // let buffer = &self.buffer;
        // if
        // /*self.first_configure.get() ||*/
        // redraw_val {
        //     // if self.first_configure {
        //     // self.first_configure.set(false);
        //     self.layer.as_ref().unwrap().wl_surface().damage_buffer(
        //         0,
        //         0,
        //         width as i32,
        //         height as i32,
        //     );
        //     // } else {
        //     //     for (position, size) in self.damaged_part.as_ref().unwrap().iter() {
        //     //         // println!(
        //     //         //     "{}, {}, {}, {}",
        //     //         //     position.x, position.y, size.width as i32, size.height as i32,
        //     //         // );
        //     //         // if size.width != width && size.height != height {
        //     //         self.layer.wl_surface().damage_buffer(
        //     //             position.x,
        //     //             position.y,
        //     //             size.width as i32,
        //     //             size.height as i32,
        //     //         );
        //     //         //}
        //     //     }
        //     // }
        //     // Request our next frame
        //     self.layer.as_ref().unwrap().wl_surface().attach(
        //         Some(buffer.as_ref().unwrap().wl_buffer()),
        //         0,
        //         0,
        //     );
        //
        //     self.layer
        //         .as_ref()
        //         .unwrap()
        //         .wl_surface()
        //         .frame(qh, self.layer.as_ref().unwrap().wl_surface().clone());
        //     self.layer.as_ref().unwrap().commit();
        // } else {
        //     self.layer.as_ref().unwrap().commit();
        // }
    }
}
