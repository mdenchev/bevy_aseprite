use bevy::{
    asset::{AssetLoader, LoadedAsset},
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
            debug!("Loading aseprite at {:?}", load_context.path());
            let aseprite = Aseprite {
                path: load_context
                    .path()
                    .to_str()
                    .expect("Path is not valid unicode")
                    .to_owned(),
                info: None,
                frame_to_idx: vec![],
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
    mut asset_events: EventReader<AssetEvent<Aseprite>>,
    mut aseprites: ResMut<Assets<Aseprite>>,
    mut images: ResMut<Assets<Image>>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut existing: Query<&mut Handle<TextureAtlas>, With<AsepriteAnimation>>,
) {
    for event in asset_events.iter() {
        dbg!(event);
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                // Check for existince immutable so as not to trigger a Modified event
                match aseprites.get(handle) {
                    Some(aseprite) => match aseprite.atlas.is_some() {
                        true => continue,
                        false => {}
                    },
                    None => {
                        error!("Aseprite handle doesn't hold anything?");
                        continue;
                    }
                }

                // Get the created/modified aseprite
                let ase = match aseprites.get_mut(handle) {
                    Some(ase) => ase,
                    None => {
                        error!("Aseprite handle doesn't hold anything?");
                        continue;
                    }
                };

                let data = match reader::Aseprite::from_path(format!("assets/{}", &ase.path)) {
                    Ok(data) => data,
                    Err(err) => {
                        error!(
                            "Failed to load aseprite file {}. Reason: {}",
                            &ase.path, err
                        );
                        continue;
                    }
                };

                // Build out texture atlas
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
                    ase.frame_to_idx.push(atlas_idx);
                }
                let atlas_handle = atlases.add(atlas);

                // Updating any existing TextureAtlasSprites
                if let Some(prev_atlas_handle) = &ase.atlas {
                    dbg!("eists");
                    for mut cur_atlas_handle in existing.iter_mut() {
                        if &*cur_atlas_handle == prev_atlas_handle {
                            dbg!("match");
                            *cur_atlas_handle = atlas_handle.clone();
                        }
                    }
                }

                ase.info = Some(data.into());
                ase.atlas = Some(atlas_handle.clone());
            }
            AssetEvent::Removed { .. } => (),
        }
    }
}

pub(crate) fn insert_sprite_sheet(
    mut commands: Commands,
    aseprites: ResMut<Assets<Aseprite>>,
    mut query: Query<
        (
            Entity,
            &Transform,
            &Handle<Aseprite>,
            &mut AsepriteAnimation,
        ),
        Without<TextureAtlasSprite>,
    >,
) {
    for (entity, &transform, handle, _anim) in query.iter_mut() {
        // FIXME The first time the query runs the aseprite atlas might not be ready
        // so failing to find it is expected.
        let aseprite = match aseprites.get(handle) {
            Some(aseprite) => aseprite,
            None => {
                debug!("Aseprite handle invalid");
                continue;
            }
        };
        let atlas = match aseprite.atlas.clone() {
            Some(atlas) => atlas,
            None => {
                debug!("Aseprite atlas not ready");
                continue;
            }
        };
        commands.entity(entity).insert_bundle(SpriteSheetBundle {
            texture_atlas: atlas,
            transform,
            ..Default::default()
        });
    }
}
