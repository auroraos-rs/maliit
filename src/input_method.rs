use std::time::Duration;

use crate::error::MaliitError;
use crate::events::InputMethodEvent;
use crate::maliit_dbus::{DbusMaliit, MaliitContext, MaliitUiServer};

pub struct InputMethod {
    ui_server: MaliitUiServer,
    context: MaliitContext,
}

impl InputMethod {
    pub fn new() -> Result<Self, MaliitError> {
        let dbus_maliit = DbusMaliit::new()?;
        let (ui_server, context) = dbus_maliit.into_main_interfaces();
        Ok(Self {
            ui_server,
            context,
        })
    }

    pub fn show(&mut self) -> Result<(), MaliitError> {
        self.ui_server.activate_context()?;
        self.ui_server.show_input_method()?;
        self.context.start_input_events_processing()?;
        Ok(())
    }

    pub fn hide(&mut self) -> Result<(), MaliitError> {
        self.ui_server.activate_context()?;
        self.ui_server.hide_input_method()?;
        self.context.stop_input_events_processing()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), MaliitError> {
        self.ui_server.reset()?;
        Ok(())
    }

    pub fn set_language(&mut self, lang: &str) -> Result<(), MaliitError> {
        self.context.set_language(lang)?;
        Ok(())
    }

    /// Poll pending events and invoke the handler for each.
    pub fn process_events_with<F>(
        &mut self,
        timeout: Duration,
        mut handler: F,
    ) -> Result<(), MaliitError>
    where
        F: FnMut(InputMethodEvent),
    {
        let events = self.poll_events(timeout)?;
        for event in events {
            handler(event);
        }
        Ok(())
    }

    /// Low-level: return pending events without a callback.
    pub fn poll_events(
        &mut self,
        timeout: Duration,
    ) -> Result<Vec<InputMethodEvent>, MaliitError> {
        self.context.process_events(timeout)?;
        Ok(self.context.get_new_events())
    }
}
