#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.MD")]

mod anim;
mod loader;

use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use anim::{AnimState, AsepriteTag, update_animations};
use bevy_aseprite_reader::{Aseprite, AsepriteSliceImage, NineSlice};
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
        app.add_asset::<AsepriteBevy>()
            .add_asset_loader(AsepriteLoader)
            .add_system(check_aseprite_data.before(AsepriteSystems::UpdateAnim))
            .add_system(load_aseprites)
            .add_system(switch_tag.before(AsepriteSystems::UpdateAnim))
            .add_system(update_animations.label(AsepriteSystems::UpdateAnim))
            .add_system(update_spritesheet_anim.after(AsepriteSystems::UpdateAnim));
    }
}

fn switch_tag(
    aseprite_image_assets: Res<Assets<AsepriteBevy>>,
    mut aseprites_query: Query<
        (&Handle<AsepriteBevy>, &mut AsepriteAnimation),
        Changed<AsepriteAnimation>,
    >,
) {
}

#[derive(Component)]
pub(crate) struct AsepriteSheetEntity(Entity);

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "b29abc81-6179-42e4-b696-3a5a52f44f73"]
/// The loaded aseprite file without image data
pub struct AsepriteInfo {
    dimensions: (u16, u16),
    tags: HashMap<String, AsepriteTag>,
    slices: HashMap<String, AsepriteSlice>,
    frame_count: usize,
    palette: Option<AsepritePalette>,
    transparent_palette: Option<u8>,
    frame_infos: Vec<AsepriteFrameInfo>,
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

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub aseprite_path: AsepritePath,
    pub animation: AnimState,
    pub atlas: TextureAtlas,
    pub handle: Handle<AsepriteInfo>,
}