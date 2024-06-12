#[derive(Debug, PartialEq)]
pub enum SkipImplementation {
    SkipFromImplementation,
    SkipTryIntoImplementation,
}

impl SkipImplementation {
    pub fn from_str(string: &str) -> Option<Self> {
        match string {
            "skip_from" => Some(Self::SkipFromImplementation),
            "skip_try_into" => Some(Self::SkipTryIntoImplementation),
            _ => None,
        }
    }
}
