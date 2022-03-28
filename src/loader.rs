use bevy::{
    asset::{AssetLoader, LoadedAsset},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use bevy_aseprite_reader as reader;

use crate::{anim::AsepriteAnimation, Aseprite, AsepritePath, PathToAseH};

#[derive(Debug, Default)]
pub struct AsepriteLoader;

impl AssetLoader for AsepriteLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            debug!("Loading aseprite at {:?}", load_context.path());
            let aseprite_path = AsepritePath {
                path: load_context
                    .path()
                    .to_str()
                    .expect("Path is not valid unicode")
                    .to_owned(),
            };
            load_context.set_default_asset(LoadedAsset::new(aseprite_path));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ase", "aseprite"]
    }
}

/// The actual loading happens on AssetEvents.
pub(crate) fn process_load(
    mut asset_events: EventReader<AssetEvent<AsepritePath>>,
    mut path_to_ase_h: ResMut<PathToAseH>,
    aseprite_paths: Res<Assets<AsepritePath>>,
    mut aseprites: ResMut<Assets<Aseprite>>,
    mut images: ResMut<Assets<Image>>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut existing: Query<&mut Handle<TextureAtlas>, With<AsepriteAnimation>>,
) {
    for event in asset_events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                let path_handle = handle;

                let aseprite_path = match aseprite_paths.get(path_handle) {
                    Some(aseprite_path) => aseprite_path,
                    None => {
                        error!("AsepritePath handle doesn't hold anything");
                        continue;
                    }
                };

                let data =
                    match reader::Aseprite::from_path(format!("assets/{}", &aseprite_path.path)) {
                        Ok(data) => data,
                        Err(err) => {
                            error!(
                                "Failed to load aseprite file {}. Reason: {}",
                                &aseprite_path.path, err
                            );
                            continue;
                        }
                    };

                // Build out texture atlas
                let mut frame_to_idx = vec![];
                let frames = data.frames();
                let ase_images = frames
                    .get_for(&(0..frames.count() as u16))
                    .get_images()
                    .unwrap();

                let mut frame_handles = vec![];
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
                    let _label = format!("Frame{}", idx);
                    let texture_handle = images.add(texture.clone());
                    frame_handles.push(texture_handle.as_weak());

                    atlas.add_texture(texture_handle, &texture);
                }
                let atlas = match atlas.finish(&mut *images) {
                    Ok(atlas) => atlas,
                    Err(err) => {
                        error!("{:?}", err);
                        continue;
                    }
                };
                for handle in frame_handles {
                    let atlas_idx = atlas.get_texture_index(&handle).unwrap();
                    frame_to_idx.push(atlas_idx);
                }
                let atlas_handle = atlases.add(atlas);

                let aseprite_handle = aseprites.add(Aseprite {
                    info: data.into(),
                    frame_to_idx,
                    atlas: atlas_handle,
                });
                path_to_ase_h
                    .0
                    .insert(aseprite_path.path.clone(), aseprite_handle);
            },
            AssetEvent::Removed { .. } => {
                // todo
                dbg!("Removed");
            }
        };

        match event {
            AssetEvent::Modified { handle } => {
                // NEW
                // Get the created/modified aseprite
                let ase = match aseprites.get_mut(path_handle) {
                    Some(ase) => ase,
                    None => {
                        error!("Aseprite handle doesn't hold anything");
                        continue;
                    }
                };

                // Updating any existing TextureAtlasSprites
                let prev_atlas_handle = &ase.atlas;
                for mut cur_atlas_handle in existing.iter_mut() {
                    if &*cur_atlas_handle == prev_atlas_handle {
                        dbg!("match");
                        *cur_atlas_handle = atlas_handle.clone();
                    }
                }

                //let ase = match path_to_ase.0.get(&aseprite_path.path) {
                //    Some(ase_h) => todo!(),
                //    None => {
                //        error!("AsepritePath handle doesn't hold anything");
                //        continue;
                //    }
                //};
            }
            AssetEvent::Removed { .. } => (),
        }
    }
}

pub(crate) fn insert_sprite_sheet(
    mut commands: Commands,
    aseprite_paths: Res<Assets<AsepritePath>>,
    mut path_to_ase_h: ResMut<PathToAseH>,
    aseprites: Res<Assets<Aseprite>>,
    mut query: Query<(Entity, &Transform, &Handle<AsepritePath>), Without<TextureAtlasSprite>>,
) {
    for (entity, &transform, path_handle) in query.iter_mut() {
        let aseprite_path = match aseprite_paths.get(path_handle) {
            Some(path) => path,
            None => {
                error!("AsepritePath handle point to nothing");
                continue;
            }
        };
        let aseprite_handle = match path_to_ase_h.0.get(&aseprite_path.path) {
            Some(aseprite_handle) => aseprite_handle,
            None => {
                error!("Aseprite path not found in map to handles");
                continue;
            }
        };
        let aseprite = match aseprites.get(aseprite_handle) {
            Some(aseprite) => aseprite,
            None => {
                error!("Aseprite invalid");
                continue;
            }
        };
        commands
            .entity(entity)
            .insert_bundle(SpriteSheetBundle {
                texture_atlas: aseprite.atlas.clone(),
                transform,
                ..Default::default()
            })
            .insert(aseprite_handle.clone());
    }
}
