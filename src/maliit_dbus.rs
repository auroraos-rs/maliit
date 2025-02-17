use std::time::Duration;

use thiserror::Error;
use dbus::{
    blocking::stdintf::org_freedesktop_dbus::Properties, channel::Channel as DbusChannel, Error as DbusError
};

const ADDRESS_DBUS_NAME: &str = "org.maliit.server";
const ADDRESS_PATH: &str = "/org/maliit/server/address";
const ADDRESS_INTERFACE: &str = "org.maliit.Server.Address";
const ADDRESS_PROPERTY: &str = "address";

const SERVER_DBUS_NAME: &str = "com.meego.inputmethod.uiserver1";
const SERVER_PATH: &str = "/com/meego/inputmethod/uiserver1";

pub(crate) struct DbusMaliit {
    dbus_conn: dbus::blocking::Connection,
}

impl DbusMaliit {
    pub fn new() -> Result<Self, DbusMaliitServerError> {
        let conn = dbus::blocking::Connection::new_session()?;
        let proxy = conn.with_proxy(ADDRESS_DBUS_NAME, ADDRESS_PATH, Duration::from_secs(5));
        let address: String = proxy.get(ADDRESS_INTERFACE, ADDRESS_PROPERTY)?;
        let channel = DbusChannel::open_private(address.as_str())?;

        Ok(Self { dbus_conn: dbus::blocking::Connection::from(channel) })
    }

    pub fn activate_context(&self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "activateContext", ())?;
        Ok(())
    }

    pub fn show_input_method(&self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "showInputMethod", ())?;
        Ok(())
    }

    pub fn hide_input_method(&self) -> Result<(), DbusMaliitServerError> {
        let maliit_proxy = self.dbus_conn.with_proxy(SERVER_DBUS_NAME, SERVER_PATH, Duration::from_secs(5));
        let _: () = maliit_proxy.method_call(SERVER_DBUS_NAME, "hideInputMethod", ())?;
        Ok(())
    }
}


#[derive(Error, Debug)]
pub enum DbusMaliitServerError {
    #[error("Dbus error")]
    SessionConnectionFailed(#[from] DbusError),
}
