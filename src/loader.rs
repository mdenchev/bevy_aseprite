use crate::{anim::AsepriteAnimation, error, Aseprite};
use bevy::{
    asset::{AssetLoader, AsyncReadExt},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_aseprite_reader as reader;

#[derive(Debug, Default)]
pub struct AsepriteLoader;

impl AssetLoader for AsepriteLoader {
    type Asset = Aseprite;
    type Settings = ();
    type Error = error::AsepriteLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            debug!("Loading aseprite at {:?}", load_context.path());

            let mut buffer = vec![];
            let _ = reader.read_to_end(&mut buffer).await?;
            let data = Some(reader::Aseprite::from_bytes(buffer)?);

            Ok(Aseprite {
                data,
                info: None,
                frame_to_idx: vec![],
                atlas: None,
                image: None,
            })
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
    mut atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    asset_events.read().for_each(|event| {
        if let AssetEvent::Added { id } | AssetEvent::Modified { id } = event {
            // Get the created/modified aseprite
            match aseprites.get(*id) {
                Some(aseprite) => match aseprite.atlas.is_some() {
                    true => return,
                    false => {}
                },
                None => {
                    error!("Aseprite handle doesn't hold anything?");
                    return;
                }
            }

            let ase = match aseprites.get_mut(*id) {
                Some(ase) => ase,
                None => {
                    error!("Aseprite handle doesn't hold anything?");
                    return;
                }
            };
            let data = match ase.data.take() {
                Some(data) => data,
                None => {
                    error!("Ase data is empty");
                    return;
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

            let textures = ase_images
                .into_iter()
                .map(|image| {
                    Image::new(
                        Extent3d {
                            width: image.width(),
                            height: image.height(),
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        image.into_raw(),
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::MAIN_WORLD,
                    )
                })
                .collect::<Vec<_>>();
            for texture in textures.iter() {
                let texture_handle = images.add(texture.clone());
                frame_handles.push(texture_handle.clone_weak());
                atlas.add_texture(Some(texture_handle.id()), texture);
            }
            let (atlas, image) = match atlas.finish() {
                Ok(atlas) => atlas,
                Err(err) => {
                    error!("{:?}", err);
                    return;
                }
            };
            for handle in frame_handles {
                let atlas_idx = atlas.get_texture_index(&handle).unwrap();
                ase.frame_to_idx.push(atlas_idx);
            }
            let atlas_handle = atlases.add(atlas);
            let image_handle = images.add(image);
            ase.info = Some(data.into());
            ase.atlas = Some(atlas_handle);
            ase.image = Some(image_handle);
        }
    });
}

pub(crate) fn insert_sprite_sheet(
    mut commands: Commands,
    aseprites: ResMut<Assets<Aseprite>>,
    mut query: Query<
        (Entity, &Transform, &Handle<Aseprite>),
        (Without<TextureAtlas>, With<AsepriteAnimation>),
    >,
) {
    for (entity, &transform, handle) in query.iter_mut() {
        // FIXME The first time the query runs the aseprite atlas might not be ready
        // so failing to find it is expected.
        let aseprite = match aseprites.get(handle) {
            Some(aseprite) => aseprite,
            None => {
                debug!("Aseprite handle invalid");
                continue;
            }
        };
        let mut atlas = match aseprite.atlas.clone() {
            Some(atlas) => atlas,
            None => {
                debug!("Aseprite atlas not ready");
                continue;
            }
        };
        let image = match aseprite.image.clone() {
            Some(image) => image,
            None => {
                debug!("Aseprite image not ready");
                continue;
            }
        };
        commands.entity(entity).insert(SpriteSheetBundle {
            atlas: TextureAtlas {
                layout: atlas,
                index: 0,
            },
            texture: image,
            transform,
            ..Default::default()
        });
    }
}
