use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::error::MaliitError;
use crate::events::{InputMethodEvent, Orientation};
use crate::maliit_dbus::{DbusMaliit, MaliitContext, MaliitUiServer};

pub struct InputMethod {
    ui_server: MaliitUiServer,
    context: MaliitContext,
    event_thread: Option<EventThread>,
}

struct EventThread {
    command_sender: mpsc::Sender<EventCommand>,
    join_handle: JoinHandle<()>,
}

enum EventCommand {
    AddHandler(Box<dyn Fn(InputMethodEvent) + Send>),
    Stop,
}

impl InputMethod {
    pub fn new() -> Result<Self, MaliitError> {
        let dbus_maliit = DbusMaliit::new()?;
        let (ui_server, context) = dbus_maliit.into_main_interfaces();
        Ok(Self {
            ui_server,
            context,
            event_thread: None,
        })
    }

    pub fn show(&mut self) -> Result<(), MaliitError> {
        self.ui_server.activate_context()?;
        self.ui_server.show_input_method()?;
        Ok(())
    }

    pub fn hide(&mut self) -> Result<(), MaliitError> {
        self.ui_server.activate_context()?;
        self.ui_server.hide_input_method()?;
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

    pub fn rotate(&mut self, orientation: Orientation) -> Result<(), MaliitError> {
        let angle = orientation as i32;
        self.ui_server.activate_context()?;
        self.ui_server.app_orientation_about_to_change(angle)?;
        self.ui_server.app_orientation_changed(angle)?;
        Ok(())
    }

    /// Register a callback that will be called for every IME event.
    ///
    /// The first call spawns a background thread that polls D-Bus.
    /// Subsequent calls add more handlers. All handlers receive the same event.
    pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), MaliitError>
    where
        F: Fn(InputMethodEvent) + Send + 'static,
    {
        if self.event_thread.is_none() {
            let dbus_conn = self.context.dbus_conn();
            let (cmd_tx, cmd_rx) = mpsc::channel();
            let (startup_tx, startup_rx) = mpsc::channel();

            let handle = std::thread::spawn(move || {
                let mut context = MaliitContext::new(dbus_conn);

                if let Err(e) = context.start_input_events_processing() {
                    let _ = startup_tx.send(Err(e));
                    return;
                }

                if startup_tx.send(Ok(())).is_err() {
                    let _ = context.stop_input_events_processing();
                    return;
                }

                let mut handlers: Vec<Box<dyn Fn(InputMethodEvent) + Send>> = Vec::new();

                loop {
                    match cmd_rx.try_recv() {
                        Ok(EventCommand::AddHandler(h)) => handlers.push(h),
                        Ok(EventCommand::Stop) => break,
                        Err(mpsc::TryRecvError::Disconnected) => break,
                        Err(mpsc::TryRecvError::Empty) => {}
                    }

                    while context.process_events(Duration::from_secs(0)).unwrap_or(false) {
                        for event in context.get_new_events() {
                            for handler in &handlers {
                                handler(event.clone());
                            }
                        }
                    }

                    std::thread::sleep(Duration::from_millis(10));
                }

                if let Err(e) = context.stop_input_events_processing() {
                    log::error!("Failed to stop input events processing in event thread: {}", e);
                }
            });

            match startup_rx.recv() {
                Ok(Ok(())) => {
                    self.event_thread = Some(EventThread {
                        command_sender: cmd_tx,
                        join_handle: handle,
                    });
                }
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(MaliitError::NotAvailable),
            }
        }

        let thread = self.event_thread.as_ref().unwrap();
        thread
            .command_sender
            .send(EventCommand::AddHandler(Box::new(handler)))
            .map_err(|_| MaliitError::NotAvailable)?;

        Ok(())
    }

    /// Remove all event handlers and stop the background thread.
    pub fn clear_event_handlers(&mut self) {
        if let Some(thread) = self.event_thread.take() {
            let _ = thread.command_sender.send(EventCommand::Stop);
            if let Err(e) = thread.join_handle.join() {
                log::error!("Event thread panicked: {:?}", e);
            }
        }
    }
}
