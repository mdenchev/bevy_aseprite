use bevy::prelude::*;

use crate::Aseprite;

/// A component identifing a slice by name
#[derive(Component, Debug, Default)]
pub struct AsepriteSlice {
    name: String,
    flip_x: bool,
    flip_y: bool,
}

impl AsepriteSlice {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn flip_x(mut self) -> Self {
        self.flip_x = true;
        self
    }

    pub fn flip_y(mut self) -> Self {
        self.flip_y = true;
        self
    }

    pub fn set_flip_x(&mut self, flip_x: bool) {
        self.flip_x = flip_x;
    }

    pub fn set_flip_y(&mut self, flip_y: bool) {
        self.flip_y = flip_y;
    }
}

impl From<&str> for AsepriteSlice {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

pub fn insert_slice_sprite_sheet(
    mut cmd: Commands,
    aseprite_assets: Res<Assets<Aseprite>>,
    atlas_assets: Res<Assets<TextureAtlas>>,
    query: Query<(Entity, &AsepriteSlice, &Transform, &Handle<Aseprite>), Without<Sprite>>,
) {
    query
        .iter()
        .for_each(|(entity, slice, &transform, handle)| {
            let aseprite = match aseprite_assets.get(handle) {
                Some(aseprite) => aseprite,
                None => {
                    debug!("Aseprite asset not loaded");
                    return;
                }
            };

            let atlas_handle = match &aseprite.atlas {
                Some(atlas_handle) => atlas_handle,
                None => {
                    debug!("Aseprite atlas not loaded");
                    return;
                }
            };

            let atlas = match atlas_assets.get(atlas_handle) {
                Some(atlas) => atlas,
                None => {
                    debug!("Aseprite atlas is invalid");
                    return;
                }
            };

            let slice_data = aseprite
                .info
                .as_ref()
                // we know its loaded, because we found the atlas
                .expect("Aseprite info not loaded")
                .slices
                .get(&slice.name)
                .expect(format!("Slice {} not found", slice.name).as_str());

            let min = IVec2::new(slice_data.position_x, slice_data.position_y).as_vec2();
            let max = min + UVec2::new(slice_data.width, slice_data.height).as_vec2();

            cmd.entity(entity).insert(SpriteBundle {
                sprite: Sprite {
                    rect: Some(Rect::from_corners(min, max)),
                    flip_x: slice.flip_x,
                    flip_y: slice.flip_y,
                    ..default()
                },
                texture: atlas.texture.clone(),
                transform,
                ..default()
            });
        });
}
