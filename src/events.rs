#[derive(Debug, Clone, Copy)]
pub enum Key {
    Enter,
    Backspace,
}

#[derive(Debug, Clone)]
pub enum InputMethodEvent {
    Text(String),
    AreaChanged(i32, i32),
    Key {
        key: Key,
        pressed: bool
    }
}
