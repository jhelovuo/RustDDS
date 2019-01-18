/// Enumeration used to indicate the status of a ChangeForReader
pub enum ChangeForReaderStatusKind {
    UNSENT,
    UNACKNOWLEDGED,
    REQUESTED,
    ACKNOWLEDGED,
    UNDERWAY,
}
