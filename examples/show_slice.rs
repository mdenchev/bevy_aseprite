use bevy::prelude::*;
use bevy_aseprite::{slice::AsepriteSlice, AsepritePlugin, AsepriteSliceBundle};

pub fn main() {
    App::new()
        // nearest filtering
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(AsepritePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut cmd: Commands, server: Res<AssetServer>) {
    cmd.spawn(Camera2dBundle {
        transform: Transform::default().with_scale(Vec3::splat(0.2)),
        ..default()
    });

    cmd.spawn(AsepriteSliceBundle {
        slice: "ghost_blue".into(),
        aseprite: server.load("ghost_slices.aseprite"),
        transform: Transform::from_translation(Vec3::new(-32., 0., 0.)),
        ..Default::default()
    });

    cmd.spawn(AsepriteSliceBundle {
        slice: AsepriteSlice::new("ghost_red").flip_x(),
        aseprite: server.load("ghost_slices.aseprite"),
        transform: Transform::from_translation(Vec3::new(32., 0., 0.)),
        ..Default::default()
    });
}
