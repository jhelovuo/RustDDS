// A macro for logging of security events.
// Currently just creates a normal info-level log entry.
// However, this dedicated macro is intended as a reminder that security-related
// logging should be handled with special care (by a security-logging-plugin in
// the future?) So it acts as a placeholder for more to come.
#[macro_export]
macro_rules! security_info {
  ($($arg:tt)*) => (
      {log::info!($($arg)*);}
    )
}

// Same as above, just with warn-level
#[macro_export]
macro_rules! security_warn {
  ($($arg:tt)*) => (
      {log::warn!($($arg)*);}
    )
}
