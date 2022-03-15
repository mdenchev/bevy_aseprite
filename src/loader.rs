use std::path::{Path, PathBuf};

use aseprite_reader::{Aseprite, AsepriteSliceImage};
use aseprite_reader2 as aseprite_reader;
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    anim::AnimState, AsepriteInfo, AsepriteSheetEntity,
};

#[derive(Debug, Default)]
pub struct AsepriteLoader;

impl AssetLoader for AsepriteLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            info!("Loading aseprite at {:?}", load_context.path());
            let aseprite = Aseprite::from_bytes(bytes)?;

            let frames = aseprite.frames();
            let images = frames
                .get_for(&(0..frames.count() as u16))
                .get_images()
                .unwrap();

            let mut aseprite_atlas = TextureAtlasBuilder::default();

            for (idx, image) in images.into_iter().enumerate() {
                let texture = Image::new(
                    Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    image.into_raw(),
                    TextureFormat::Rgba8UnormSrgb,
                );
                let label = format!("Frame{}", idx);
                let texture_handle =
                    load_context.set_labeled_asset(&label, LoadedAsset::new(texture.clone()));

                aseprite_atlas.add_texture(texture_handle.clone(), &texture);
            }

            load_context.set_default_asset(LoadedAsset::new(
                ,
                atlas: Atlas::Builder(aseprite_atlas),
            }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ase", "aseprite"]
    }
}

// TODO add support for hot reloading
pub(crate) fn check_aseprite_data(
    mut commands: Commands,
    mut aseprite_image_events: EventReader<AssetEvent<AsepriteInfo>>,
    mut aseprite_image_assets: ResMut<Assets<AsepriteInfo>>,
    mut texture_assets: ResMut<Assets<Image>>,
    mut texture_atlas_assets: ResMut<Assets<TextureAtlas>>,
    mut existing_aseprites: Query<
        (
            Entity,
            &Transform,
            Option<&AsepriteSheetEntity>,
            &mut Handle<AsepriteInfo>,
        ),
        With<AnimState>,
    >,
) {
    for event in aseprite_image_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                // Get the created/modified aseprite
                let ase = match aseprite_image_assets.get_mut(handle) {
                    Some(ase) => ase,
                    None => continue,
                };

                // If the atlas is already built, don't do anything
                if ase.atlas.is_handle() {
                    continue;
                }

                // Build the TextureAtlas -- Assets<Image> is not available in the loader so do it here
                ase.atlas
                    .load(&mut texture_atlas_assets, &mut texture_assets);

                // Check if the aseprite has previously been loaded -- ?
                let existing_ase = existing_aseprites
                    .iter_mut()
                    .find(|(_, _, _, query_handle)| &**query_handle == handle);

                let (ent, &transform, sheet, mut handle) = match existing_ase {
                    Some(v) => v,
                    None => continue,
                };

                // TODO Is this needed? Wouldn't the handle be invalid if the asset didn't exist?
                let image = match aseprite_image_assets.get(&*handle) {
                    Some(image) => image,
                    None => {
                        info!("Not aseprite image");
                        continue;
                    }
                };

                // TODO This also feel redundant as a similar check was done at the start of the function
                let atlas_handle = match image.atlas.get_atlas() {
                    Some(atlas_handle) => atlas_handle,
                    None => {
                        info!("No texture atlas");
                        continue;
                    }
                };

                // If no sprite, add create and add as child of current entity
                let sheet_entity = match &sheet {
                    Some(AsepriteSheetEntity(entity)) => *entity,
                    None => {
                        let entity = commands
                            .spawn_bundle(SpriteSheetBundle {
                                texture_atlas: atlas_handle.clone(),
                                transform,
                                ..Default::default()
                            })
                            .id();

                        commands
                            .entity(ent)
                            //.push_children(&[entity])
                            .insert(AsepriteSheetEntity(entity));

                        entity
                    }
                };

                commands
                    .entity(sheet_entity)
                    .insert(TextureAtlasSprite::new(0));

                info!("Finished inserting entities for aseprite");
            }
            AssetEvent::Removed { .. } | AssetEvent::Modified { .. } => (),
        }
    }
}

/// The path for loading a sprite
#[derive(Debug, Default, Component)]
pub struct AsepritePath(PathBuf);

impl AsepritePath {
    fn path(&self) -> &Path {
        &self.0.as_path()
    }
}

// This is used so you don't need to load the aseprite when creating the bundle
/* TODO maybe get rid of this? Though it does provide cool ergonomics as you
    can create a bundle without an assetserver. But then again why wouldn't
    you have an assetserver..
*/
pub(crate) fn load_aseprites(
    mut commands: Commands,
    asset_server_settings_folder: Res<AssetServerSettings>,
    asset_server: Res<AssetServer>,
    new_aseprites: Query<(Entity, &AsepritePath), Added<AsepritePath>>,
) {
    for (entity, ase_path) in new_aseprites.iter() {
        let path = ase_path
            .path()
            .strip_prefix(&asset_server_settings_folder.asset_folder)
            .unwrap();

        let handle: Handle<AsepriteInfo> = asset_server.load(path);

        commands.entity(entity).insert(handle);
    }
}
