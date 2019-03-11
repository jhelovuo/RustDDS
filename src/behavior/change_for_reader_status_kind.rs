/// Enumeration used to indicate the status of a ChangeForReader
#[derive(Debug, PartialEq, Eq)]
pub enum ChangeForReaderStatusKind {
    UNSENT,
    UNACKNOWLEDGED,
    REQUESTED,
    ACKNOWLEDGED,
    UNDERWAY,
}
