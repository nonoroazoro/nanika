use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum PngResourceError {
    NotFound,
    OutsideRoot,
    EncodedSize,
    Dimensions { width: u32, height: u32 },
    Decode(png::DecodingError),
    Io(std::io::Error),
}

impl Display for PngResourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("PNG resource was not found"),
            Self::OutsideRoot => formatter.write_str("PNG resource is outside its payload root"),
            Self::EncodedSize => formatter.write_str("PNG resource exceeds the encoded size limit"),
            Self::Dimensions { width, height } => write!(
                formatter,
                "PNG resource exceeds the dimension limit: {width} x {height}"
            ),
            Self::Decode(error) => write!(formatter, "PNG resource is invalid: {error}"),
            Self::Io(error) => write!(formatter, "PNG resource could not be read: {error}"),
        }
    }
}

impl std::error::Error for PngResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl PngResourceError {
    pub fn from_io(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::Io(error)
        }
    }
}
