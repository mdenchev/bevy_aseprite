use std::{fmt::Debug, string::FromUtf8Error};

use flate2::DecompressError;
use nom::{error::ParseError, IResult};

/// Errors that can occur during parsing
///
/// Encountering an error could mean one (or several) of these problems:
/// - You passed in an incomplete file (For example when loading immediately after a file change, as this means that the file could be empty or only partially saved)
/// - You passed in an invalid file (Not representing an .aseprite file)
/// - You are passing a file that is either too old, or too new. Make sure that you are saving/using a compatible Aseprite file.
///
/// If you encounter this error even though you have checked for the above problems, please report this as a bug.
#[derive(Debug, thiserror::Error)]
pub enum AsepriteParseError<I: std::fmt::Debug> {
    /// Color depth was invalid
    #[error("Found invalid color depth {0}. Expected 32/16/8.")]
    InvalidColorDepth(u16),
    /// An embedded string was not utf-8
    #[error("Found invalid UTF-8 {0}")]
    InvalidUtf8(FromUtf8Error),
    /// An invalid layer type was found
    #[error("Found invalid layer type {0}. Expected 0 (Normal) / 1 (Group)")]
    InvalidLayerType(u16),
    /// An invalid blend mode was found
    #[error("Found invalid blend mode {0}")]
    InvalidBlendMode(u16),
    /// The pixel data could not be decompressed
    #[error("Found invalid compressed data {0}")]
    InvalidCompressedData(DecompressError),
    /// There was not enough compressed data
    #[error("Did not find enough compressed data. File invalid.")]
    NotEnoughCompressedData,
    /// An invalid cel was found while decompressing
    #[error("Found invalid cel while decompressing")]
    InvalidCel,
    /// An invalid cel type was found
    #[error("Found invalid cel type {0}")]
    InvalidCelType(u16),
    /// An invalid animation direction was found
    #[error("Found invalid animation type {0}")]
    InvalidAnimationDirection(u8),

    /// A generic [`nom`] error was found
    #[error("Nom error: {nom:?}")]
    GenericNom {
        /// The input causing the error
        input: I,
        /// The error kind reported by [`nom`]
        nom: nom::error::ErrorKind,
    },

    /// Could not parse a layer chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidLayerChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a cel chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidCelChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a cel extra chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidCelExtraChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a tags chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidTagsChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a palette chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidPaletteChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a user data chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidUserDataChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a slice chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidSliceChunk(Box<AsepriteParseError<I>>),
    /// Could not parse a color profile chunk
    #[error("An error occured while parsing a layer_chunk")]
    InvalidColorProfileChunk(Box<AsepriteParseError<I>>),
}

impl<I: Debug> ParseError<I> for AsepriteParseError<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        AsepriteParseError::GenericNom { input, nom: kind }
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

/// Errors that can happen while loading an aseprite file
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AsepriteError {
    /// An error occured during parsing, see [`AsepriteParseError`] for possible causes
    ///
    /// Either way, this cannot be recovered from
    #[error("An error occured during parsing: {0}")]
    Parse(String),
    /// An IO error occured
    #[error("An IO error occured")]
    Io(#[from] std::io::Error),
    /// An invalid configuration was found while decoding
    #[error("Invalid configuration of the aseprite file")]
    InvalidConfiguration(#[from] AsepriteInvalidError),
}

impl<'a> From<AsepriteParseError<&'a [u8]>> for AsepriteError {
    fn from(other: AsepriteParseError<&'a [u8]>) -> Self {
        AsepriteError::Parse(other.to_string())
    }
}

/// An invalid configuration exists in the aseprite file
///
/// This should not happen with files that have not been manually edited
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AsepriteInvalidError {
    /// An invalid layer was specified in the aseprite file
    #[error("An invalid layer was specified")]
    InvalidLayer(usize),
    /// An invalid frame was specified in the frame
    #[error("An invalid frame was specified")]
    InvalidFrame(usize),
    /// An invalid palette index was specified as a color
    #[error("An invalid palette index was specified as a color")]
    InvalidPaletteIndex(usize),
}

pub(crate) type AseParseResult<'a, R> = IResult<&'a [u8], R, AsepriteParseError<&'a [u8]>>;
pub(crate) type AseResult<R> = std::result::Result<R, AsepriteError>;
