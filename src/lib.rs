#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.MD")]

pub mod anim;
mod loader;

use anim::AsepriteAnimation;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;

use bevy::utils::HashMap;
use bevy_aseprite_reader as reader;

pub use bevy::sprite::TextureAtlasBuilder;
pub use bevy_aseprite_derive::aseprite;
use reader::AsepriteInfo;

pub struct AsepritePlugin;

#[derive(Debug, SystemLabel, Clone, Hash, PartialEq, Eq)]
enum AsepriteSystems {
    InsertSpriteSheet,
}

impl Plugin for AsepritePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<AsepritePath>()
            .add_asset::<Aseprite>()
            .add_asset_loader(loader::AsepriteLoader)
            .init_resource::<PathToAseH>()
            .add_system(loader::process_load)
            .add_system(loader::insert_sprite_sheet.label(AsepriteSystems::InsertSpriteSheet))
            .add_system(anim::update_animations.after(AsepriteSystems::InsertSpriteSheet));
    }
}

#[derive(Debug, Default)]
pub struct PathToAseH(pub HashMap<String, Handle<Aseprite>>);

#[derive(Debug, Clone, TypeUuid)]
// TODO change uuid
#[uuid = "b29abc81-6179-42e4-b696-3a5a52f44f74"]
pub struct Aseprite {
    info: AsepriteInfo,
    // TextureAtlasBuilder might shift the index order when building so
    // we keep a mapping of frame# -> atlas index here
    frame_to_idx: Vec<usize>,
    // Atlas that gets built from the frame info of the aseprite file
    atlas: Handle<TextureAtlas>,
}

/// Path to the aseprite file
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "b29abc81-6179-42e4-b696-3a5a52f44f73"]
pub struct AsepritePath {
    path: String,
}

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub animation: AsepriteAnimation,
    pub aseprite: Handle<AsepritePath>,
}
