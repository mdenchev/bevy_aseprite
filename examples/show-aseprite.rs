use bevy::prelude::*;
use bevy_spicy_aseprite::{
    AsepriteAnimation, AsepriteAnimationState, AsepriteBundle, AsepritePlugin,
};

#[derive(Component, Clone, Copy, Debug)]
struct CrowTag;

mod sprites {
    use bevy::prelude::Component;
    use bevy_spicy_aseprite::aseprite;

    aseprite!(pub Crow, "assets/crow.aseprite");
    //https://shubibubi.itch.io/cozy-people
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
    commands.spawn_bundle(AsepriteBundle {
        aseprite: sprites::Crow::sprite(),
        animation: AsepriteAnimation::from(sprites::Crow::tags::FLAP_WINGS),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., 150., 0.),
            ..Default::default()
        },
        ..Default::default()
    }).insert(CrowTag);
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
    commands.spawn_bundle(AsepriteBundle {
        aseprite: sprites::Player::sprite(),
        animation: AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., -200., 0.),
            ..Default::default()
        },
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
                        ..text_style.clone()
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., -100., 0.)),
        ..Default::default()
    });
}
// Made by https://meitdev.itch.io/crow

fn toggle_sprite(keys: Res<Input<KeyCode>>, mut aseprites: Query<&mut AsepriteAnimationState>) {
    if keys.just_pressed(KeyCode::Space) {
        for mut state in aseprites.iter_mut() {
            state.toggle();
        }
    }
}

fn change_animation(keys: Res<Input<KeyCode>>, mut aseprites: Query<&mut AsepriteAnimation, With<CrowTag>>) {
    if keys.just_pressed(KeyCode::Key1) {
        for mut crow_anim in aseprites.iter_mut() {
            *crow_anim = AsepriteAnimation::from(sprites::Crow::tags::GROOVE);
        }
    }
    if keys.just_pressed(KeyCode::Key2) {
        for mut crow_anim in aseprites.iter_mut() {
            *crow_anim = AsepriteAnimation::from(sprites::Crow::tags::FLAP_WINGS);
        }
    }
}
