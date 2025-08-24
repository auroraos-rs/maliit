use std::time::Duration;

use crate::maliit_dbus::{DbusMaliit, MaliitContext, MaliitUiServer};
use crate::events::InputMethodEvent;

pub struct InputMethod {
    ui_server: MaliitUiServer,
    context: MaliitContext,
}

impl InputMethod {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dbus_maliit =  DbusMaliit::new()?;
        let (ui_server, context) = dbus_maliit.into_main_interfaces();
        Ok(Self { ui_server, context })
    }

    pub fn show(&mut self) {
        self.ui_server.activate_context().unwrap();
        self.ui_server.show_input_method().unwrap();
        self.ui_server.set_preedit("hell", 0).unwrap();
        self.context.start_input_events_processing().unwrap();
    }

    pub fn hide(&mut self) {
        self.ui_server.activate_context().unwrap();
        self.ui_server.hide_input_method().unwrap();
        self.context.stop_input_eventsprocessing().unwrap();
    }

    pub fn poll_new_events(&mut self, timeout: Duration) -> Option<Vec<InputMethodEvent>> {
        self.context.process_events(timeout).unwrap();
        self.context.get_new_events()
    }

    pub fn reset(&mut self) {
        self.ui_server.reset().unwrap()
    }
}
