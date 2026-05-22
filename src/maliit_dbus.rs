use std::sync::{Arc, Mutex};
use std::time::Duration;

use dbus::arg::messageitem::{MessageItem, MessageItemDict};
use dbus::blocking::{stdintf::org_freedesktop_dbus::Properties, Proxy, SyncConnection};
use dbus::channel::{Channel as DbusChannel, Token};
use dbus::strings::Signature;
use dbus::Message;

use crate::error::MaliitError;
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
    pub fn new() -> Result<Self, MaliitError> {
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
    dbus_conn: Arc<SyncConnection>,
}

impl MaliitUiServer {
    pub fn new(dbus_conn: Arc<SyncConnection>) -> Self {
        Self { dbus_conn }
    }

    fn proxy(&self) -> Proxy<'_, &SyncConnection> {
        self.dbus_conn
            .with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5))
    }

    pub fn activate_context(&self) -> Result<(), MaliitError> {
        let _: () = self
            .proxy()
            .method_call(SERVER_DBUS_NAME, "activateContext", ())?;
        Ok(())
    }

    pub fn reset(&self) -> Result<(), MaliitError> {
        let _: () = self.proxy().method_call(SERVER_DBUS_NAME, "reset", ())?;
        Ok(())
    }

    pub fn show_input_method(&mut self) -> Result<(), MaliitError> {
        let _: () = self
            .proxy()
            .method_call(SERVER_DBUS_NAME, "showInputMethod", ())?;
        Ok(())
    }

    pub fn hide_input_method(&mut self) -> Result<(), MaliitError> {
        let _: () = self
            .proxy()
            .method_call(SERVER_DBUS_NAME, "hideInputMethod", ())?;
        Ok(())
    }

    // pub fn set_preedit(
    //     &mut self,
    //     preedit_text: &str,
    //     cursor_position: i32,
    // ) -> Result<(), MaliitError> {
    //     let _: () = self.proxy().method_call(
    //         SERVER_DBUS_NAME,
    //         "setPreedit",
    //         (preedit_text, cursor_position),
    //     )?;
    //     Ok(())
    // }

    pub fn app_orientation_about_to_change(&mut self, angle: i32) -> Result<(), MaliitError> {
        let mut msg = Message::new_method_call(
            SERVER_DBUS_NAME,
            SERVER_PATH,
            SERVER_DBUS_NAME,
            "appOrientationAboutToChange",
        )
        .map_err(|_| MaliitError::NotAvailable)?;
        msg.set_no_reply(true);
        msg.append_items(&[MessageItem::Int32(angle)]);
        self.dbus_conn
            .channel()
            .send(msg)
            .map_err(|_| MaliitError::NotAvailable)?;
        Ok(())
    }

    pub fn app_orientation_changed(&mut self, angle: i32) -> Result<(), MaliitError> {
        let mut msg = Message::new_method_call(
            SERVER_DBUS_NAME,
            SERVER_PATH,
            SERVER_DBUS_NAME,
            "appOrientationChanged",
        )
        .map_err(|_| MaliitError::NotAvailable)?;
        msg.set_no_reply(true);
        msg.append_items(&[MessageItem::Int32(angle)]);
        self.dbus_conn
            .channel()
            .send(msg)
            .map_err(|_| MaliitError::NotAvailable)?;
        Ok(())
    }

    pub fn update_widget_information(
        &mut self,
        focus_state: bool,
        content_type: i32,
        prediction_enabled: bool,
        cursor_position: i32,
        surrounding_text: &str,
        focus_changed: bool,
    ) -> Result<(), MaliitError> {
        let entries = vec![
            (
                MessageItem::Str("focusState".to_string()),
                MessageItem::Variant(Box::new(MessageItem::Bool(focus_state))),
            ),
            (
                MessageItem::Str("contentType".to_string()),
                MessageItem::Variant(Box::new(MessageItem::Int32(content_type))),
            ),
            (
                MessageItem::Str("predictionEnabled".to_string()),
                MessageItem::Variant(Box::new(MessageItem::Bool(prediction_enabled))),
            ),
            (
                MessageItem::Str("cursorPosition".to_string()),
                MessageItem::Variant(Box::new(MessageItem::Int32(cursor_position))),
            ),
            (
                MessageItem::Str("surroundingText".to_string()),
                MessageItem::Variant(Box::new(MessageItem::Str(surrounding_text.to_string()))),
            ),
        ];

        let dict = MessageItemDict::new(
            entries,
            Signature::new("s").expect("s is a valid signature"),
            Signature::new("v").expect("v is a valid signature"),
        )
        .map_err(|_| MaliitError::NotAvailable)?;

        let dict_item = MessageItem::Dict(dict);

        let _: () = self.proxy().method_call(
            SERVER_DBUS_NAME,
            "updateWidgetInformation",
            (dict_item, focus_changed),
        )?;
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

    pub(crate) fn dbus_conn(&self) -> Arc<SyncConnection> {
        self.dbus_conn.clone()
    }

    fn proxy(&self) -> Proxy<'_, &SyncConnection> {
        self.dbus_conn
            .with_proxy(CONTEXT_INTERFACE, CONTEXT_PATH, Duration::from_secs(5))
    }

    pub fn stop_input_events_processing(&mut self) -> Result<(), MaliitError> {
        if let Some(token) = self.token.take() {
            self.proxy().match_stop(token, false)?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn start_input_events_processing(&mut self) -> Result<(), MaliitError> {
        let events = self.events.clone();
        let token = self.proxy().match_start(
            dbus::message::MatchRule::new_method_call(),
            false,
            Box::new(move |msg, _conn| {
                log::debug!("Received dbus message: {:?}", msg);
                if let Some(member) = msg.member() {
                    match member.to_string().as_str() {
                        "commitString" => {
                            if let (Some(text), Some(x), Some(y), Some(z)) =
                                msg.get4::<String, i32, i32, i32>()
                            {
                                log::debug!(
                                    "commitString received: text={}, args=({}, {}, {})",
                                    text,
                                    x,
                                    y,
                                    z
                                );
                                if let Ok(mut ev) = events.lock() {
                                    ev.push(InputMethodEvent::Text(text));
                                }
                            }
                        }
                        "updateInputMethodArea" => {
                            if let (Some(x), Some(y), Some(width), Some(height)) =
                                msg.get4::<i32, i32, i32, i32>()
                            {
                                if let Ok(mut ev) = events.lock() {
                                    ev.push(InputMethodEvent::AreaChanged(x, y, width, height));
                                }
                            }
                        }
                        "keyEvent" => {
                            if let (Some(is_pressed), _, _, Some(key_str)) =
                                msg.get4::<i32, i32, i32, String>()
                            {
                                // Не знаю почему, но при нажатии на кнопку Enter или Backspace прилетают два события с разными первыми аргументами.
                                // Полагаю, что это события по "нажатию" и "отпусканию", но почему-то они приходят одновременно.
                                let key = match key_str.as_str() {
                                    "\r" => Key::Enter,
                                    "\u{8}" => Key::Backspace,
                                    _ => {
                                        log::warn!(
                                            "Unknown key received, it won't be handled: {}.",
                                            key_str
                                        );
                                        return true;
                                    }
                                };
                                if let Ok(mut ev) = events.lock() {
                                    ev.push(InputMethodEvent::Key {
                                        key,
                                        pressed: is_pressed == 6,
                                    })
                                }
                            }
                        }
                        "imInitiatedHide" => {
                            if let Ok(mut ev) = events.lock() {
                                ev.push(InputMethodEvent::ImInitiatedHide);
                            }
                        }
                        "activationLostEvent" => {
                            if let Ok(mut ev) = events.lock() {
                                ev.push(InputMethodEvent::ActivationLost);
                            }
                        }
                        _ => {
                            println!("Received unknown method: {}", member.to_string().as_str());
                            // Unknown method call. We must return true to keep the filter alive.
                            // If the call expects a reply, send a default error reply.
                            if !msg.get_no_reply() {
                                if let Some(reply) = dbus::channel::default_reply(&msg) {
                                    let _ = _conn.channel().send(reply);
                                }
                            }
                        }
                    }
                }
                true
            }),
        )?;
        self.token = Some(token);
        Ok(())
    }

    pub fn get_new_events(&mut self) -> Vec<InputMethodEvent> {
        match self.events.lock() {
            Ok(mut mutex) => std::mem::take(mutex.as_mut()),
            Err(e) => {
                log::error!("Error with events mutex locking: {}", e);
                Vec::new()
            }
        }
    }

    pub fn process_events(&self, timeout: Duration) -> Result<bool, MaliitError> {
        Ok(self.proxy().connection.process(timeout)?)
    }

    // pub fn update_preedit(
    //     &mut self,
    //     text: &str,
    //     preedit_formats: (i32, i32, i32),
    //     replacement_start: i32,
    //     replacement_length: i32,
    //     cursor_position: i32,
    // ) -> Result<(), MaliitError> {
    //     let method_args = (
    //         text,
    //         preedit_formats,
    //         replacement_start,
    //         replacement_length,
    //         cursor_position,
    //     );
    //     let _: () = self
    //         .proxy()
    //         .method_call(CONTEXT_INTERFACE, "updatePreedit", method_args)?;
    //     Ok(())
    // }

    pub fn set_language(&mut self, language: &str) -> Result<(), MaliitError> {
        let _: () = self
            .proxy()
            .method_call(CONTEXT_INTERFACE, "setLanguage", (language,))?;
        Ok(())
    }
}
