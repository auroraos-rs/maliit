use std::time::Duration;

use crate::maliit_dbus::DbusMaliit;
use crate::events::InputMethodEvent;

pub struct InputMethod {
    dbus_maliit: DbusMaliit
}

impl InputMethod {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dbus_maliit =  DbusMaliit::new()?;
        Ok(Self { dbus_maliit })
    }

    pub fn show(&mut self) {
        self.dbus_maliit.activate_context().unwrap();
        self.dbus_maliit.show_input_method().unwrap();
    }

    pub fn hide(&mut self) {
        self.dbus_maliit.activate_context().unwrap();
        self.dbus_maliit.hide_input_method().unwrap();
    }

    pub fn poll_new_events(&mut self, timeout: Duration) -> Option<Vec<InputMethodEvent>> {
        self.dbus_maliit.process_events(timeout).unwrap();
        self.dbus_maliit.get_new_events()
    }

    pub fn reset(&mut self) {
        self.dbus_maliit.reset().unwrap()
    }
}
