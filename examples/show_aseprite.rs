use bevy::prelude::*;
use bevy_aseprite::{anim::AsepriteAnimation, AsepriteBundle, AsepritePlugin};

#[derive(Component, Clone, Copy, Debug)]
struct CrowTag;

#[derive(Component, Clone, Copy, Debug)]
struct PlayerTag;

mod sprites {
    
    use bevy_aseprite::aseprite;

    // https://meitdev.itch.io/crow
    aseprite!(pub Crow, "assets/crow.aseprite");
    // https://shubibubi.itch.io/cozy-people
    aseprite!(pub Player, "assets/player.ase");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AsepritePlugin)
        .add_startup_system(setup)
        .add_system(toggle_sprite)
        .add_system(change_animation)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    let font = asset_server.load("Share-Regular.ttf");

    let text_style = TextStyle {
        font,
        font_size: 30.,
        ..Default::default()
    };

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: asset_server.load("crow.aseprite"),
            animation: AsepriteAnimation::from("flap_wings"),
            transform: Transform {
                scale: Vec3::splat(4.),
                translation: Vec3::new(0., 150., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CrowTag);
    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: asset_server.load("player.ase"),
            transform: Transform {
                scale: Vec3::splat(4.),
                translation: Vec3::new(0., -200., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PlayerTag);
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
            sections: vec![
                TextSection {
                    value: String::from("The crow was made by "),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("meitdev"),
                    style: TextStyle {
                        color: Color::LIME_GREEN,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from(" on itch.io"),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., 300., 0.)),
        ..Default::default()
    });
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
            sections: vec![
                TextSection {
                    value: String::from("The human was made by "),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("shubibubi"),
                    style: TextStyle {
                        color: Color::BLUE,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from(" on itch.io"),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., -100., 0.)),
        ..Default::default()
    });
}

fn toggle_sprite(keys: Res<Input<KeyCode>>, mut aseprites: Query<&mut AsepriteAnimation>) {
    if keys.just_pressed(KeyCode::Space) {
        for mut state in aseprites.iter_mut() {
            state.toggle();
        }
    }
}

fn change_animation(
    keys: Res<Input<KeyCode>>,
    mut aseprites: QuerySet<(
        QueryState<&mut AsepriteAnimation, With<CrowTag>>,
        QueryState<&mut AsepriteAnimation, With<PlayerTag>>,
    )>
) {
    if keys.just_pressed(KeyCode::Key1) {
        for mut crow_anim in aseprites.q0().iter_mut() {
            *crow_anim = AsepriteAnimation::from("flap_wings");
        }
        for mut player_anim in aseprites.q1().iter_mut() {
            *player_anim = AsepriteAnimation::from("left_walk");
        }
    }
    if keys.just_pressed(KeyCode::Key2) {
        for mut crow_anim in aseprites.q0().iter_mut() {
            *crow_anim = AsepriteAnimation::from("groove");
        }
        for mut player_anim in aseprites.q1().iter_mut() {
            *player_anim = AsepriteAnimation::from("right_walk");
        }
    }
}
