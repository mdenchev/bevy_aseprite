#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.MD")]

mod loader;

use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use aseprite_reader::{Aseprite, AsepriteSliceImage, NineSlice};
use aseprite_reader2 as aseprite_reader;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    utils::HashMap,
};

use bevy::sprite::TextureAtlasBuilder;
pub use bevy_aseprite_derive::aseprite;
use loader::{check_aseprite_data, load_aseprites, AsepriteLoader};

/// The required plugin to fully use your aseprite files
pub struct AsepritePlugin;

#[derive(Debug, SystemLabel, Clone, Hash, PartialEq, Eq)]
enum AsepriteSystems {
    UpdateAnim,
}

impl Plugin for AsepritePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<AsepriteImage>()
            .add_asset_loader(AsepriteLoader)
            .add_system(check_aseprite_data.before(AsepriteSystems::UpdateAnim))
            .add_system(load_aseprites)
            .add_system(switch_tag.before(AsepriteSystems::UpdateAnim))
            .add_system(update_animations.label(AsepriteSystems::UpdateAnim))
            .add_system(update_spritesheet_anim.after(AsepriteSystems::UpdateAnim));
    }
}

fn update_animations(
    time: Res<Time>,
    aseprite_image_assets: Res<Assets<AsepriteImage>>,
    mut aseprites_query: Query<(
        &Handle<AsepriteImage>,
        &AsepriteAnimation,
        &mut AsepriteAnimationState,
    )>,
) {
    for (aseprite_handle, aseprite_animation, mut aseprite_animation_state) in
        aseprites_query.iter_mut()
    {
        let image = if let Some(image) = aseprite_image_assets.get(aseprite_handle.clone_weak()) {
            image
        } else {
            continue;
        };

        let mut added_time = Some(time.delta().as_millis() as u64);

        loop {
            let (current_frame_idx, forward, rest_time) = match &mut *aseprite_animation_state {
                AsepriteAnimationState::Paused { .. } => break,
                AsepriteAnimationState::Playing {
                    current_frame,
                    forward,
                    time_elapsed,
                } => (current_frame, forward, time_elapsed),
            };

            let frame_info =
                if let Some(info) = image.aseprite.frame_infos().get(*current_frame_idx) {
                    info
                } else {
                    break;
                };

            if let Some(added_time) = added_time.take() {
                *rest_time += added_time;
            }

            if *rest_time >= frame_info.delay_ms as u64 {
                *rest_time -= frame_info.delay_ms as u64;

                let (next_frame_idx, switch_direction) =
                    aseprite_animation.get_next_frame(image, *current_frame_idx, *forward);

                *current_frame_idx = next_frame_idx;
                if switch_direction {
                    *forward = !*forward;
                }
            } else {
                break;
            }
        }
    }
}

fn switch_tag(
    aseprite_image_assets: Res<Assets<AsepriteImage>>,
    mut aseprites_query: Query<
        (
            &Handle<AsepriteImage>,
            &AsepriteAnimation,
            &mut AsepriteAnimationState,
        ),
        Changed<AsepriteAnimation>,
    >,
) {
    for (aseprite_handle, aseprite_animation, mut aseprite_animation_state) in
        aseprites_query.iter_mut()
    {
        let image = match aseprite_image_assets.get(aseprite_handle) {
            Some(image) => image,
            None => continue,
        };

        let (current_frame_idx, _forward, rest_time) = match &mut *aseprite_animation_state {
            AsepriteAnimationState::Paused { .. } => break,
            AsepriteAnimationState::Playing {
                current_frame,
                forward,
                time_elapsed,
            } => (current_frame, forward, time_elapsed),
        };

        *rest_time = 0;
        *current_frame_idx = aseprite_animation.get_first_frame(image);
    }
}

// Update the
fn update_spritesheet_anim(
    aseprite_assets: Res<Assets<AsepriteImage>>,
    texture_atlas_assets: Res<Assets<TextureAtlas>>,
    mut atlas_sprite: Query<(&Handle<TextureAtlas>, &mut TextureAtlasSprite)>,
    aseprites_query: Query<
        (
            &AsepriteAnimationState,
            &Handle<AsepriteImage>,
            &AsepriteSheetEntity,
        ),
        Changed<AsepriteAnimationState>,
    >,
) {
    for (aseprite_animation_state, aseprite_handle, sheet_entity) in aseprites_query.iter() {
        let frame_idx = aseprite_animation_state.get_current_frame();

        let aseprite = if let Some(aseprite) = aseprite_assets.get(aseprite_handle.clone_weak()) {
            aseprite
        } else {
            continue;
        };

        let texture = if let Some(tex) = aseprite.frames.get(frame_idx) {
            tex
        } else {
            continue;
        };

        let (atlas_handle, mut atlas_sprite) =
            if let Ok(sprite) = atlas_sprite.get_mut(sheet_entity.0) {
                sprite
            } else {
                continue;
            };

        let atlas = if let Some(atlas) = texture_atlas_assets.get(atlas_handle.clone_weak()) {
            atlas
        } else {
            continue;
        };

        atlas_sprite.index = if let Some(idx) = atlas.get_texture_index(&texture) {
            if atlas_sprite.index == idx {
                continue;
            }
            idx
        } else {
            continue;
        };
    }
}

#[derive(Component)]
pub(crate) struct AsepriteSheetEntity(Entity);

/// A tag representing an animation
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct AsepriteTag(&'static str);

impl std::ops::Deref for AsepriteTag {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsepriteTag {
    /// Create a new tag
    pub const fn new(id: &'static str) -> AsepriteTag {
        AsepriteTag(id)
    }
}

// TODO I don't think this is used anywhere currently.
#[derive(Debug, Default, Copy, Clone)]
pub struct AsepriteSlice(&'static str);

impl std::ops::Deref for AsepriteSlice {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsepriteSlice {
    /// Create a new tag
    pub const fn new(id: &'static str) -> AsepriteSlice {
        AsepriteSlice(id)
    }
}

#[derive(Debug)]
enum Atlas {
    Builder(TextureAtlasBuilder),
    Handle(Handle<TextureAtlas>),
}

impl Atlas {
    fn load(&mut self, texture_atlases: &mut Assets<TextureAtlas>, textures: &mut Assets<Image>) {
        match self {
            Atlas::Builder(_) => {
                if let Atlas::Builder(builder) =
                    std::mem::replace(self, Atlas::Handle(Handle::default()))
                {
                    let texture_atlas = builder.finish(textures).unwrap();
                    let handle = texture_atlases.add(texture_atlas);
                    *self = Atlas::Handle(handle);
                }
            }
            Atlas::Handle(_) => (),
        }
    }

    fn get_atlas(&self) -> Option<&Handle<TextureAtlas>> {
        match self {
            Atlas::Builder(_) => None,
            Atlas::Handle(handle) => Some(handle),
        }
    }

    /// Returns `true` if the atlas is [`Handle`].
    fn is_handle(&self) -> bool {
        matches!(self, Self::Handle(..))
    }
}

#[derive(Debug)]
/// The textures in a slice
#[allow(missing_docs)]
pub struct AsepriteSliceTextures {
    pub texture_handle: Handle<Image>,
    pub nine_patch_handles: Option<HashMap<NineSlice, Handle<Image>>>,
}

/// An internal type containing the different images the associated aseprite file has
#[derive(Debug, TypeUuid)]
#[uuid = "8da03a16-d6d5-42c3-b4c7-fc68f53e0769"]
pub struct AsepriteImage {
    aseprite: Aseprite,
    atlas: Atlas,
    frames: Vec<Handle<Image>>,
    slices: HashMap<String, AsepriteSliceTextures>,
}

impl AsepriteImage {
    /// Get the texture handles associated to the frames
    pub fn frames(&self) -> &[Handle<Image>] {
        &self.frames
    }

    /// Get the slice handles associated to this aseprite
    pub fn slices(&self) -> &HashMap<String, AsepriteSliceTextures> {
        &self.slices
    }

    /// Get the underlying aseprite definition
    pub fn aseprite(&self) -> &Aseprite {
        &self.aseprite
    }
}

/// All the info about a specific aseprite
#[derive(Debug, Default, Component)]
pub struct AsepriteInfo {
    /// The path to the aseprite file, relative to the crate root
    pub path: PathBuf,
}

impl AsepriteInfo {
    fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsepriteAnimationState {
    Playing,
    Paused
}

#[derive(Debug, Component, PartialEq, Eq)]
/// An aseprite animation
pub struct AsepriteAnimation {
    pub tag: Option<&'static str>,
    pub state: AsepriteAnimationState,
    pub current_frame: usize,
    pub forward: bool,
    pub time_elapsed: u64,
}

impl Default for AsepriteAnimation {
    fn default() -> Self {
    }
}

impl AsepriteAnimation {
    /// Return the first frame of this tag
    pub fn get_first_frame(&self, aseprite: &AsepriteImage) -> usize {
        match self {
            AsepriteAnimation::Tag {
                tag: AsepriteTag(name),
            } => {
                let tags = aseprite.aseprite.tags();
                let tag = match tags.get_by_name(name) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", name);
                        return 0;
                    }
                };

                let range = tag.frames.clone();
                range.start as usize
            }
            _ => 0,
        }
    }

    /// Calculate the next frame from the current one
    pub fn get_next_frame(
        &self,
        aseprite: &AsepriteImage,
        current_frame: usize,
        forward: bool,
    ) -> (usize, bool) {
        match self {
            AsepriteAnimation::Tag {
                tag: AsepriteTag(name),
            } => {
                let tags = aseprite.aseprite.tags();
                let tag = match tags.get_by_name(name) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", name);
                        return (0, false);
                    }
                };

                let range = tag.frames.clone();
                match tag.animation_direction {
                    aseprite_reader::raw::AsepriteAnimationDirection::Forward => {
                        let next_frame = current_frame + 1;
                        if range.contains(&(next_frame as u16)) {
                            return (next_frame, false);
                        } else {
                            return (range.start as usize, false);
                        }
                    }
                    aseprite_reader::raw::AsepriteAnimationDirection::Reverse => {
                        let next_frame = current_frame.checked_sub(1);
                        if let Some(next_frame) = next_frame {
                            if range.contains(&(next_frame as u16)) {
                                return (next_frame, false);
                            }
                        }
                        return (range.end as usize, false);
                    }
                    aseprite_reader::raw::AsepriteAnimationDirection::PingPong => {
                        if forward {
                            let next_frame = current_frame + 1;
                            if range.contains(&(next_frame as u16)) {
                                return (next_frame, false);
                            } else {
                                return (next_frame.saturating_sub(1), true);
                            }
                        } else {
                            let next_frame = current_frame.checked_sub(1);
                            if let Some(next_frame) = next_frame {
                                if range.contains(&(next_frame as u16)) {
                                    return (next_frame, false);
                                }
                            }
                            return (current_frame + 1, true);
                        }
                    }
                }
            }
            AsepriteAnimation::None => (0, false),
        }
    }

    /// Check if the current animation tag is the one provided
    pub fn is_tag(&self, tag: AsepriteTag) -> bool {
        self == &Self::Tag { tag }
    }

    /// Get the current frame to be shown
    pub fn get_current_frame(&self) -> usize {
        match self {
            Self::Playing { current_frame, .. } => *current_frame,
            Self::Paused {
                current_frame: frame,
                ..
            } => *frame,
        }
    }

    /// Start playing an animation
    pub fn start(&mut self) {
        match self {
            AsepriteAnimationState::Playing { .. } => (),
            AsepriteAnimationState::Paused {
                current_frame,
                forward,
            } => {
                *self = AsepriteAnimationState::Playing {
                    current_frame: *current_frame,
                    forward: *forward,
                    time_elapsed: 0,
                }
            }
        }
    }

    /// Pause the current animation
    pub fn pause(&mut self) {
        match self {
            AsepriteAnimationState::Paused { .. } => (),
            AsepriteAnimationState::Playing {
                current_frame,
                forward,
                ..
            } => {
                *self = AsepriteAnimationState::Paused {
                    current_frame: *current_frame,
                    forward: *forward,
                }
            }
        }
    }

    /// Returns `true` if the aseprite_animation_state is [`Playing`].
    pub fn is_playing(&self) -> bool {
        matches!(self, Self::Playing { .. })
    }

    /// Returns `true` if the aseprite_animation_state is [`Paused`].
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused { .. })
    }

    /// Toggle state between playing and pausing
    pub fn toggle(&mut self) {
        match self {
            AsepriteAnimationState::Playing {
                current_frame,
                forward,
                ..
            } => {
                *self = Self::Paused {
                    current_frame: *current_frame,
                    forward: *forward,
                };
            }
            AsepriteAnimationState::Paused {
                current_frame,
                forward,
            } => {
                *self = Self::Playing {
                    current_frame: *current_frame,
                    forward: *forward,
                    time_elapsed: 0,
                };
            }
        }
    }
}

impl From<AsepriteTag> for AsepriteAnimation {
    fn from(tag: AsepriteTag) -> Self {
        AsepriteAnimation::Tag { tag }
    }
}

impl Default for AsepriteAnimationState {
    fn default() -> Self {
        AsepriteAnimationState::Playing {
            current_frame: 0,
            forward: true,
            time_elapsed: 0,
        }
    }
}

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    // TODO what's the point of this?
    pub aseprite: AsepriteInfo,
    pub animation: AsepriteAnimation,
    pub animation_state: AsepriteAnimationState,
    pub handle: Handle<AsepriteImage>,
}

/// Helper methods to get the label for a specific slice
pub trait AsepriteSliceName {
    /// Label for the whole slice
    fn label(&self) -> String;

    /// Label for just a part of the slice as given by `nine_slice`
    fn label_with_nine_slice(&self, nine_slice: NineSlice) -> String;
}

impl AsepriteSliceName for aseprite_reader::AsepriteSlice {
    fn label(&self) -> String {
        format!("Slices/{}", self.name)
    }

    fn label_with_nine_slice(&self, nine_slice: NineSlice) -> String {
        format!("Slices/{}/{:?}", self.name, nine_slice)
    }
}

impl AsepriteSliceName for AsepriteSlice {
    fn label(&self) -> String {
        format!("Slices/{}", self.0)
    }

    fn label_with_nine_slice(&self, nine_slice: NineSlice) -> String {
        format!("Slices/{}/{:?}", self.0, nine_slice)
    }
}
