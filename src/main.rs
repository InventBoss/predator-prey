use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::Window,
};
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use config::Config;
use egui::Color32;
use rand::Rng;
use std::collections::HashMap;

use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Legend, Line, Plot, PlotPoints};

mod position_systems;
use position_systems::{
    avoid, in_detection_range, is_colliding, move_towards, update_transform, wiggle_squares,
    window_collision, PositionSize,
};

#[derive(Reflect, Resource)]
#[reflect(Resource)]
struct PopulationHistory {
    prey_population: Vec<[f64; 2]>,
    predator_population: Vec<[f64; 2]>,
}

#[derive(Reflect, Resource)]
#[reflect(Resource)]
pub struct Settings {
    window_width: f32,
    window_height: f32,
    predator_population: i32,
    prey_population: i32,
    predator_speed: f32,
    prey_speed: f32,
    predator_life: i32,
    prey_life: i32,
    prey_energy_loss: i32,
    predator_energy_loss: i32,
    prey_idle_energy_gain: i32,
    predator_hunt_energy_gain: i32,
    prey_reproduction_energy: i32,
    predator_reproduction_energy: i32,
    prey_detection_range: f32,
    predator_detection_range: f32,
    default_dimensions: f32,
    environment_grow_rate: f32,
    environment_max: i32,
    wiggle_when_hunted: bool,
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct Mortal {
    dead: bool,
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct Prey {
    status: u16, // 0 is idle, 1 is mating, 2 is avoiding
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct Predator {
    status: u16, // 0 is idle, 1 is mating, 2 is hunting
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct MatingTarget {
    entity: Option<PositionSize>,
    index: Option<u32>,
    // Index is stored for when we mate with the target. Eventually, the partner with the higher index
    // will have the child. This is to prevent making twins when both the partners run reproduction code.
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct Life {
    value: i32,
}

#[derive(Reflect, Component)]
#[reflect(Component)]
struct Environment {
    energy_pool: i32,
}

fn can_mate(current_energy: i32, required_energy: i32, status: u16) -> bool {
    // Check to make sure the predator or prey isn't hunting or being hunted
    if status == 2 {
        return false;
    }

    return current_energy >= required_energy;
}

fn update_predators(
    mut predators: Query<
        (&mut PositionSize, &MatingTarget, &mut Predator),
        (With<Predator>, Without<Prey>),
    >,
    preys: Query<&PositionSize, (With<Prey>, Without<Predator>)>,
    settings: Res<Settings>,
) {
    for (mut predator_position_size, mating_target, mut predator) in predators.iter_mut() {
        // Store the closest position of a prey
        let mut closest_prey_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest prey as we'll narrow down from there
        let mut closest_prey_distance: f32 = f32::MAX;

        for prey_position_size in preys.iter() {
            let (detected, distance) = in_detection_range(
                &predator_position_size,
                prey_position_size,
                settings.predator_detection_range,
            );

            if detected && distance < closest_prey_distance {
                closest_prey_position = Some(prey_position_size);
                closest_prey_distance = distance;
            }
        }

        if mating_target.entity.is_some() {
            predator.status = 1; // Mating
        } else if closest_prey_position.is_some() {
            predator.status = 2; // Hunting
        } else {
            predator.status = 0; // Idle
        }

        // Check to see if we can mate, then move towards the target
        if predator.status == 1 {
            if let Some(target) = &mating_target.entity {
                move_towards(&mut predator_position_size, target, settings.predator_speed);
            }
        } else if predator.status == 2 {
            if let Some(closest_prey) = closest_prey_position {
                move_towards(
                    &mut predator_position_size,
                    closest_prey,
                    settings.predator_speed,
                );
            }
        }
    }
}

fn update_preys(
    mut prey_query: Query<
        (&mut PositionSize, &mut Life, &MatingTarget, &mut Prey),
        (With<Prey>, Without<Predator>),
    >,
    predator_query: Query<&PositionSize, (With<Predator>, Without<Prey>)>,
    mut environment_query: Query<&mut Environment>,
    settings: Res<Settings>,
) {
    for (mut prey_position_size, mut life, mating_target, mut prey) in prey_query.iter_mut() {
        // Store the closest position of a predator
        let mut closest_predator_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest predator as we'll narrow down from there
        let mut closest_predator_distance: f32 = f32::MAX;

        for predator_position_size in predator_query.iter() {
            let (detected, distance) = in_detection_range(
                &prey_position_size,
                predator_position_size,
                settings.prey_detection_range,
            );

            if detected && distance < closest_predator_distance {
                closest_predator_position = Some(predator_position_size);
                closest_predator_distance = distance;
            }
        }

        if closest_predator_position.is_some() {
            prey.status = 2 // Running
        } else if can_mate(life.value, settings.prey_reproduction_energy, prey.status) {
            prey.status = 1 // Mating
        } else {
            prey.status = 0 // Idle
        }

        // This code checks to see if there is a closest predator position
        // and assigns closest predator the value to pass to the avoid function
        if prey.status == 2 {
            if let Some(closest_predator) = closest_predator_position {
                avoid(
                    &mut prey_position_size,
                    closest_predator,
                    settings.prey_speed,
                );
            }
        // Check to see we can mate and there is an available mate
        } else if prey.status == 1 && mating_target.entity.is_some() {
            if let Some(target) = &mating_target.entity {
                move_towards(&mut prey_position_size, target, settings.prey_speed);
            }
        }

        // Prey "eats" the environment to regain life
        for mut environment in environment_query.iter_mut() {
            // Checks to make sure energy can be taken from the environment and that we aren't being chased
            if environment.energy_pool > 0 && prey.status != 3 {
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

fn try_mate_prey(
    mut seekers: Query<(Entity, &Life, &PositionSize, &mut MatingTarget), With<Prey>>,
    targets: Query<(Entity, &Life, &PositionSize), With<Prey>>,
    settings: Res<Settings>,
) {
    for (seeker_entity, seeker_life, seeker_pos, mut seeker_final_target) in seekers.iter_mut() {
        if seeker_life.value < settings.prey_reproduction_energy {
            continue;
        }

        let mut closest_target_pos = None;
        let mut closest_target_index = None;
        let mut min_distance = f32::MAX;

        for (target_entity, target_life, target_pos) in targets.iter() {
            if target_entity == seeker_entity
                || target_life.value < settings.prey_reproduction_energy
            {
                continue;
            }

            let (detected, distance) =
                in_detection_range(&seeker_pos, target_pos, settings.prey_detection_range);
            if detected && distance < min_distance {
                min_distance = distance;
                closest_target_pos = Some(target_pos);
                closest_target_index = Some(target_entity.index());
            }
        }

        if let Some(target_pos) = closest_target_pos {
            seeker_final_target.entity = Some(target_pos.clone());
            seeker_final_target.index = closest_target_index;
        }
    }
}

fn try_mate_predator(
    mut seekers: Query<(Entity, &Life, &PositionSize, &mut MatingTarget), With<Predator>>,
    targets: Query<(Entity, &Life, &PositionSize), With<Predator>>,
    settings: Res<Settings>,
) {
    for (seeker_entity, seeker_life, seeker_pos, mut seeker_final_target) in seekers.iter_mut() {
        if seeker_life.value < settings.predator_reproduction_energy {
            continue;
        }

        let mut closest_target = None;
        let mut min_distance = f32::MAX;

        for (target_entity, target_life, target_pos) in targets.iter() {
            if target_entity == seeker_entity
                || target_life.value < settings.predator_reproduction_energy
            {
                continue;
            }

            let (detected, distance) =
                in_detection_range(&seeker_pos, target_pos, settings.predator_detection_range);
            if detected && distance < min_distance {
                min_distance = distance;
                closest_target = Some(target_pos);
            }
        }

        if let Some(target_pos) = closest_target {
            seeker_final_target.entity = Some(target_pos.clone());
        }
    }
}

fn drain_life(
    // This query makes it so that we fetch either a predator or a prey if the option is there
    mut query: Query<
        (&mut Mortal, &mut Life, Option<&Predator>, Option<&Prey>),
        Or<(With<Predator>, With<Prey>)>,
    >,
    settings: Res<Settings>,
) {
    for (mut mortal, mut life, predator, prey) in query.iter_mut() {
        if predator.is_some() {
            // Predators lose energy constantly
            life.value -= settings.predator_energy_loss;
        }
        if prey.is_some() && prey.unwrap().status == 3 {
            // Preys only lose it if they're being hunted as it's being regenerated
            // by eating the environment anyway
            life.value -= settings.prey_energy_loss;
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

fn handle_mating(
    mut query: Query<
        (
            Entity,
            &PositionSize,
            &mut MatingTarget,
            &mut Life,
            Option<&Predator>,
            Option<&Prey>,
        ),
        Or<(With<Predator>, With<Prey>)>,
    >,
    settings: Res<Settings>,
    mut commands: Commands,
) {
    for (entity, position_size, mut mating_target, mut life, predator, prey) in query.iter_mut() {
        // Check what kind of entity we're dealing with
        let mut entity_type: u16 = 0; // 0 is prey, 1 is predator
        let required_energy: i32; // We default to prey and overwrite if neccesary
        let entity_status: u16;

        if predator.is_some() {
            entity_type = 1;
            required_energy = settings.predator_reproduction_energy;
            entity_status = predator.unwrap().status;
        } else {
            required_energy = settings.prey_reproduction_energy;
            entity_status = prey.unwrap().status;
        }

        // Prey can't breed if they're being hunted and
        // we need to check to make sure the entity (both prey or predator) has enough energy to mate
        if entity_status == 2 && entity_type == 0 || life.value < required_energy {
            continue;
        }

        // We check to see if there is even a mate
        if let Some(target) = &mating_target.entity {
            // Ensure we are actually colliding with our target
            if !is_colliding(&position_size, target) {
                continue;
            }

            // We give breeding priority to the mate with a higher index to prevent twins
            // by only having one partner run the reproduction code. If this entity skips the code,
            // their partner either had or is going to have the child entity
            if let Some(index) = mating_target.index {
                if entity.index() < index {
                    continue;
                }
            }

            match entity_type {
                0 => {
                    let position_x = (position_size.x + target.x) / 2.0;
                    let position_y = (position_size.y + target.y) / 2.0;

                    commands.spawn((
                        Prey { status: 0 },
                        Mortal { dead: false },
                        MatingTarget {
                            entity: None,
                            index: None,
                        },
                        Life {
                            value: settings.prey_life,
                        },
                        PositionSize {
                            x: position_x,
                            y: position_y,
                            width: settings.default_dimensions,
                            height: settings.default_dimensions,
                        },
                        Sprite {
                            color: Color::srgb(0.0, 1.0, 0.0),
                            custom_size: Some(Vec2::new(
                                settings.default_dimensions,
                                settings.default_dimensions,
                            )),
                            ..default()
                        },
                        Transform::from_xyz(position_x, position_y, 0.0),
                    ));
                }
                1 => {
                    let position_x = (position_size.x + target.x) / 2.0;
                    let position_y = (position_size.y + target.y) / 2.0;

                    commands.spawn((
                        Predator { status: 0 },
                        Mortal { dead: false },
                        MatingTarget {
                            entity: None,
                            index: None,
                        },
                        Life {
                            value: settings.predator_life,
                        },
                        PositionSize {
                            x: position_x,
                            y: position_y,
                            width: settings.default_dimensions,
                            height: settings.default_dimensions,
                        },
                        Sprite {
                            color: Color::srgb(1.0, 0.0, 0.0),
                            custom_size: Some(Vec2::new(
                                settings.default_dimensions,
                                settings.default_dimensions,
                            )),
                            ..default()
                        },
                        Transform::from_xyz(position_x, position_y, 0.0),
                    ));
                }
                _ => {} // Handle the impossible edge case where it isn't 0 or 1
            }

            life.value -= required_energy; // Reduce the energy of the parent
            mating_target.entity = None;
            mating_target.index = None
        }
    }
}

fn handle_hostile_collisions(
    mut prey_query: Query<(&PositionSize, &mut Mortal), With<Prey>>,
    mut predator_query: Query<(&PositionSize, &mut Life), With<Predator>>,
) {
    for (prey_posision_size, mut prey_mortal) in prey_query.iter_mut() {
        for (predator_position_size, mut predator_life) in predator_query.iter_mut() {
            if is_colliding(prey_posision_size, predator_position_size) && !prey_mortal.dead {
                prey_mortal.dead = true;
                predator_life.value += 500;
            }
        }
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

fn update_population_history(
    time: Res<Time>,
    prey_query: Query<&Prey>,
    predator_query: Query<&Predator>,
    mut history: ResMut<PopulationHistory>,
) {
    let prey_count = prey_query.iter().count() as f64;
    let predator_count = predator_query.iter().count() as f64;

    let time_elapsed = time.elapsed_secs_f64();

    history.prey_population.push([time_elapsed, prey_count]);
    history
        .predator_population
        .push([time_elapsed, predator_count]);
}

fn plot_ui(mut contexts: EguiContexts, history: Res<PopulationHistory>) {
    egui::Window::new("Populations & Environment Energy Over Time")
        .default_open(false)
        .show(contexts.ctx_mut(), |ui| {
            let prey_line = Line::new(PlotPoints::from(history.prey_population.clone()))
                .name("Prey Population")
                .color(Color32::GREEN);
            let predator_line = Line::new(PlotPoints::from(history.predator_population.clone()))
                .name("Predator Population")
                .color(Color32::RED);

            Plot::new("entity_population_plot")
                .legend(Legend::default())
                .x_axis_label("Time (s)")
                .y_axis_label("Amount")
                .label_formatter(|name, value| {
                    let display_name = &name.replace(" Population", "");
                    if !display_name.is_empty() {
                        format!(
                            "{} Amount: {}\nTime: {}:{:04.1}s",
                            display_name,
                            value.y,
                            (value.x / 60.0).floor(),
                            value.x % 60.0
                        )
                    } else {
                        "".to_owned()
                    }
                })
                .show(ui, |plot_ui| {
                    plot_ui.line(prey_line);
                    plot_ui.line(predator_line);
                });
        });
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
        TextLayout::new_with_justify(JustifyText::Right),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
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
            Predator { status: 0 },
            Mortal { dead: false },
            MatingTarget {
                entity: None,
                index: None,
            },
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
            Prey { status: 0 },
            Mortal { dead: false },
            MatingTarget {
                entity: None,
                index: None,
            },
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
        .add_source(config::File::with_name("Settings.toml")) // Read config values from file
        .add_source(config::Environment::with_prefix("APP")) // Also read config values from environment variables
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
        prey_energy_loss: settings["prey_energy_loss"].parse::<i32>().unwrap(),
        predator_energy_loss: settings["predator_energy_loss"].parse::<i32>().unwrap(),
        prey_idle_energy_gain: settings["prey_idle_energy_gain"].parse::<i32>().unwrap(),
        predator_hunt_energy_gain: settings["predator_hunt_energy_gain"]
            .parse::<i32>()
            .unwrap(),
        prey_reproduction_energy: settings["prey_reproduction_energy"].parse::<i32>().unwrap(),
        predator_reproduction_energy: settings["predator_reproduction_energy"]
            .parse::<i32>()
            .unwrap(),
        prey_detection_range: settings["prey_detection_range"].parse::<f32>().unwrap(),
        predator_detection_range: settings["predator_detection_range"].parse::<f32>().unwrap(),
        default_dimensions: settings["default_dimensions"].parse::<f32>().unwrap(),
        environment_grow_rate: settings["environment_grow_rate"].parse::<f32>().unwrap(),
        environment_max: settings["environment_max"].parse::<i32>().unwrap(),
        wiggle_when_hunted: settings["wiggle_when_hunted"].parse::<bool>().unwrap(),
    });
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
        EguiPlugin,
    ));

    // Make sure settings resource is created BEFORE
    // setting up the simulation with all the necessary values
    app.add_systems(Startup, (read_settings, setup.after(read_settings)));
    app.insert_resource(PopulationHistory {
        prey_population: Vec::new(),
        predator_population: Vec::new(),
    });

    // These components and resources are being "registered" to appear in the inspector gui
    app.register_type::<PopulationHistory>();
    app.register_type::<Settings>();
    app.register_type::<PositionSize>();
    app.register_type::<Mortal>();
    app.register_type::<Prey>();
    app.register_type::<Predator>();
    app.register_type::<MatingTarget>();
    app.register_type::<Life>();
    app.register_type::<Environment>();

    // These are all the functions to add the ui elements to the simulation
    app.add_plugins((
        ResourceInspectorPlugin::<Settings>::default(),
        WorldInspectorPlugin::new(),
    ));
    // app.add_systems(Update, plot_ui);

    app.add_systems(
        Update,
        (
            update_environment,
            wiggle_squares,
            update_transform,
            handle_mating,
            update_preys,
            update_predators,
            try_mate_prey,
            try_mate_predator,
            window_collision,
            handle_hostile_collisions,
            remove_dead,
            drain_life,
            update_ui_text,
            update_population_history,
            plot_ui,
        ),
    );

    app.run();
}
