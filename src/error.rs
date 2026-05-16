use thiserror::Error;

#[derive(Debug, Error)]
pub enum MaliitError {
    #[error("D-Bus error: {0}")]
    Dbus(#[from] dbus::Error),
    #[error("IME not available")]
    NotAvailable,
}
