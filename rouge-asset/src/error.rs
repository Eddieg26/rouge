#[derive(Debug)]
pub enum AssetError {
    Io(std::io::Error),
    InvalidPath,
    InvalidData,
    InvalidDependency,
    InvalidMetadata,
    InvalidSettings,
    Other(String),
}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        AssetError::Io(e)
    }
}
