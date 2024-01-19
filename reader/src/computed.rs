use std::{
    collections::{BTreeMap, HashMap},
    ops::{Index, Range},
    path::Path,
};

use image::{Pixel, Rgba, RgbaImage};
use tracing::{error, warn};

use crate::raw::RawAsepriteCel::Raw;
use crate::{
    error::{AseResult, AsepriteError, AsepriteInvalidError},
    raw::{
        AsepriteAnimationDirection, AsepriteBlendMode, AsepriteColor, AsepriteColorDepth,
        AsepriteLayerType, AsepriteNinePatchInfo, AsepritePixel, RawAseprite, RawAsepriteCel,
        RawAsepriteChunk, RawAsepritePaletteEntry,
    },
};

#[derive(Debug, Clone)]
/// Data structure representing an Aseprite file
pub struct Aseprite {
    dimensions: (u16, u16),
    tags: HashMap<String, AsepriteTag>,
    slices: HashMap<String, AsepriteSlice>,
    layers: BTreeMap<usize, AsepriteLayer>,
    frame_count: usize,
    palette: Option<AsepritePalette>,
    transparent_palette: Option<u8>,
    frame_infos: Vec<AsepriteFrameInfo>,
}

impl Aseprite {
    /// Get the [`AsepriteTag`]s defined in this Aseprite
    pub fn tags(&self) -> AsepriteTags {
        AsepriteTags { tags: &self.tags }
    }

    /// Get the associated [`AsepriteLayer`]s defined in this Aseprite
    pub fn layers(&self) -> AsepriteLayers {
        AsepriteLayers {
            layers: &self.layers,
        }
    }

    /// Get the frames inside this aseprite
    pub fn frames(&self) -> AsepriteFrames {
        AsepriteFrames { aseprite: self }
    }

    /// Get infos about the contained frames
    pub fn frame_infos(&self) -> &[AsepriteFrameInfo] {
        &self.frame_infos
    }

    /// Get the slices inside this aseprite
    pub fn slices(&self) -> AsepriteSlices {
        AsepriteSlices { aseprite: self }
    }
}

impl Aseprite {
    /// Construct a [`Aseprite`] from a [`RawAseprite`]
    pub fn from_raw(raw: RawAseprite) -> AseResult<Self> {
        let mut tags = HashMap::new();
        let mut layers = BTreeMap::new();
        let mut palette = None;
        let mut frame_infos = vec![];
        let mut slices = HashMap::new();

        let frame_count = raw.frames.len();

        let mut frame_idx: usize = 0;

        for frame in raw.frames {
            frame_infos.push(AsepriteFrameInfo {
                delay_ms: frame.duration_ms as usize,
            });

            for chunk in frame.chunks {
                match chunk {
                    RawAsepriteChunk::Layer {
                        flags,
                        layer_type,
                        layer_child,
                        width: _,
                        height: _,
                        blend_mode,
                        opacity,
                        name,
                    } => {
                        let id = layers.len();
                        let layer = AsepriteLayer::new(
                            id,
                            name,
                            layer_type,
                            flags & 0x1 != 0,
                            blend_mode,
                            if raw.header.flags & 0x1 != 0 {
                                Some(opacity)
                            } else {
                                None
                            },
                            layer_child,
                        );
                        layers.insert(id, layer);
                    }
                    crate::raw::RawAsepriteChunk::Cel {
                        layer_index,
                        x,
                        y,
                        opacity,
                        cel,
                    } => {
                        let layer = layers
                            .get_mut(&(layer_index as usize))
                            .ok_or(AsepriteInvalidError::InvalidLayer(layer_index as usize))?;

                        layer.add_cel(
                            frame_idx,
                            AsepriteCel::new(x as f64, y as f64, opacity, cel),
                        )?;
                    }
                    crate::raw::RawAsepriteChunk::CelExtra {
                        flags: _,
                        x: _,
                        y: _,
                        width: _,
                        height: _,
                    } => warn!("Not yet implemented cel extra"),
                    crate::raw::RawAsepriteChunk::Tags { tags: raw_tags } => {
                        tags.extend(raw_tags.into_iter().map(|raw_tag| {
                            (
                                raw_tag.name.clone(),
                                AsepriteTag {
                                    frames: raw_tag.from..raw_tag.to + 1,
                                    animation_direction: raw_tag.anim_direction,
                                    name: raw_tag.name,
                                },
                            )
                        }))
                    }
                    crate::raw::RawAsepriteChunk::Palette {
                        palette_size,
                        from_color,
                        to_color: _,
                        entries,
                    } => {
                        palette =
                            Some(AsepritePalette::from_raw(palette_size, from_color, entries));
                    }
                    crate::raw::RawAsepriteChunk::UserData { data: _ } => {
                        warn!("Not yet implemented user data")
                    }
                    crate::raw::RawAsepriteChunk::Slice {
                        flags: _,
                        name,
                        slices: raw_slices,
                    } => slices.extend(raw_slices.into_iter().map(
                        |crate::raw::RawAsepriteSlice {
                             frame,
                             x_origin,
                             y_origin,
                             width,
                             height,
                             nine_patch_info,
                             pivot: _,
                         }| {
                            (
                                name.clone(),
                                AsepriteSlice {
                                    name: name.clone(),
                                    valid_frame: frame as u16,
                                    position_x: x_origin,
                                    position_y: y_origin,
                                    width,
                                    height,
                                    nine_patch_info,
                                },
                            )
                        },
                    )),
                    crate::raw::RawAsepriteChunk::ColorProfile {
                        profile_type: _,
                        flags: _,
                        gamma: _,
                        icc_profile: _,
                    } => warn!("Not yet implemented color profile"),
                }
            }

            frame_idx += 1;
        }

        Ok(Aseprite {
            dimensions: (raw.header.width, raw.header.height),
            transparent_palette: if raw.header.color_depth == AsepriteColorDepth::Indexed {
                Some(raw.header.transparent_palette)
            } else {
                None
            },
            tags,
            layers,
            frame_count,
            palette,
            frame_infos,
            slices,
        })
    }

    /// Construct a [`Aseprite`] from a [`Path`]
    pub fn from_path<S: AsRef<Path>>(path: S) -> AseResult<Self> {
        let buffer = std::fs::read(path)?;

        let raw_aseprite = crate::raw::read_aseprite(&buffer)?;

        Self::from_raw(raw_aseprite)
    }

    /// Construct a [`Aseprite`] from a `&[u8]`
    pub fn from_bytes<S: AsRef<[u8]>>(buffer: S) -> AseResult<Self> {
        let raw_aseprite = crate::raw::read_aseprite(buffer.as_ref())?;

        Self::from_raw(raw_aseprite)
    }
}

/// The loaded aseprite file without image data
#[derive(Debug, Clone)]
pub struct AsepriteInfo {
    pub dimensions: (u16, u16),
    pub tags: HashMap<String, AsepriteTag>,
    pub slices: HashMap<String, AsepriteSlice>,
    pub frame_count: usize,
    pub palette: Option<AsepritePalette>,
    pub transparent_palette: Option<u8>,
    pub frame_infos: Vec<AsepriteFrameInfo>,
}

impl Into<AsepriteInfo> for Aseprite {
    fn into(self) -> AsepriteInfo {
        AsepriteInfo {
            dimensions: self.dimensions,
            tags: self.tags,
            slices: self.slices,
            frame_count: self.frame_count,
            palette: self.palette,
            transparent_palette: self.transparent_palette,
            frame_infos: self.frame_infos,
        }
    }
}

/// The palette entries in the aseprite file
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct AsepritePalette {
    pub entries: Vec<AsepriteColor>,
}

impl AsepritePalette {
    fn from_raw(
        palette_size: u32,
        from_color: u32,
        raw_entries: Vec<RawAsepritePaletteEntry>,
    ) -> Self {
        let mut entries = vec![
            AsepriteColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0
            };
            palette_size as usize
        ];

        for (raw_idx, idx) in ((from_color as usize)..entries.len()).enumerate() {
            entries[idx] = raw_entries[raw_idx].color;
        }

        AsepritePalette { entries }
    }
}

/// All the tags defined in the corresponding aseprite
pub struct AsepriteTags<'a> {
    tags: &'a HashMap<String, AsepriteTag>,
}

impl<'a> AsepriteTags<'a> {
    /// Get a tag defined by its name
    pub fn get_by_name<N: AsRef<str>>(&self, name: N) -> Option<&AsepriteTag> {
        self.tags.get(name.as_ref())
    }

    /// Get all available tags
    pub fn all(&self) -> impl Iterator<Item = &AsepriteTag> {
        self.tags.values()
    }
}

impl<'a, 'r> Index<&'r str> for AsepriteTags<'a> {
    type Output = AsepriteTag;

    fn index(&self, index: &'r str) -> &Self::Output {
        self.get_by_name(index).unwrap()
    }
}

#[derive(Debug, Clone)]
/// A single Aseprite tag
pub struct AsepriteTag {
    /// The frames which this tag represents
    pub frames: Range<u16>,
    /// The direction of its animation
    pub animation_direction: AsepriteAnimationDirection,
    /// The tag name
    pub name: String,
}

#[derive(Debug, Clone)]
/// A single Aseprite slice
pub struct AsepriteSlice {
    /// The slice name
    pub name: String,
    /// The frame from which it is valid
    pub valid_frame: u16,
    /// The slice's x position
    pub position_x: i32,
    /// The slice's y position
    pub position_y: i32,
    /// The slice's width
    pub width: u32,
    /// The slice's height
    pub height: u32,
    /// Nine-Patch Info if it exists
    pub nine_patch_info: Option<AsepriteNinePatchInfo>,
}

/// The layers inside an aseprite file
pub struct AsepriteLayers<'a> {
    layers: &'a BTreeMap<usize, AsepriteLayer>,
}

impl<'a> AsepriteLayers<'a> {
    /// Get a layer by its name
    ///
    /// If you have its id, prefer fetching it using [`get_by_id`]
    pub fn get_by_name<N: AsRef<str>>(&self, name: N) -> Option<&AsepriteLayer> {
        let name = name.as_ref();
        self.layers
            .iter()
            .find(|(_, layer)| layer.name() == name)
            .map(|(_, layer)| layer)
    }

    /// Get a layer by its id
    pub fn get_by_id(&self, id: usize) -> Option<&AsepriteLayer> {
        self.layers.get(&id)
    }
}

#[derive(Debug, Clone)]
/// An aseprite layer
pub enum AsepriteLayer {
    /// A layer group
    Group {
        /// Name of the layer
        name: String,
        /// Id of the layer
        id: usize,
        /// Visibility of the layer
        visible: bool,
        /// How deep it is nested in the layer hierarchy
        child_level: u16,
    },
    /// A normal layer
    Normal {
        /// Name of the layer
        name: String,
        /// Id of the layer
        id: usize,
        /// Blend mode of this layer
        blend_mode: AsepriteBlendMode,
        /// Opacity of this layer (if enabled)
        opacity: Option<u8>,
        /// Visibility of this layer
        visible: bool,
        /// How deep it is nested in the layer hierarchy
        child_level: u16,
        /// Cels keyed by frame index
        cels: HashMap<usize, AsepriteCel>,
    },
}

impl AsepriteLayer {
    fn new(
        id: usize,
        name: String,
        layer_type: AsepriteLayerType,
        visible: bool,
        blend_mode: AsepriteBlendMode,
        opacity: Option<u8>,
        child_level: u16,
    ) -> Self {
        match layer_type {
            AsepriteLayerType::Normal => AsepriteLayer::Normal {
                name,
                id,
                blend_mode,
                opacity,
                visible,
                child_level,
                cels: HashMap::new(),
            },
            AsepriteLayerType::Group => AsepriteLayer::Group {
                name,
                id,
                visible,
                child_level,
            },
        }
    }

    /// Get the name of the layer
    pub fn name(&self) -> &str {
        match self {
            AsepriteLayer::Group { name, .. } | AsepriteLayer::Normal { name, .. } => name,
        }
    }

    /// Get the id of the layer
    pub fn id(&self) -> usize {
        match self {
            AsepriteLayer::Group { id, .. } | AsepriteLayer::Normal { id, .. } => *id,
        }
    }

    /// Get the visibility of the layer
    pub fn is_visible(&self) -> bool {
        match self {
            AsepriteLayer::Group { visible, .. } | AsepriteLayer::Normal { visible, .. } => {
                *visible
            }
        }
    }

    /// Returns `true` if the aseprite layer is [`Group`].
    ///
    /// [`Group`]: AsepriteLayer::Group
    #[must_use]
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group { .. })
    }

    fn cel_count(&self) -> usize {
        match self {
            AsepriteLayer::Group { .. } => 0,
            AsepriteLayer::Normal { cels, .. } => cels.len(),
        }
    }

    fn add_cel(&mut self, frame: usize, cel: AsepriteCel) -> AseResult<()> {
        match self {
            AsepriteLayer::Group { id, .. } => {
                return Err(AsepriteError::InvalidConfiguration(
                    AsepriteInvalidError::InvalidLayer(*id),
                ));
            }
            AsepriteLayer::Normal { cels, .. } => {
                cels.insert(frame, cel);
            }
        }

        Ok(())
    }

    fn get_cel(&self, frame: usize) -> AseResult<&AsepriteCel> {
        match self {
            AsepriteLayer::Group { id, .. } => Err(AsepriteError::InvalidConfiguration(
                AsepriteInvalidError::InvalidLayer(*id),
            )),
            AsepriteLayer::Normal { cels, .. } => cels.get(&frame).ok_or(
                AsepriteError::InvalidConfiguration(AsepriteInvalidError::InvalidFrame(frame)),
            ),
        }
    }
}

#[derive(Debug, Clone)]
/// A single cel in a frame in a layer
pub struct AsepriteCel {
    x: f64,
    y: f64,
    opacity: u8,
    raw_cel: RawAsepriteCel,
}

impl AsepriteCel {
    fn new(x: f64, y: f64, opacity: u8, raw_cel: RawAsepriteCel) -> Self {
        AsepriteCel {
            x,
            y,
            opacity,
            raw_cel,
        }
    }
}

/// The frames contained in an aseprite
pub struct AsepriteFrames<'a> {
    aseprite: &'a Aseprite,
}

impl<'a> AsepriteFrames<'a> {
    /// Get a range of frames
    pub fn get_for(&self, range: &Range<u16>) -> AsepriteFrameRange {
        AsepriteFrameRange {
            aseprite: self.aseprite,
            range: range.clone(),
        }
    }

    /// Get the amount of frames in this aseprite
    pub fn count(&self) -> usize {
        self.aseprite.frame_count
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// The nine slices in a nine-patch image
#[allow(missing_docs)]
pub enum NineSlice {
    TopLeft,
    TopCenter,
    TopRight,
    RightCenter,
    BottomRight,
    BottomCenter,
    BottomLeft,
    LeftCenter,
    Center,
}

/// A single slice image
///
/// Only contains nine-patch info if the aseprite also contained one
#[allow(missing_docs)]
pub struct AsepriteSliceImage {
    pub image: RgbaImage,
    pub nine_slices: Option<HashMap<NineSlice, RgbaImage>>,
}

/// The slices contained in an aseprite
pub struct AsepriteSlices<'a> {
    aseprite: &'a Aseprite,
}

impl<'a> AsepriteSlices<'a> {
    /// Get a slice by name
    pub fn get_by_name(&self, name: &str) -> Option<&AsepriteSlice> {
        self.aseprite.slices.get(name)
    }

    /// Get all slices in this aseprite
    pub fn get_all(&self) -> impl Iterator<Item = &AsepriteSlice> + '_ {
        self.aseprite.slices.values()
    }

    /// Get the images represented by the slices
    pub fn get_images<I: Iterator<Item = &'a AsepriteSlice>>(
        &self,
        wanted_slices: I,
    ) -> AseResult<Vec<AsepriteSliceImage>> {
        let mut slices = vec![];

        for slice in wanted_slices {
            let frame = image_for_frame(self.aseprite, slice.valid_frame)?;

            let image = image::imageops::crop_imm(
                &frame,
                slice.position_x.max(0) as u32,
                slice.position_y.max(0) as u32,
                slice.width,
                slice.height,
            )
            .to_image();

            let slice_image = AsepriteSliceImage {
                nine_slices: slice.nine_patch_info.as_ref().map(|info| {
                    let mut map: HashMap<_, RgbaImage> = HashMap::new();

                    let patch_x = info.x_center as u32;
                    let patch_y = info.y_center as u32;

                    let x = 0;
                    let y = 0;
                    let width = patch_x;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopLeft,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = 0;
                    let width = info.width;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = 0;
                    let width = slice.width - info.width - patch_x;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopRight,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = patch_y;
                    let width = slice.width - info.width - patch_x;
                    let height = info.height;
                    map.insert(
                        NineSlice::RightCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = info.height + patch_y;
                    let width = slice.width - info.width - patch_x;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomRight,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = patch_y + info.height;
                    let width = info.width;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = 0;
                    let y = patch_y + info.height;
                    let width = patch_x;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomLeft,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = 0;
                    let y = patch_y;
                    let width = patch_x;
                    let height = info.height;
                    map.insert(
                        NineSlice::LeftCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = patch_y;
                    let width = info.width;
                    let height = info.height;
                    map.insert(
                        NineSlice::Center,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    map
                }),
                image,
            };

            slices.push(slice_image);
        }

        Ok(slices)
    }
}

/// Information about a single animation frame
#[derive(Debug, Clone)]
pub struct AsepriteFrameInfo {
    /// The delay of this frame in milliseconds
    pub delay_ms: usize,
}

/// A range of frames in an aseprite
pub struct AsepriteFrameRange<'a> {
    aseprite: &'a Aseprite,
    range: Range<u16>,
}

impl<'a> AsepriteFrameRange<'a> {
    /// Get the timings attached to each frame
    pub fn get_infos(&self) -> AseResult<&[AsepriteFrameInfo]> {
        Ok(&self.aseprite.frame_infos[self.range.start as usize..self.range.end as usize])
    }

    /// Get the images represented by this range
    pub fn get_images(&self) -> AseResult<Vec<RgbaImage>> {
        let mut frames = vec![];
        for frame in self.range.clone() {
            let image = image_for_frame(self.aseprite, frame)?;
            frames.push(image);
        }
        Ok(frames)
    }
}

fn image_for_frame(aseprite: &Aseprite, frame: u16) -> AseResult<RgbaImage> {
    let dim = aseprite.dimensions;
    let mut image = RgbaImage::new(dim.0 as u32, dim.1 as u32);
    for (_layer_id, layer) in &aseprite.layers {
        if !layer.is_visible() || layer.is_group() {
            continue;
        }

        let mut blank_cel: AsepriteCel;

        let cel = match layer.get_cel(frame as usize) {
            Ok(aseprite_cel) => aseprite_cel,
            Err(_) => {
                blank_cel = AsepriteCel {
                    x: 0.0,
                    y: 0.0,
                    opacity: 0,
                    raw_cel: RawAsepriteCel::Raw {
                        width: dim.0,
                        height: dim.1,
                        pixels: vec![
                            AsepritePixel::RGBA(AsepriteColor {
                                red: 0,
                                green: 0,
                                blue: 0,
                                alpha: 0,
                            });
                            (dim.0 * dim.1) as usize
                        ],
                    },
                };
                &blank_cel
            }
        };

        let mut write_to_image = |cel: &AsepriteCel,
                                  width: u16,
                                  height: u16,
                                  pixels: &[AsepritePixel]|
         -> AseResult<()> {
            for x in 0..width {
                for y in 0..height {
                    let pix_x = cel.x as i16 + x as i16;
                    let pix_y = cel.y as i16 + y as i16;

                    if pix_x < 0 || pix_y < 0 {
                        continue;
                    }
                    let raw_pixel = &pixels[(x + y * width) as usize];
                    let pixel = Rgba(
                        raw_pixel
                            .get_rgba(aseprite.palette.as_ref(), aseprite.transparent_palette)?,
                    );

                    image
                        .get_pixel_mut(pix_x as u32, pix_y as u32)
                        .blend(&pixel);
                }
            }
            Ok(())
        };

        match &cel.raw_cel {
            RawAsepriteCel::Raw {
                width,
                height,
                pixels,
            }
            | RawAsepriteCel::Compressed {
                width,
                height,
                pixels,
            } => {
                write_to_image(cel, *width, *height, pixels)?;
            }
            RawAsepriteCel::Linked { frame_position } => {
                match &layer.get_cel(*frame_position as usize)?.raw_cel {
                    RawAsepriteCel::Raw {
                        width,
                        height,
                        pixels,
                    }
                    | RawAsepriteCel::Compressed {
                        width,
                        height,
                        pixels,
                    } => {
                        write_to_image(cel, *width, *height, pixels)?;
                    }
                    RawAsepriteCel::Linked { frame_position } => {
                        error!("Tried to draw a linked cel twice!");
                        return Err(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                        ));
                    }
                }
            }
        }
    }

    Ok(image)
}
