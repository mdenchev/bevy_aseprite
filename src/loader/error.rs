use bevy_aseprite_reader as reader;

#[derive(Debug)]
pub enum AsepriteLoaderError {
    Aseprite(reader::error::AsepriteError),
    Anyhow(anyhow::Error),
    Io(std::io::Error),
}

impl From<reader::error::AsepriteError> for AsepriteLoaderError {
    fn from(value: reader::error::AsepriteError) -> Self {
        Self::Aseprite(value)
    }
}

impl From<anyhow::Error> for AsepriteLoaderError {
    fn from(value: anyhow::Error) -> Self {
        Self::Anyhow(value)
    }
}

impl From<std::io::Error> for AsepriteLoaderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for AsepriteLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AsepriteLoaderError {}
