#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.MD")]

pub mod anim;
mod loader;

use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use anim::{AsepriteAnimation, AsepriteTag};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    utils::HashMap,
};
use bevy_aseprite_reader as reader;

pub use bevy::sprite::TextureAtlasBuilder;
pub use bevy_aseprite_derive::aseprite;
use reader::AsepriteInfo;

pub struct AsepritePlugin;

#[derive(Debug, SystemLabel, Clone, Hash, PartialEq, Eq)]
enum AsepriteSystems {
    InsertSpriteSheet,
    UpdateAnim,
}

impl Plugin for AsepritePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<Aseprite>()
            .add_asset_loader(loader::AsepriteLoader)
            .add_system(loader::process_load)
            .add_system(loader::insert_sprite_sheet.label(AsepriteSystems::InsertSpriteSheet))
            .add_system(anim::update_animations.after(AsepriteSystems::InsertSpriteSheet));
    }
}
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "b29abc81-6179-42e4-b696-3a5a52f44f73"]
pub struct Aseprite {
    // Data is dropped after the atlas is built
    data: Option<reader::Aseprite>,
    // Info stores data such as tags and slices
    info: Option<AsepriteInfo>,
    frame_handles: Vec<Handle<Image>>,
    // Atlas that gets built from the frame info of the aseprite file
    atlas: Option<Handle<TextureAtlas>>,
}

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub animation: AsepriteAnimation,
    pub aseprite: Handle<Aseprite>,
}
