use maliit_glib_sys::{maliit_input_method_new, maliit_input_method_show, maliit_input_method_hide, MaliitInputMethod};

pub struct InputMethod {
    inner: *mut MaliitInputMethod
}

impl InputMethod {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let inner =  unsafe {
            maliit_input_method_new()
        };
        if inner.is_null() {
            return Err("Error with creating MaliitInputMethod".into())
        }
        Ok(Self { inner })
    }

    pub fn show_input_method(&self) {
        unsafe {
            maliit_input_method_show(self.inner);
        }
    }

    pub fn hide_input_method(&self) {
        unsafe {
            maliit_input_method_hide(self.inner);
        }
    }
}
