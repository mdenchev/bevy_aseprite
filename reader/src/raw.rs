use std::convert::TryInto;

use crate::{
    error::{AseParseResult, AseResult, AsepriteError, AsepriteInvalidError, AsepriteParseError},
    AsepritePalette,
};

use flate2::Decompress;
use nom::{
    bytes::complete::{tag, take},
    combinator::{all_consuming, cond},
    multi::{count, length_data, many1},
    number::complete::{le_i16, le_i32, le_u16, le_u32, le_u8},
    Finish,
};
use tracing::{debug, debug_span, error, info};

// As specified in https://github.com/aseprite/aseprite/blob/fc79146c56f941f834f28809f0d2c4d7fd60076c/docs/ase-file-specs.md

/// Color depth in a single .aseprite file
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum AsepriteColorDepth {
    RGBA,
    Grayscale,
    Indexed,
}

impl AsepriteColorDepth {
    fn bytes_per_pixel(&self) -> usize {
        match self {
            AsepriteColorDepth::RGBA => 4,
            AsepriteColorDepth::Grayscale => 2,
            AsepriteColorDepth::Indexed => 1,
        }
    }
}

/// The raw Aseprite Header
#[derive(Debug, PartialEq)]
pub struct RawAsepriteHeader {
    /// File size of the .aseprite file
    pub file_size: u32,
    /// Magic number in the file, always `0xA5E0`
    pub magic_number: u16,
    /// Amount of frames in the body of the file
    pub frames: u16,
    /// Base width
    pub width: u16,
    /// Base height
    pub height: u16,
    /// The color depth used
    pub color_depth: AsepriteColorDepth,
    /// Flags for this file
    ///
    /// - 1 = Layer opacity has a valid value
    pub flags: u32,
    /// Milliseconds between frames (DEPRECATED)
    #[deprecated = "You should use the duration in each frame"]
    pub speed: u16,
    /// Palette entry which should be considered transparent
    ///
    /// This is only useful for indexed colors
    pub transparent_palette: u8,
    /// The amount of colors in the palette
    pub color_count: u16,
    /// Width of one pixel
    pub pixel_width: u8,
    /// Height of one pixel
    pub pixel_height: u8,
    /// Grid x start position
    pub grid_x: i16,
    /// Grid y start position
    pub grid_y: i16,
    /// Grid width
    pub grid_width: u16,
    /// Grid height
    pub grid_height: u16,
}

fn color_depth(input: &[u8]) -> AseParseResult<AsepriteColorDepth> {
    let (input, depth) = le_u16(input)?;
    Ok((
        input,
        match depth {
            32 => AsepriteColorDepth::RGBA,
            16 => AsepriteColorDepth::Grayscale,
            8 => AsepriteColorDepth::Indexed,
            depth => {
                return Err(nom::Err::Failure(AsepriteParseError::InvalidColorDepth(
                    depth,
                )))
            }
        },
    ))
}

const ASEPRITE_MAGIC_NUMBER: u16 = 0xA5E0;

fn aseprite_header(input: &[u8]) -> AseParseResult<RawAsepriteHeader> {
    let input_len = input.len();
    let (input, file_size) = le_u32(input)?;

    let (input, magic_number) = tag(&ASEPRITE_MAGIC_NUMBER.to_le_bytes())(input)?;
    let (input, frames) = le_u16(input)?;
    let (input, width) = le_u16(input)?;
    let (input, height) = le_u16(input)?;
    let (input, color_depth) = color_depth(input)?;
    let (input, flags) = le_u32(input)?;
    let (input, speed) = le_u16(input)?;
    let (input, _) = le_u32(input)?;
    let (input, _) = le_u32(input)?;
    let (input, transparent_palette) = le_u8(input)?;
    let (input, _) = take(3usize)(input)?;
    let (input, color_count) = le_u16(input)?;
    let (input, pixel_width) = le_u8(input)?;
    let (input, pixel_height) = le_u8(input)?;
    let (input, grid_x) = le_i16(input)?;
    let (input, grid_y) = le_i16(input)?;
    let (input, grid_width) = le_u16(input)?;
    let (input, grid_height) = le_u16(input)?;
    let (input, _) = take(84usize)(input)?;

    assert_eq!(input_len - input.len(), 128);

    Ok((
        input,
        #[allow(deprecated)]
        RawAsepriteHeader {
            file_size,
            magic_number: u16::from_le_bytes(
                magic_number
                    .try_into()
                    .expect("Invalid ASEPRITE_MAGIC_NUMBER matched. This is a bug."),
            ),
            frames,
            width,
            height,
            color_depth,
            flags,
            speed,
            transparent_palette,
            color_count,
            pixel_width,
            pixel_height,
            grid_x,
            grid_y,
            grid_width,
            grid_height,
        },
    ))
}

fn aseprite_string(input: &[u8]) -> AseParseResult<String> {
    let (input, name_len) = le_u16(input)?;
    let (input, name_bytes) = take(name_len as usize)(input)?;

    Ok((
        input,
        String::from_utf8(name_bytes.to_vec())
            .map_err(|err| nom::Err::Failure(AsepriteParseError::InvalidUtf8(err)))?,
    ))
}

/// A raw frame
pub struct RawAsepriteFrame {
    /// The magic frame number, always `0xF1FA`
    pub magic_number: u16,
    /// Duration of this frame, in ms
    pub duration_ms: u16,
    /// The chunks in this frame
    pub chunks: Vec<RawAsepriteChunk>,
}

/// A full RGBA color
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub struct AsepriteColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

fn aseprite_color(input: &[u8]) -> AseParseResult<AsepriteColor> {
    let (input, colors) = take(4usize)(input)?;

    Ok((
        input,
        AsepriteColor {
            red: colors[0],
            green: colors[1],
            blue: colors[2],
            alpha: colors[3],
        },
    ))
}

/// Raw user data
pub struct RawAsepriteUserData {
    /// Text, if any
    pub text: Option<String>,
    /// Color, if any
    pub color: Option<AsepriteColor>,
}

fn aseprite_user_data(input: &[u8]) -> AseParseResult<RawAsepriteUserData> {
    let (input, kind) = le_u32(input)?;

    let (input, text) = cond(kind & 0x1 != 0, aseprite_string)(input)?;
    let (input, color) = cond(kind & 0x2 != 0, aseprite_color)(input)?;

    Ok((input, RawAsepriteUserData { text, color }))
}

/// Layer type
pub enum AsepriteLayerType {
    /// A normal layer
    Normal,
    /// A layer group
    Group,
}

fn aseprite_layer_type(input: &[u8]) -> AseParseResult<AsepriteLayerType> {
    let (input, layer_type) = le_u16(input)?;

    Ok((
        input,
        match layer_type {
            0 => AsepriteLayerType::Normal,
            1 => AsepriteLayerType::Group,
            unknown => {
                return Err(nom::Err::Failure(AsepriteParseError::InvalidLayerType(
                    unknown,
                )));
            }
        },
    ))
}

#[derive(Debug, Clone, Copy)]
/// The different blend modes
#[allow(missing_docs)]
pub enum AsepriteBlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Addition,
    Subtract,
    Divide,
}

fn aseprite_blend_mode(input: &[u8]) -> AseParseResult<AsepriteBlendMode> {
    let (input, blend_mode) = le_u16(input)?;

    Ok((
        input,
        match blend_mode {
            0 => AsepriteBlendMode::Normal,
            1 => AsepriteBlendMode::Multiply,
            2 => AsepriteBlendMode::Screen,
            3 => AsepriteBlendMode::Overlay,
            4 => AsepriteBlendMode::Darken,
            5 => AsepriteBlendMode::Lighten,
            6 => AsepriteBlendMode::ColorDodge,
            7 => AsepriteBlendMode::ColorBurn,
            8 => AsepriteBlendMode::HardLight,
            9 => AsepriteBlendMode::SoftLight,
            10 => AsepriteBlendMode::Difference,
            11 => AsepriteBlendMode::Exclusion,
            12 => AsepriteBlendMode::Hue,
            13 => AsepriteBlendMode::Saturation,
            14 => AsepriteBlendMode::Color,
            15 => AsepriteBlendMode::Luminosity,
            16 => AsepriteBlendMode::Addition,
            17 => AsepriteBlendMode::Subtract,
            18 => AsepriteBlendMode::Divide,
            unknown => {
                return Err(nom::Err::Failure(AsepriteParseError::InvalidBlendMode(
                    unknown,
                )));
            }
        },
    ))
}

#[derive(Debug, Clone)]
/// A single pixel
pub enum AsepritePixel {
    /// Pixel in RGBA format
    RGBA(AsepriteColor),
    /// A grayscale pixel
    Grayscale {
        /// Gray intensity
        intensity: u16,
        /// Alpha value (opacity)
        alpha: u16,
    },
    /// Indexed pixel
    Indexed(u8),
}

impl AsepritePixel {
    /// Get the pixel as an array of RGBA values
    pub fn get_rgba(
        &self,
        palette: Option<&AsepritePalette>,
        transparent_palette: Option<u8>,
    ) -> AseResult<[u8; 4]> {
        match self {
            AsepritePixel::RGBA(color) => Ok([color.red, color.green, color.blue, color.alpha]),
            AsepritePixel::Grayscale { intensity, alpha } => Ok([
                (*intensity / 2) as u8,
                (*intensity / 2) as u8,
                (*intensity / 2) as u8,
                (*alpha / 2) as u8,
            ]),
            AsepritePixel::Indexed(idx) => {
                if transparent_palette != Some(*idx) {
                    palette
                        .and_then(|palette| palette.entries.get(*idx as usize))
                        .map(|color| [color.red, color.green, color.blue, color.alpha])
                        .ok_or(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidPaletteIndex(*idx as usize),
                        ))
                } else {
                    Ok([0; 4])
                }
            }
        }
    }
}

fn aseprite_pixel<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
) -> AseParseResult<'a, AsepritePixel> {
    match header.color_depth {
        AsepriteColorDepth::RGBA => {
            let (input, color) = aseprite_color(input)?;

            Ok((input, AsepritePixel::RGBA(color)))
        }
        AsepriteColorDepth::Grayscale => {
            let (input, intensity) = le_u16(input)?;
            let (input, alpha) = le_u16(input)?;

            Ok((input, AsepritePixel::Grayscale { intensity, alpha }))
        }
        AsepriteColorDepth::Indexed => {
            let (input, index) = le_u8(input)?;

            Ok((input, AsepritePixel::Indexed(index)))
        }
    }
}

fn aseprite_pixels<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
    amt: usize,
) -> AseParseResult<'a, Vec<AsepritePixel>> {
    count(|input: &'a [u8]| aseprite_pixel(input, header), amt)(input)
}

#[derive(Clone)]
/// Raw Cel
pub enum RawAsepriteCel {
    /// Raw Cel Data
    Raw {
        /// Width in pixels
        width: u16,
        /// Height in pixels
        height: u16,
        /// The pixels themselves
        pixels: Vec<AsepritePixel>,
    },
    /// Linked Cel Data
    Linked {
        /// Frame position to link with
        frame_position: u16,
    },
    /// Compressed Cel Data
    Compressed {
        /// Width in pixels
        width: u16,
        /// Height in pixels
        height: u16,
        /// The decompressed pixels
        pixels: Vec<AsepritePixel>,
    },
}

impl std::fmt::Debug for RawAsepriteCel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw { .. } => write!(f, "Raw"),
            Self::Linked { .. } => write!(f, "Linked"),
            Self::Compressed { .. } => write!(f, "Compressed"),
        }
    }
}

fn aseprite_cel<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
    cel_type: u16,
) -> AseParseResult<'a, RawAsepriteCel> {
    match cel_type {
        0 => {
            let (input, width) = le_u16(input)?;
            let (input, height) = le_u16(input)?;
            let (input, pixels) = aseprite_pixels(input, header, width as usize * height as usize)?;

            Ok((
                input,
                RawAsepriteCel::Raw {
                    width,
                    height,
                    pixels,
                },
            ))
        }
        1 => {
            let (input, frame_position) = le_u16(input)?;

            Ok((input, RawAsepriteCel::Linked { frame_position }))
        }
        2 => {
            let (input, width) = le_u16(input)?;
            let (input, height) = le_u16(input)?;

            // let (input, width) = le_u16(input)?;
            // let (input, height) = le_u16(input)?;

            // assert_eq!(outer_width, width);
            // assert_eq!(outer_height, height);

            let mut pixel_data =
                vec![0; width as usize * height as usize * header.color_depth.bytes_per_pixel()];

            let mut zlib_decompressor = Decompress::new(true);
            let status = zlib_decompressor
                .decompress(input, &mut pixel_data, flate2::FlushDecompress::Finish)
                .map_err(|flate_err| {
                    nom::Err::Failure(AsepriteParseError::InvalidCompressedData(flate_err))
                })?;

            match status {
                flate2::Status::Ok | flate2::Status::BufError => {
                    return Err(nom::Err::Failure(
                        AsepriteParseError::NotEnoughCompressedData,
                    ));
                }
                flate2::Status::StreamEnd => (),
            }

            let (_, pixels) =
                aseprite_pixels(&pixel_data, header, width as usize * height as usize)
                    .map_err(|_| nom::Err::Failure(AsepriteParseError::InvalidCel))?;

            Ok((
                &input[input.len()..],
                RawAsepriteCel::Compressed {
                    width,
                    height,
                    pixels,
                },
            ))
        }
        unknown => Err(nom::Err::Failure(AsepriteParseError::InvalidCelType(
            unknown,
        ))),
    }
}

#[derive(Debug, Clone, Copy)]
/// Animation Direction
pub enum AsepriteAnimationDirection {
    /// Forward animation direction
    ///
    /// When encountering the end, starts from the beginning
    Forward,
    /// Reverse animation direction
    ///
    /// When encountering the start, starts from the end
    Reverse,
    /// Ping-Pong animation direction
    ///
    /// Starts at beginning and reverses direction whenever it hits either end or beginning
    PingPong,
}

fn aseprite_anim_direction(input: &[u8]) -> AseParseResult<AsepriteAnimationDirection> {
    let (input, dir) = le_u8(input)?;

    Ok((
        input,
        match dir {
            0 => AsepriteAnimationDirection::Forward,
            1 => AsepriteAnimationDirection::Reverse,
            2 => AsepriteAnimationDirection::PingPong,
            unknown => {
                return Err(nom::Err::Failure(
                    AsepriteParseError::InvalidAnimationDirection(unknown),
                ));
            }
        },
    ))
}

/// A raw Tag
pub struct RawAsepriteTag {
    /// Starting frame
    pub from: u16,
    /// End frame
    pub to: u16,
    /// animation direction
    pub anim_direction: AsepriteAnimationDirection,
    /// name of the tag
    pub name: String,
}

fn aseprite_tag(input: &[u8]) -> AseParseResult<RawAsepriteTag> {
    let (input, from) = le_u16(input)?;
    let (input, to) = le_u16(input)?;
    let (input, anim_direction) = aseprite_anim_direction(input)?;
    let (input, _) = take(8usize)(input)?;
    let (input, _) = take(3usize)(input)?;
    let (input, _) = take(1usize)(input)?;
    let (input, name) = aseprite_string(input)?;

    Ok((
        input,
        RawAsepriteTag {
            from,
            to,
            anim_direction,
            name,
        },
    ))
}

/// Raw Chunk
pub enum RawAsepriteChunk {
    /// Layer Chunk
    ///
    /// All the layer chunks determine the general layer layout
    Layer {
        /// The flags set for this layer
        ///
        /// 1 = Visible
        /// 2 = Editable
        /// 4 = Lock Movement
        /// 8 = Background
        /// 16 = Prefer linked cels
        /// 32 = The layer group should be displayed collapsed
        /// 64 = The layer is a reference layer
        flags: u16,
        /// Type of the layer
        layer_type: AsepriteLayerType,
        /// How deep the child is in the hierarchy
        ///
        /// The higher, the deeper in the previous chunk
        layer_child: u16,
        /// layer width
        ///
        /// This is ignored
        width: u16,
        /// layer height
        ///
        /// This is ignored
        height: u16,
        /// The blend mode of the layer
        blend_mode: AsepriteBlendMode,
        /// Opacity of this layer
        opacity: u8,
        /// The name of the layer
        name: String,
    },
    /// A Cel is a container of pixel
    Cel {
        /// Which layer this cel corresponds to (0 based)
        layer_index: u16,
        /// x position
        x: i16,
        /// y position
        y: i16,
        /// Opacity of the cel
        opacity: u8,
        /// The cel content
        cel: RawAsepriteCel,
    },
    /// Extra data for the previous cel
    CelExtra {
        /// Flags for the extra cel
        ///
        /// - 1 = Precise bounds are set
        flags: u32,
        /// Precise x position
        x: f64,
        /// Precise y position
        y: f64,
        /// precise width of the cel
        width: f64,
        /// precise height of the cel
        height: f64,
    },
    /// Tags for this image
    Tags {
        /// the different tags
        tags: Vec<RawAsepriteTag>,
    },
    /// A color palette
    Palette {
        /// The total count of entries in the palette
        palette_size: u32,
        /// First color index to change
        from_color: u32,
        /// Last color index to change
        to_color: u32,
        /// The individual palette entries
        entries: Vec<RawAsepritePaletteEntry>,
    },
    /// User Data for the last chunk
    UserData {
        /// the data itself
        data: RawAsepriteUserData,
    },
    /// A slice in the image
    Slice {
        /// the flags for this slice
        flags: u32,
        /// the name of the slices
        name: String,
        /// the individual slices
        slices: Vec<RawAsepriteSlice>,
    },
    /// An embedded color profile
    ColorProfile {
        /// The type of color profile
        profile_type: u16,
        /// The flags for this color profile
        ///
        /// 1 = use the fixed gamma
        flags: u16,
        /// Fixed gamma
        gamma: f64,
        /// An embedded ICC Profile
        icc_profile: Option<RawAsepriteIccProfile>,
    },
}

/// A raw Icc Profile
pub struct RawAsepriteIccProfile {
    /// The bytes of the icc profile
    pub icc_profile: Vec<u8>,
}

fn aseprite_icc_profile(input: &[u8]) -> AseParseResult<RawAsepriteIccProfile> {
    let (input, icc_profile) = length_data(le_u32)(input)?;

    Ok((
        input,
        RawAsepriteIccProfile {
            icc_profile: icc_profile.to_vec(),
        },
    ))
}

fn color_profile_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, profile_type) = le_u16(input)?;
    let (input, flags) = le_u16(input)?;
    let (input, gamma) = aseprite_fixed(input)?;
    let (input, _) = take(8usize)(input)?;

    let (input, icc_profile) = cond(profile_type & 0x2 != 0, aseprite_icc_profile)(input)?;

    Ok((
        input,
        RawAsepriteChunk::ColorProfile {
            profile_type,
            flags,
            gamma,
            icc_profile,
        },
    ))
}

/// Raw Slice
pub struct RawAsepriteSlice {
    /// For which frame this slice is valid from (to the end of the animation)
    pub frame: u32,
    /// x origin, relative to the sprite
    pub x_origin: i32,
    /// y origin, relative to the sprite
    pub y_origin: i32,
    /// slice width
    ///
    /// this may be 0 if hidden from the given frame
    pub width: u32,
    /// slice height
    pub height: u32,
    /// 9-Patch info, if any
    pub nine_patch_info: Option<AsepriteNinePatchInfo>,
    /// A pivot, if any
    pub pivot: Option<AsepritePivot>,
}

#[derive(Debug, Clone)]
/// 9-Patch slice info
pub struct AsepriteNinePatchInfo {
    /// x center, relative to slice bounds
    pub x_center: i32,
    /// y center, relative to slice bounds
    pub y_center: i32,
    /// width of center
    pub width: u32,
    /// height of center
    pub height: u32,
}

fn aseprite_nine_patch_info(input: &[u8]) -> AseParseResult<AsepriteNinePatchInfo> {
    let (input, x_center) = le_i32(input)?;
    let (input, y_center) = le_i32(input)?;
    let (input, width) = le_u32(input)?;
    let (input, height) = le_u32(input)?;

    Ok((
        input,
        AsepriteNinePatchInfo {
            x_center,
            y_center,
            width,
            height,
        },
    ))
}

/// A raw pivot inside a slice
pub struct AsepritePivot {
    /// x position, relative to origin
    pub x_pivot: i32,
    /// y position, relative to origin
    pub y_pivot: i32,
}

fn aseprite_pivot(input: &[u8]) -> AseParseResult<AsepritePivot> {
    let (input, x_pivot) = le_i32(input)?;
    let (input, y_pivot) = le_i32(input)?;

    Ok((input, AsepritePivot { x_pivot, y_pivot }))
}

fn aseprite_slice(input: &[u8], flags: u32) -> AseParseResult<RawAsepriteSlice> {
    let (input, frame) = le_u32(input)?;
    let (input, x_origin) = le_i32(input)?;
    let (input, y_origin) = le_i32(input)?;
    let (input, width) = le_u32(input)?;
    let (input, height) = le_u32(input)?;
    let (input, nine_patch_info) = cond(flags & 0x1 != 0, aseprite_nine_patch_info)(input)?;
    let (input, pivot) = cond(flags & 0x2 != 0, aseprite_pivot)(input)?;

    Ok((
        input,
        RawAsepriteSlice {
            frame,
            x_origin,
            y_origin,
            width,
            height,
            nine_patch_info,
            pivot,
        },
    ))
}

fn aseprite_slices(
    input: &[u8],
    slice_count: usize,
    flags: u32,
) -> AseParseResult<Vec<RawAsepriteSlice>> {
    count(|input| aseprite_slice(input, flags), slice_count)(input)
}

fn slice_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, slice_count) = le_u32(input)?;
    let (input, flags) = le_u32(input)?;
    let (input, _) = le_u32(input)?;
    let (input, name) = aseprite_string(input)?;
    let (input, slices) = aseprite_slices(input, slice_count as usize, flags)?;

    Ok((
        input,
        RawAsepriteChunk::Slice {
            flags,
            name,
            slices,
        },
    ))
}

fn user_data_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, data) = aseprite_user_data(input)?;

    Ok((input, RawAsepriteChunk::UserData { data }))
}

/// A raw palette entry
pub struct RawAsepritePaletteEntry {
    /// color of this entry
    pub color: AsepriteColor,
    /// name of this entry
    pub name: Option<String>,
}

fn aseprite_palette(input: &[u8]) -> AseParseResult<RawAsepritePaletteEntry> {
    let (input, flags) = le_u16(input)?;
    let (input, color) = aseprite_color(input)?;

    let (input, name) = cond(flags & 0x1 == 0x1, aseprite_string)(input)?;

    Ok((input, RawAsepritePaletteEntry { color, name }))
}

fn aseprite_palettes(
    input: &[u8],
    palette_count: usize,
) -> AseParseResult<Vec<RawAsepritePaletteEntry>> {
    count(aseprite_palette, palette_count)(input)
}

fn palette_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, palette_size) = le_u32(input)?;
    let (input, from_color) = le_u32(input)?;
    let (input, to_color) = le_u32(input)?;
    let (input, _) = take(8usize)(input)?;

    let (input, entries) = aseprite_palettes(input, (to_color - from_color + 1) as usize)?;

    Ok((
        input,
        RawAsepriteChunk::Palette {
            palette_size,
            from_color,
            to_color,
            entries,
        },
    ))
}

fn tags(input: &[u8], tag_count: u16) -> AseParseResult<Vec<RawAsepriteTag>> {
    count(aseprite_tag, tag_count as usize)(input)
}

fn tags_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, tag_count) = le_u16(input)?;
    let (input, _) = take(8usize)(input)?;
    let (input, tags) = tags(input, tag_count)?;

    Ok((input, RawAsepriteChunk::Tags { tags }))
}

fn aseprite_fixed(input: &[u8]) -> AseParseResult<f64> {
    let (input, whole) = le_u32(input)?;

    Ok((input, whole as f64 / 0x10000 as f64))
}

fn cel_extra_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, flags) = le_u32(input)?;
    let (input, x) = aseprite_fixed(input)?;
    let (input, y) = aseprite_fixed(input)?;
    let (input, width) = aseprite_fixed(input)?;
    let (input, height) = aseprite_fixed(input)?;

    Ok((
        input,
        RawAsepriteChunk::CelExtra {
            flags,
            x,
            y,
            width,
            height,
        },
    ))
}

fn cel_chunk<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
) -> AseParseResult<'a, RawAsepriteChunk> {
    let (input, layer_index) = le_u16(input)?;
    let (input, x) = le_i16(input)?;
    let (input, y) = le_i16(input)?;
    let (input, opacity) = le_u8(input)?;
    let (input, cel_type) = le_u16(input)?;
    let (input, _) = take(7usize)(input)?;
    // We do not immediately try to load the cel, as the reserved bytes are decoupled from the type itself
    let (input, cel) = aseprite_cel(input, header, cel_type)?;

    Ok((
        input,
        RawAsepriteChunk::Cel {
            layer_index,
            x,
            y,
            opacity,
            cel,
        },
    ))
}

fn layer_chunk(input: &[u8]) -> AseParseResult<RawAsepriteChunk> {
    let (input, flags) = le_u16(input)?;
    let (input, layer_type) = aseprite_layer_type(input)?;
    let (input, layer_child) = le_u16(input)?;
    let (input, width) = le_u16(input)?;
    let (input, height) = le_u16(input)?;
    let (input, blend_mode) = aseprite_blend_mode(input)?;
    let (input, opacity) = le_u8(input)?;
    let (input, _) = take(3usize)(input)?;
    let (input, name) = aseprite_string(input)?;

    Ok((
        input,
        RawAsepriteChunk::Layer {
            flags,
            layer_type,
            layer_child,
            width,
            height,
            blend_mode,
            opacity,
            name,
        },
    ))
}

fn aseprite_chunk<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
) -> AseParseResult<'a, Option<RawAsepriteChunk>> {
    let input_len = input.len();
    let (input, chunk_size) = le_u32(input)?;
    let (input, chunk_type) = le_u16(input)?;
    // Get the remaining data of this chunk and parse it as the corresponding type
    let (input, chunk_data) = take(chunk_size as usize - (input_len - input.len()))(input)?;

    let _span = debug_span!("chunk", chunk_type);

    let res =
        match chunk_type {
            0x0004 => {
                debug!("Ignoring chunk of kind {} (Old palette chunk)", chunk_type);
                None
            }
            0x0011 => {
                debug!("Ignoring chunk of kind {} (Old palette chunk)", chunk_type);
                None
            }
            0x2004 => Some(all_consuming(layer_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidLayerChunk(Box::new(err)))
            })?),
            0x2005 => Some(
                all_consuming(|input: &'a [u8]| cel_chunk(input, header))(chunk_data).map_err(
                    |err| err.map(|err| AsepriteParseError::InvalidCelChunk(Box::new(err))),
                )?,
            ),
            0x2006 => Some(all_consuming(cel_extra_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidCelExtraChunk(Box::new(err)))
            })?),
            0x2007 => Some(color_profile_chunk(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidColorProfileChunk(Box::new(err)))
            })?),
            0x2016 => {
                info!("Got a deprecated profile chunk");
                None
            }
            0x2018 => Some(all_consuming(tags_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidTagsChunk(Box::new(err)))
            })?),
            0x2019 => Some(all_consuming(palette_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidPaletteChunk(Box::new(err)))
            })?),
            0x2020 => Some(all_consuming(user_data_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidUserDataChunk(Box::new(err)))
            })?),
            0x2022 => Some(all_consuming(slice_chunk)(chunk_data).map_err(|err| {
                err.map(|err| AsepriteParseError::InvalidSliceChunk(Box::new(err)))
            })?),
            chunk_type => {
                error!("Got unknown chunk type: {:?}", chunk_type);
                None
            }
        };

    Ok((input, res.map(|(_, chunk)| chunk)))
}

const ASEPRITE_FRAME_MAGIC_NUMBER: u16 = 0xF1FA;

fn aseprite_frame<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
) -> AseParseResult<'a, RawAsepriteFrame> {
    let (input, magic_number) = tag(&ASEPRITE_FRAME_MAGIC_NUMBER.to_le_bytes())(input)?;
    let (input, small_chunk_count) = le_u16(input)?;
    let (input, duration_ms) = le_u16(input)?;
    let (input, _) = take(2usize)(input)?;
    let (input, chunk_count) = le_u32(input)?;

    // As per spec, if an older file is being read, it might not set chunk_count yet, so we use small_chunk_count
    let actual_count = if chunk_count == 0 {
        small_chunk_count as usize
    } else {
        chunk_count as usize
    };

    let (input, chunks) = count(
        |input: &'a [u8]| aseprite_chunk(input, header),
        actual_count,
    )(input)?;

    let chunks = chunks.into_iter().flatten().collect();

    Ok((
        input,
        RawAsepriteFrame {
            magic_number: u16::from_le_bytes(
                magic_number
                    .try_into()
                    .expect("Invalid ASEPRITE_FRAME_MAGIC_NUMBER matched. This is a bug."),
            ),
            duration_ms,
            chunks,
        },
    ))
}

fn aseprite_frames<'a>(
    input: &'a [u8],
    header: &'_ RawAsepriteHeader,
) -> AseParseResult<'a, Vec<RawAsepriteFrame>> {
    all_consuming(many1(
        |input: &'a [u8]| -> AseParseResult<RawAsepriteFrame> {
            let (input, _length) = le_u32(input)?;
            aseprite_frame(input, header)
        },
    ))(input)
}

/// A raw .aseprite file
pub struct RawAseprite {
    /// The header describes how the rest of the file is to be interpreted
    pub header: RawAsepriteHeader,
    /// A vector of frames inside the file
    pub frames: Vec<RawAsepriteFrame>,
}

fn aseprite(input: &[u8]) -> AseParseResult<RawAseprite> {
    let (input, header) = aseprite_header(input)?;
    let (input, frames) = aseprite_frames(input, &header)?;

    Ok((input, RawAseprite { header, frames }))
}

/// Read a [`RawAseprite`] from memory
pub fn read_aseprite(input: &[u8]) -> Result<RawAseprite, AsepriteError> {
    let (_, ase) = aseprite(input).finish()?;

    Ok(ase)
}

#[cfg(test)]
#[allow(deprecated)]
mod test {
    use super::{aseprite_frames, aseprite_header, RawAsepriteHeader, ASEPRITE_MAGIC_NUMBER};

    #[test]
    fn check_valid_file_header() {
        let ase_file = std::fs::read("./tests/test_cases/simple.aseprite").unwrap();

        let (_rest, raw_header) = aseprite_header(&ase_file).unwrap();

        let expected = RawAsepriteHeader {
            file_size: 787,
            magic_number: ASEPRITE_MAGIC_NUMBER,
            frames: 1,
            width: 123,
            height: 456,
            color_depth: super::AsepriteColorDepth::RGBA,
            flags: 1,
            speed: 125,
            transparent_palette: 0,
            color_count: 32,
            pixel_width: 1,
            pixel_height: 1,
            grid_x: 0,
            grid_y: 0,
            grid_width: 16,
            grid_height: 16,
        };

        assert_eq!(raw_header, expected);
    }

    #[test]
    fn check_valid_file() {
        let ase_file = std::fs::read("./tests/test_cases/simple.aseprite").unwrap();

        let (body, raw_header) = aseprite_header(&ase_file).unwrap();

        let (rest, raw_body) = aseprite_frames(body, &raw_header).unwrap();

        assert_eq!(rest.len(), 0);
        assert_eq!(raw_body.len(), 1);
        let frame = &raw_body[0];

        assert_eq!(frame.duration_ms, 125);
    }
}
