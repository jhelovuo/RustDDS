// Macros for logging security events.
// Currently these just create a normal log entry,
// but they're intended as a reminder that security-related
// logging should be handled with special care (by a security-logging-plugin in
// the future?) So they act as a placeholder for more to come.
#[macro_export]
macro_rules! security_info {
  ($($arg:tt)*) => (
      {log::info!($($arg)*);}
    )
}

#[macro_export]
macro_rules! security_warn {
  ($($arg:tt)*) => (
      {log::warn!($($arg)*);}
    )
}

#[macro_export]
macro_rules! security_error {
  ($($arg:tt)*) => (
      {log::error!($($arg)*);}
    )
}
