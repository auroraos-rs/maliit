use crate::maliit_dbus::DbusMaliit;

pub struct InputMethod {
    dbus_maliit: DbusMaliit
}

impl InputMethod {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let dbus_maliit =  DbusMaliit::new()?;
        Ok(Self { dbus_maliit })
    }

    pub fn show(&self) {
        self.dbus_maliit.activate_context().unwrap();
        self.dbus_maliit.show_input_method().unwrap();
    }

    pub fn hide(&self) {
        self.dbus_maliit.activate_context().unwrap();
        self.dbus_maliit.hide_input_method().unwrap();
    }
}
