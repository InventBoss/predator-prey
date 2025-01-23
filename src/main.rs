use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::Window,
};
use config::Config;
use rand::Rng;
use std::collections::HashMap;

mod position_systems;
use position_systems::{
    avoid, in_detection_range, is_colliding, move_towards, wiggle_squares, window_collision,
    PositionSize,
};

#[derive(Resource)]
struct Settings {
    window_width: f32,
    window_height: f32,
    predator_population: i32,
    prey_population: i32,
    predator_speed: f32,
    prey_speed: f32,
    predator_life: i32,
    prey_life: i32,
    detection_range: f32,
    default_dimensions: f32,
    environment_grow_rate: f32,
    environment_max: i32,
}

#[derive(Component)]
struct Mortal {
    dead: bool,
}

#[derive(Component)]
struct Prey {
    hunted: bool,
    try_mating: bool,
}

#[derive(Component)]
struct Predator {
    hunting: bool,
}

#[derive(Component)]
struct Life {
    value: i32,
}

#[derive(Component)]
struct Environment {
    energy_pool: i32,
}

fn update_predators(
    mut predators: Query<(&mut PositionSize, &mut Predator), (With<Predator>, Without<Prey>)>,
    preys: Query<&PositionSize, (With<Prey>, Without<Predator>)>,
    settings: Res<Settings>,
) {
    for (mut predator_position_size, mut predator) in predators.iter_mut() {
        // Store the closest position of a prey
        let mut closest_prey_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest prey as we'll narrow down from there
        let mut closest_prey_distance: f32 = f32::MAX;

        for prey_position_size in preys.iter() {
            let (detected, distance) = in_detection_range(
                &predator_position_size,
                &prey_position_size,
                settings.detection_range,
            );

            if detected && distance < closest_prey_distance {
                closest_prey_position = Some(prey_position_size);
                closest_prey_distance = distance;

                predator.hunting = true;
            } else {
                predator.hunting = false;
            }
        }

        // This code checks to see if there is a closest prey position
        // and assigns closest prey the value to pass to the move_towards function
        if let Some(closest_prey) = closest_prey_position {
            move_towards(
                &mut predator_position_size,
                closest_prey,
                settings.predator_speed,
            );
        }
    }
}

fn update_preys(
    mut preys: Query<(&mut PositionSize, &mut Life, &mut Prey), (With<Prey>, Without<Predator>)>,
    predators: Query<&PositionSize, (With<Predator>, Without<Prey>)>,
    mut environment_query: Query<&mut Environment>,
    settings: Res<Settings>,
) {
    // if let Some((prey_position_size, life, prey)) = preys.iter().next() {
    //     println!("Position: {}, {}, Life: {}, Hunted: {}, Mating: {}", prey_position_size.x, prey_position_size.y, life.value, prey.hunted, prey.try_mating);
    // }

    for (mut prey_position_size, mut life, mut prey) in preys.iter_mut() {
        // Store the closest position of a predator
        let mut closest_predator_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest predator as we'll narrow down from there
        let mut closest_predator_distance: f32 = f32::MAX;

        for predator_position_size in predators.iter() {
            let (detected, distance) = in_detection_range(
                &prey_position_size,
                &predator_position_size,
                settings.detection_range,
            );

            if detected && distance < closest_predator_distance {
                closest_predator_position = Some(predator_position_size);
                closest_predator_distance = distance;
            }
        }

        // This code checks to see if there is a closest predator position
        // and assigns closest predator the value to pass to the avoid function
        if let Some(closest_predator) = closest_predator_position {
            avoid(
                &mut prey_position_size,
                closest_predator,
                settings.prey_speed,
            );

            prey.hunted = true;
        } else {
            prey.hunted = false;
        }

        // Prey "eats" the environment to regain life
        for mut environment in environment_query.iter_mut() {
            if environment.energy_pool > 0 && !prey.hunted {
                environment.energy_pool -= 1;
                life.value += 1;
            }
        }
    }
}

fn update_environment(mut query: Query<&mut Environment>, settings: Res<Settings>) {
    for mut environment in query.iter_mut() {
        environment.energy_pool =
            ((environment.energy_pool as f32 * settings.environment_grow_rate).round() as i32)
                .min(settings.environment_max);
    }
}

fn drain_life(
    // This query makes it so that we fetch either a predator or a prey if the option is there
    mut query: Query<
        (&mut Mortal, &mut Life, Option<&Predator>, Option<&Prey>),
        Or<(With<Predator>, With<Prey>)>,
    >,
) {
    for (mut mortal, mut life, predator, prey) in query.iter_mut() {
        if predator.is_some() && predator.unwrap().hunting {
            life.value -= 1;
        }
        if prey.is_some() && prey.unwrap().hunted {
            life.value -= 1;
        }

        if life.value <= 0 {
            mortal.dead = true;
        }
    }
}

fn remove_dead(mut commands: Commands, query: Query<(Entity, &Mortal)>) {
    for (entity, mortal) in query.iter() {
        if mortal.dead {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn handle_collisions(
    mut prey_query: Query<(&PositionSize, &mut Mortal), With<Prey>>,
    mut predator_query: Query<(&PositionSize, &mut Life), With<Predator>>,
) {
    for (prey_posision_size, mut mortal) in prey_query.iter_mut() {
        for (predator_position_size, mut life) in predator_query.iter_mut() {
            if is_colliding(prey_posision_size, predator_position_size) {
                mortal.dead = true;
                life.value += 500;
            }
        }
    }
}

fn update_transform(mut query: Query<(&PositionSize, &mut Transform, &mut Sprite)>) {
    for (position_size, mut transform, mut sprite) in query.iter_mut() {
        // Make sure the transform components line up with their entities position
        transform.translation.x = position_size.x;
        transform.translation.y = position_size.y;

        // Shouldn't be used regularly, but if the size of PositionSize changes, it will be updated in the sprite
        sprite.custom_size = Some(Vec2::new(position_size.width, position_size.height));
    }
}

fn update_ui_text(
    mut text_query: Query<&mut Text>,
    environment_query: Query<&Environment>,
    diagnostics: Res<DiagnosticsStore>,
) {
    for mut text in text_query.iter_mut() {
        let environment = environment_query.single();
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps_diagnostic| fps_diagnostic.average())
            .unwrap_or(0.0);

        *text = Text::from(format!(
            "FPS {:.2}\nEnvironment Energy Pool {}",
            fps, environment.energy_pool
        ));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, settings: Res<Settings>) {
    commands.spawn(Camera2d::default());

    commands.spawn(Environment {
        energy_pool: settings.environment_max / 2,
    });

    // Import font and use it to create ui text elements.
    let text_font: Handle<Font> = asset_server.load("fonts/SpaceMono-Regular.ttf");

    commands.spawn((
        Text::new(""),
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font: text_font.clone(),
            font_size: 15.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));

    let window_width: f32 = settings.window_width;
    let window_height: f32 = settings.window_height;

    let default_dimensions: f32 = settings.default_dimensions;

    // Spawn all the initial predators into the simulation
    for _i in 1..=settings.predator_population {
        let random_x: f32 =
            rand::thread_rng().gen_range((-(window_width / 2.0).abs())..(window_width / 2.0).abs());
        let random_y: f32 = rand::thread_rng()
            .gen_range((-(window_height / 2.0).abs())..(window_height / 2.0).abs());

        commands.spawn((
            Predator { hunting: false },
            Mortal { dead: false },
            Life {
                value: settings.predator_life,
            },
            PositionSize {
                x: random_x,
                y: random_y,
                width: default_dimensions,
                height: default_dimensions,
            },
            Sprite {
                color: Color::srgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(default_dimensions, default_dimensions)),
                ..default()
            },
            Transform::from_xyz(random_x, random_y, 0.0),
        ));
    }

    // Spawn all the initial prey into the simulation
    for _i in 1..=settings.prey_population {
        let random_x: f32 =
            rand::thread_rng().gen_range((-(window_width / 2.0).abs())..(window_width / 2.0).abs());
        let random_y: f32 = rand::thread_rng()
            .gen_range((-(window_height / 2.0).abs())..(window_height / 2.0).abs());

        commands.spawn((
            Prey {
                hunted: false,
                try_mating: false,
            },
            Mortal { dead: false },
            Life {
                value: settings.prey_life,
            },
            PositionSize {
                x: random_x,
                y: random_y,
                width: default_dimensions,
                height: default_dimensions,
            },
            Sprite {
                color: Color::srgb(0.0, 1.0, 0.0),
                custom_size: Some(Vec2::new(default_dimensions, default_dimensions)),
                ..default()
            },
            Transform::from_xyz(random_x, random_y, 0.0),
        ));
    }
}

fn read_settings(mut commands: Commands) {
    let settings = Config::builder()
        .add_source(config::File::with_name("Settings.toml"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap()
        .try_deserialize::<HashMap<String, String>>()
        .unwrap();

    // DO NOT MESS UP THE TYPE IN THE CONFIG
    commands.insert_resource(Settings {
        window_width: settings["window_width"].parse::<f32>().unwrap(),
        window_height: settings["window_height"].parse::<f32>().unwrap(),
        predator_population: settings["predator_population"].parse::<i32>().unwrap(),
        prey_population: settings["prey_population"].parse::<i32>().unwrap(),
        predator_speed: settings["predator_speed"].parse::<f32>().unwrap(),
        prey_speed: settings["prey_speed"].parse::<f32>().unwrap(),
        predator_life: settings["predator_life"].parse::<i32>().unwrap(),
        prey_life: settings["prey_life"].parse::<i32>().unwrap(),
        detection_range: settings["detection_range"].parse::<f32>().unwrap(),
        default_dimensions: settings["default_dimensions"].parse::<f32>().unwrap(),
        environment_grow_rate: settings["environment_grow_rate"].parse::<f32>().unwrap(),
        environment_max: settings["environment_max"].parse::<i32>().unwrap(),
    });

    println!("{:#?}", settings["window_width"]);
}

fn main() {
    let mut app = App::new();

    // This is done to get window dimensions on startup only
    let window_settings = Config::builder()
        .add_source(config::File::with_name("Settings.toml"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap()
        .try_deserialize::<HashMap<String, String>>()
        .unwrap();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Predator Prey Simulation".to_string(),
                resolution: (
                    window_settings["window_width"].parse::<f32>().unwrap(),
                    window_settings["window_height"].parse::<f32>().unwrap(),
                )
                    .into(),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
    ));

    app.add_systems(Startup, (read_settings, setup.after(read_settings)));
    app.add_systems(
        Update,
        (
            update_environment,
            wiggle_squares,
            update_transform,
            update_preys,
            update_predators,
            window_collision,
            handle_collisions,
            remove_dead,
            drain_life,
            update_ui_text,
        ),
    );

    app.run();
}
