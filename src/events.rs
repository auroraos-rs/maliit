#[derive(Debug, Clone, Copy)]
pub enum Key {
    Enter,
    Backspace,
}

#[derive(Debug, Clone)]
pub enum InputMethodEvent {
    Text(String),
    AreaChanged(i32, i32, i32, i32),
    Key {
        key: Key,
        pressed: bool
    },
    /// Maliit server requested to hide the keyboard (e.g. user pressed hide button).
    ImInitiatedHide,
    /// The input context lost activation on the Maliit server.
    ActivationLost,
}

/// Supported screen orientations for keyboard rotation.
#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Portrait = 0,
    Landscape = 90,
    PortraitFlipped = 180,
    LandscapeFlipped = 270,
}
