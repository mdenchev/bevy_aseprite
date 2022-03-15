use std::path::{Path, PathBuf};

use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use bevy_aseprite_reader as reader;

use crate::{anim::AsepriteAnimation, Aseprite};

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
            let aseprite = Aseprite {
                data: Some(reader::Aseprite::from_bytes(bytes)?),
                info: None,
                atlas: None,
            };
            load_context.set_default_asset(LoadedAsset::new(aseprite));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ase", "aseprite"]
    }
}

pub(crate) fn process_load(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<Aseprite>>,
    mut aseprites: ResMut<Assets<Aseprite>>,
    mut images: ResMut<Assets<Image>>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
) {
    for event in asset_events.iter() {
        dbg!(&event);
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                // Get the created/modified aseprite
                let ase = match aseprites.get_mut(handle) {
                    Some(ase) => ase,
                    None => {
                        error!("Aseprite handle doesn't hold anything?");
                        continue;
                    }
                };
                let data = match ase.data.take() {
                    Some(data) => data,
                    None => {
                        error!("Ase data is empty");
                        continue;
                    }
                };

                // Build out texture atlas
                let frames = data.frames();
                let ase_images = frames
                    .get_for(&(0..frames.count() as u16))
                    .get_images()
                    .unwrap();

                let mut atlas = TextureAtlasBuilder::default();

                for (idx, image) in ase_images.into_iter().enumerate() {
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
                    let texture_handle = images.add(texture.clone());

                    atlas.add_texture(texture_handle, &texture);
                }
                let atlas_handle = match atlas.finish(&mut *images) {
                    Ok(atlas) => atlases.add(atlas),
                    Err(err) => {
                        error!("{:?}", err);
                        continue;
                    }
                };
                ase.info = Some(data.into());
                ase.atlas = Some(atlas_handle);

                //// If no sprite, add create and add as child of current entity
                //info!("Finished inserting entities for aseprite");
            }
            AssetEvent::Removed { .. } => (),
        }
    }
}

pub(crate) fn insert_sprite_sheet(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<Aseprite>>,
    mut aseprites: ResMut<Assets<Aseprite>>,
    mut images: ResMut<Assets<Image>>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<
        (
            Entity,
            &Transform,
            &Handle<Aseprite>,
            &mut AsepriteAnimation,
        ),
        Added<Handle<Aseprite>>,
    >,
) {
    for (entity, &transform, handle, anim) in query.iter_mut() {
        let aseprite = match aseprites.get(handle) {
            Some(aseprite) => aseprite,
            None => {
                error!("Aseprite handle invalid");
                continue;
            }
        };
        commands.entity(entity).insert_bundle(SpriteSheetBundle {
            texture_atlas: aseprite.atlas.clone().unwrap(),
            transform,
            ..Default::default()
        });
    }
}
