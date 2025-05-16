use std::time::Duration;
use std::sync::{Arc, Mutex};

use thiserror::Error;
use dbus::{
    blocking::{stdintf::org_freedesktop_dbus::Properties, SyncConnection},
    channel::{Channel as DbusChannel, MatchingReceiver, Token},
    Error as DbusError
};

use crate::events::{InputMethodEvent, Key};

const ADDRESS_DBUS_NAME: &str = "org.maliit.server";
const ADDRESS_PATH: &str = "/org/maliit/server/address";
const ADDRESS_INTERFACE: &str = "org.maliit.Server.Address";
const ADDRESS_PROPERTY: &str = "address";

const SERVER_DBUS_NAME: &str = "com.meego.inputmethod.uiserver1";
const SERVER_PATH: &str = "/com/meego/inputmethod/uiserver1";

const _CONTEXT_INTERFACE: &str = "com.meego.inputmethod.inputcontext1";
const _CONTEXT_PATH: &str = "/com/meego/inputmethod/inputcontext";


pub(crate) struct DbusMaliit {
    dbus_conn: Arc<SyncConnection>,
    token: Option<Token>,
    events: Arc<Mutex<Vec<InputMethodEvent>>>,
}

impl DbusMaliit {
    pub fn new() -> Result<Self, DbusMaliitServerError> {
        let conn = dbus::blocking::Connection::new_session()?;
        let proxy = conn.with_proxy(ADDRESS_DBUS_NAME, ADDRESS_PATH, Duration::from_secs(5));
        let address: String = proxy.get(ADDRESS_INTERFACE, ADDRESS_PROPERTY)?;
        log::info!("Got Maliit Server dbus address: {}", address);
        let channel = DbusChannel::open_private(address.as_str())?;

        let obj = Self {
            dbus_conn: Arc::new(SyncConnection::from(channel)),
            token: None,
            events: Arc::new(Mutex::new(Vec::new())),
        };
        // DbusMaliit::check_new_messages_async(&obj);

        Ok(obj)
    }

    pub fn activate_context(&self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "activateContext", ())?;
        Ok(())
    }

    pub fn show_input_method(&mut self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "showInputMethod", ())?;
        let events = self.events.clone();
        let token = self.dbus_conn.start_receive(dbus::message::MatchRule::new_method_call(), Box::new(move |msg, _conn|
            {
                log::debug!("Received dbus message: {:?}", msg);
                if let Some(member) = msg.member() {
                    match member.to_string().as_str() {
                        "commitString" => {
                            if let Some(text) = msg.get1::<String>() {
                                events.lock().unwrap().push(InputMethodEvent::Text(text));
                            }
                        },
                        "updateInputMethodArea" => {
                            if let (Some(x), Some(y)) = msg.get2::<i32, i32>() {
                                events.lock().unwrap().push(InputMethodEvent::AreaChanged(x, y));
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
        ));
        self.token = Some(token);

        Ok(())
    }

    pub fn hide_input_method(&mut self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "hideInputMethod", ())?;
        self.token.take().map(|t| self.dbus_conn.stop_receive(t));
        Ok(())
    }

    pub fn process_events(&self) -> Result<(), DbusMaliitServerError> {
        self.dbus_conn.process(Duration::from_millis(5))?;
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
}


#[derive(Error, Debug)]
pub enum DbusMaliitServerError {
    #[error("Dbus error")]
    SessionConnectionFailed(#[from] DbusError),
}
