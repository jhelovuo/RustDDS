use crate::behavior::change_for_reader_status_kind::ChangeForReaderStatusKind;

/// The RTPS ChangeForReader is an association class that maintains information
/// of a CacheChange in the RTPS WriterHistoryCache as it pertains to the RTPS
/// Reader represented by the ReaderProxy.
pub struct ChangeForReader {
    /// Indicates the status of a CacheChange relative to the RTPS Reader
    /// represented by the ReaderProxy.
    pub status: ChangeForReaderStatusKind,

    /// Indicates whether the change is relevant to the RTPS Reader
    /// represented by the ReaderProxy.
    pub is_relevant: bool,
}
