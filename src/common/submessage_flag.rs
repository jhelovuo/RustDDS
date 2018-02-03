#[derive(Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
pub struct SubmessageFlag {
    pub flags: [bool;8]
}

impl SubmessageFlag {
    pub fn new() -> SubmessageFlag {
        SubmessageFlag {
            flags: [false; 8]
        }
    }
}
