#[derive(Debug, PartialOrd, PartialEq, Ord, Eq)]
pub enum ChangeKind_t {
  ALIVE,
  NOT_ALIVE_DISPOSED,
  NOT_ALIVE_UNREGISTERED,
}
