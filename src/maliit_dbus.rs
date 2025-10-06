use std::time::Duration;
use std::sync::{Arc, Mutex};

use thiserror::Error;
use dbus::{
    blocking::{stdintf::org_freedesktop_dbus::Properties, SyncConnection, Proxy},
    channel::{Channel as DbusChannel, Token},
    Error as DbusError
};

use crate::events::{InputMethodEvent, Key};

const ADDRESS_DBUS_NAME: &str = "org.maliit.server";
const ADDRESS_PATH: &str = "/org/maliit/server/address";
const ADDRESS_INTERFACE: &str = "org.maliit.Server.Address";
const ADDRESS_PROPERTY: &str = "address";

const SERVER_DBUS_NAME: &str = "com.meego.inputmethod.uiserver1";
const SERVER_PATH: &str = "/com/meego/inputmethod/uiserver1";

const CONTEXT_INTERFACE: &str = "com.meego.inputmethod.inputcontext1";
const CONTEXT_PATH: &str = "/com/meego/inputmethod/inputcontext";


pub(crate) struct DbusMaliit {
    dbus_conn: SyncConnection,
}

impl DbusMaliit {
    pub fn new() -> Result<Self, DbusMaliitServerError> {
        let conn = dbus::blocking::Connection::new_session()?;
        let proxy = conn.with_proxy(ADDRESS_DBUS_NAME, ADDRESS_PATH, Duration::from_secs(5));
        let address: String = proxy.get(ADDRESS_INTERFACE, ADDRESS_PROPERTY)?;
        log::info!("Got Maliit Server dbus address: {}", address);
        let channel = DbusChannel::open_private(address.as_str())?;

        let dbus_conn = SyncConnection::from(channel);


        let obj = Self { dbus_conn };

        Ok(obj)
    }

    pub fn into_main_interfaces(self) -> (MaliitUiServer, MaliitContext) {
        let dbus_conn = Arc::new(self.dbus_conn);
        let ui_server = MaliitUiServer::new(dbus_conn.clone());
        let context = MaliitContext::new(dbus_conn.clone());

        (ui_server, context)
    }
}


pub(crate) struct MaliitUiServer {
    dbus_conn: Arc<SyncConnection>
}

impl MaliitUiServer {
    pub fn new(dbus_conn: Arc<SyncConnection>) -> Self {
        Self { dbus_conn }
    }

    fn proxy(&self) -> Proxy<'_, &SyncConnection> {
        self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5))
    }

    pub fn activate_context(&self) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "activateContext", ())?;
        Ok(())
    }

    pub fn reset(&self) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "reset", ())?;
        Ok(())
    }

    pub fn show_input_method(&mut self) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "showInputMethod", ())?;
        Ok(())
    }

    pub fn hide_input_method(&mut self) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "hideInputMethod", ())?;
        Ok(())
    }

    pub fn set_preedit(&mut self, preedit_text: &str, cursor_position: i32) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "setPreedit", (preedit_text, cursor_position))?;
        Ok(())
    }
}

pub(crate) struct MaliitContext {
    dbus_conn: Arc<SyncConnection>,
    token: Option<Token>,
    events: Arc<Mutex<Vec<InputMethodEvent>>>,
}

impl MaliitContext {
    pub fn new(dbus_conn: Arc<SyncConnection>) -> Self {
        Self {
            dbus_conn,
            token: None,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn proxy(&self) -> Proxy<'_, &SyncConnection> {
        self.dbus_conn.with_proxy(CONTEXT_INTERFACE, CONTEXT_PATH, Duration::from_secs(5))
    }

    pub fn stop_input_eventsprocessing(&mut self) -> Result<(), DbusMaliitServerError> {
        if let Some(token) = self.token.take() {
            self.proxy().match_stop(token, false)?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn start_input_events_processing(&mut self) -> Result<(), DbusMaliitServerError> {
        let events = self.events.clone();
        let token = self.proxy().match_start(dbus::message::MatchRule::new_method_call(), false, Box::new(move |msg, _conn|
            {
                log::debug!("Received dbus message: {:?}", msg);
                if let Some(member) = msg.member() {
                    match member.to_string().as_str() {
                        "commitString" => {
                            if let (Some(text), Some(x), Some(y), Some(z)) = msg.get4::<String, i32, i32, i32>() {
                                println!("Something with commitString: {}, {}, {}", x, y, z);
                                events.lock().unwrap().push(InputMethodEvent::Text(text));
                            }
                        },
                        "updateInputMethodArea" => {
                            if let (Some(x), Some(y), Some(width), Some(height)) = msg.get4::<i32, i32, i32, i32>() {
                                events.lock().unwrap().push(InputMethodEvent::AreaChanged(x, y, width, height));
                                return y != 0 // Drops callback if InputMethod hided
                            }
                        },
                        "keyEvent" => {
                            if let (Some(is_pressed), _, _, Some(key_str)) = msg.get4::<i32, i32, i32, String>() {
                                // Не знаю почему, но при нажатии на кнопку Enter или Backspace прилетают два события с разными первыми аргументами.
                                // Полагаю, что это события по "нажатию" и "отпусканию", но почему-то они приходят одновременно.
                                let key = match key_str.as_str() {
                                    "\r" => {
                                        Key::Enter
                                    },
                                    "\u{8}" => {
                                        Key::Backspace
                                    },
                                    _ => {
                                        log::warn!("Unkwnown key received, it wasn't be handled: {}.", key_str);
                                        return true
                                    }
                                };
                                events.lock().unwrap().push(InputMethodEvent::Key {
                                    key,
                                    pressed: is_pressed == 6
                                })
                            }
                        }
                        _ => { return true }
                    }
                }
                true
            }
        ))?;
        self.token = Some(token);
        Ok(())
    }

    pub fn get_new_events(&mut self) -> Option<Vec<InputMethodEvent>> {
        let mut mutex = match self.events.lock() {
            Ok(mutex) => mutex,
            Err(e) => {
                log::error!("Error with events mutex locking: {}", e);
                return None
            }
        };
        let events: &mut Vec<InputMethodEvent> = mutex.as_mut();
        Some(std::mem::take(events))
    }

    pub fn process_events(&self, timeout: Duration) -> Result<(), DbusMaliitServerError> {
        self.proxy().connection.process(timeout)?;
        Ok(())
    }

    pub fn update_preedit(
        &mut self,
        text: &str,
        preedit_formats: (i32, i32, i32),
        replacement_start: i32,
        replacement_length: i32,
        cursor_position: i32,
    ) -> Result<(), DbusMaliitServerError> {
        let method_args = (text, preedit_formats, replacement_start, replacement_length, cursor_position);
        let _: () = self.proxy().method_call(CONTEXT_INTERFACE, "updatePreedit", method_args)?;
        Ok(())
    }

    pub fn set_language(&mut self, language: &str) -> Result<(), DbusMaliitServerError> {
        let _: () = self.proxy().method_call(CONTEXT_INTERFACE, "setLanguage", (language,))?;
        Ok(())
    }
}


#[derive(Error, Debug)]
pub enum DbusMaliitServerError {
    #[error("Dbus error")]
    SessionConnectionFailed(#[from] DbusError),
}
