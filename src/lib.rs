#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.MD")]

mod anim;
mod loader;

use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use anim::{AsepriteAnimationState, AsepriteTag};
use aseprite_reader::{Aseprite, AsepriteSliceImage, NineSlice};
use aseprite_reader2 as aseprite_reader;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    utils::HashMap,
};

pub use bevy::sprite::TextureAtlasBuilder;
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

macro_rules! take_or_continue {
    ($opt:expr) => {
        match $opt {
            Some(val) => val,
            None => continue,
        }
    };
}

fn update_animations(
    time: Res<Time>,
    aseprite_image_assets: Res<Assets<AsepriteImage>>,
    mut aseprites_query: Query<(
        &Handle<AsepriteImage>,
        &mut AsepriteAnimationState,
    )>,
) {
    for (handle, mut anim_state) in aseprites_query.iter_mut() {
        if anim_state.is_paused() {
            continue;
        }

        let image = take_or_continue!(aseprite_image_assets.get(handle));
        let mut delta_millis = Some(time.delta().as_millis() as u64);

        loop {
            // let (current_frame_idx, forward, rest_time) = match &mut *aseprite_animation_state {
            //     AsepriteAnimationState::Paused { .. } => break,
            //     AsepriteAnimationState::Playing {
            //         current_frame,
            //         forward,
            //         time_elapsed,
            //     } => (current_frame, forward, time_elapsed),
            // };

            let frame_info =
                if let Some(info) = image.aseprite.frame_infos().get(anim_state.current_frame) {
                    info
                } else {
                    break;
                };

            if let Some(added_time) = delta_millis.take() {
                anim_state.time_elapsed += added_time;
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
        (&Handle<AsepriteImage>, &mut AsepriteAnimation),
        Changed<AsepriteAnimation>,
    >,
) {
    for (handle, mut animation) in aseprites_query.iter_mut() {
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

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub aseprite_path: AsepritePath,
    pub animation: AsepriteAnimation,
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
