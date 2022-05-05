use bevy::prelude::*;
use bevy_aseprite::{anim::AsepriteAnimation, AsepriteBundle, AsepritePlugin};

#[derive(Component, Clone, Copy, Debug)]
struct CrowTag;

#[derive(Component, Clone, Copy, Debug)]
struct PlayerTag;

mod sprites {
    use bevy_aseprite::aseprite;

    // https://meitdev.itch.io/crow
    aseprite!(pub Crow, "crow.aseprite");
    // https://shubibubi.itch.io/cozy-people
    aseprite!(pub Player, "player.ase");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AsepritePlugin)
        .add_startup_system(setup)
        .add_startup_system(setup_text)
        .add_system(change_animation)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: asset_server.load(sprites::Crow::PATH),
            animation: AsepriteAnimation::from(sprites::Crow::tags::FLAP_WINGS),
            transform: Transform {
                scale: Vec3::splat(4.),
                translation: Vec3::new(0., 80., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(CrowTag);

    commands
        .spawn_bundle(AsepriteBundle {
            aseprite: asset_server.load(sprites::Player::PATH),
            animation: AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK),
            transform: Transform {
                scale: Vec3::splat(4.),
                translation: Vec3::new(0., -100., 0.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(PlayerTag);
}

fn change_animation(
    keys: Res<Input<KeyCode>>,
    mut aseprites: ParamSet<(
        Query<&mut AsepriteAnimation, With<CrowTag>>,
        Query<&mut AsepriteAnimation, With<PlayerTag>>,
    )>,
) {
    if keys.just_pressed(KeyCode::Key1) {
        for mut crow_anim in aseprites.p0().iter_mut() {
            *crow_anim = AsepriteAnimation::from(sprites::Crow::tags::FLAP_WINGS);
        }
        for mut player_anim in aseprites.p1().iter_mut() {
            *player_anim = AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK);
        }
    }
    if keys.just_pressed(KeyCode::Key2) {
        for mut crow_anim in aseprites.p0().iter_mut() {
            *crow_anim = AsepriteAnimation::from(sprites::Crow::tags::GROOVE);
        }
        for mut player_anim in aseprites.p1().iter_mut() {
            *player_anim = AsepriteAnimation::from(sprites::Player::tags::RIGHT_WALK);
        }
    }
    if keys.just_pressed(KeyCode::Space) {
        for mut crow_anim in aseprites.p0().iter_mut() {
            crow_anim.toggle();
        }
        for mut player_anim in aseprites.p1().iter_mut() {
            player_anim.toggle();
        }
    }
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    let font = asset_server.load("Share-Regular.ttf");

    let text_style = TextStyle {
        font: font.clone(),
        font_size: 30.,
        ..Default::default()
    };

    let credits_text_style = TextStyle {
        font,
        font_size: 20.,
        ..Default::default()
    };

    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
            sections: vec![TextSection {
                value: String::from("Press '1' and '2' to switch animations."),
                style: TextStyle {
                    color: Color::WHITE,
                    ..text_style.clone()
                },
            }],
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
            sections: vec![TextSection {
                value: String::from("Press 'space' to pause."),
                style: TextStyle {
                    color: Color::WHITE,
                    ..text_style.clone()
                },
            }],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., 250., 0.)),
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
                    value: String::from("The crow was made by "),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..credits_text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("meitdev"),
                    style: TextStyle {
                        color: Color::LIME_GREEN,
                        ..credits_text_style.clone()
                    },
                },
                TextSection {
                    value: String::from(" on itch.io"),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..credits_text_style.clone()
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., -250., 0.)),
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
                        ..credits_text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("shubibubi"),
                    style: TextStyle {
                        color: Color::BLUE,
                        ..credits_text_style.clone()
                    },
                },
                TextSection {
                    value: String::from(" on itch.io"),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..credits_text_style
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., -280., 0.)),
        ..Default::default()
    });
}
