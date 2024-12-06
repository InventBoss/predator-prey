use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use rand::Rng;

// The starting window dimensions
pub const WINDOW_WIDTH: f32 = 1000.0;
pub const WINDOW_HEIGHT: f32 = 600.0;

// Initial values to impact the simulation
pub const PREDATOR_POPULATION: i32 = 10;
pub const PREY_POPULATION: i32 = 100000;

// Speed that the two populations can go when avoiding and hunting
pub const PREDATOR_SPEED: f32 = 0.5;
pub const PREY_SPEED: f32 = 0.5;

// Each population's life force
pub const PREDATOR_LIFE: i32 = 30000;
pub const PREY_LIFE: i32 = 50000;

// Range that the populations can "see" each other
pub const DETECTION_RANGE: f32 = 60.0;

// Size of the entities (e.g. 4 = 4 pixels long by 4 pixels wide)
pub const DEFAULT_DIMENSION: f32 = 0.5;

// Different environment variables
pub const ENVIRONMENT_GROW_RATE: f32 = 1.5;
pub const ENVIRONMENT_MAX: i32 = 1000000;

#[derive(Component)]
struct PositionSize {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Component)]
struct Mortal {
    dead: bool,
}

// #[derive(Component)]
// struct Direction {
//     x: f32,
//     y: f32,
// }

#[derive(Component)]
struct Prey;

#[derive(Component)]
struct Predator;

#[derive(Component)]
struct Life {
    value: i32,
}

#[derive(Component)]
struct Environment {
    energy_pool: i32,
    max_energy: i32,
}

fn is_colliding(entity1: &PositionSize, entity2: &PositionSize) -> bool {
    // Used this resource for intersections https://silentmatt.com/rectangle-intersection/

    if entity1.x < entity2.x + entity2.width
        && entity1.x + entity1.width > entity2.x
        && entity1.y < entity2.y + entity2.height
        && entity1.y + entity1.height > entity2.y
    {
        return true;
    }
    return false;
}

fn in_detection_range(entity1: &PositionSize, entity2: &PositionSize) -> (bool, f32) {
    // Formula from https://www.calculator.net/distance-calculator.html
    let distance = ((entity1.x - entity2.x).powf(2.0) + (entity1.y - entity2.y).powf(2.0)).sqrt();

    return (distance <= DETECTION_RANGE, distance);
}

fn avoid(entity: &mut PositionSize, target: &PositionSize, speed: f32) {
    // This sweet answer obtained from
    // https://math.stackexchange.com/questions/707673/find-angle-in-degrees-from-one-point-to-another-in-2d-space
    let angle = (target.y - entity.y).atan2(target.x - entity.x);

    entity.x += angle.cos() * -1.0 * speed;
    entity.y += angle.sin() * -1.0 * speed;
}

fn move_towards(entity: &mut PositionSize, target: &PositionSize, speed: f32) {
    // This sweet answer obtained from
    // https://math.stackexchange.com/questions/707673/find-angle-in-degrees-from-one-point-to-another-in-2d-space
    let angle = (target.y - entity.y).atan2(target.x - entity.x);

    entity.x += angle.cos() * speed;
    entity.y += angle.sin() * speed;
}

fn update_predators(
    mut predators: Query<&mut PositionSize, (With<Predator>, Without<Prey>)>,
    preys: Query<&PositionSize, (With<Prey>, Without<Predator>)>,
) {
    for mut predator_position_size in predators.iter_mut() {
        // Store the closest position of a prey
        let mut closest_prey_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest prey as we'll narrow down from there
        let mut closest_prey_distance: f32 = f32::MAX;

        for prey_position_size in preys.iter() {
            let (detected, distance) =
                in_detection_range(&predator_position_size, &prey_position_size);

            if detected && distance < closest_prey_distance {
                closest_prey_position = Some(prey_position_size);
                closest_prey_distance = distance;
            }
        }

        // This code checks to see if there is a closest prey position
        // and assigns closest prey the value to pass to the move_towards function
        if let Some(closest_prey) = closest_prey_position {
            move_towards(&mut predator_position_size, closest_prey, PREDATOR_SPEED);
        }
    }
}

fn update_preys(
    mut preys: Query<(&mut PositionSize, &mut Life), (With<Prey>, Without<Predator>)>,
    predators: Query<&PositionSize, (With<Predator>, Without<Prey>)>,
    mut environment_query: Query<&mut Environment>,
) {
    for (mut prey_position_size, mut life) in preys.iter_mut() {
        // Store the closest position of a predator
        let mut closest_predator_position: Option<&PositionSize> = None;

        // Have a humongous initial value for
        // the closest predator as we'll narrow down from there
        let mut closest_predator_distance: f32 = f32::MAX;

        for predator_position_size in predators.iter() {
            let (detected, distance) =
                in_detection_range(&prey_position_size, &predator_position_size);

            if detected && distance < closest_predator_distance {
                closest_predator_position = Some(predator_position_size);
                closest_predator_distance = distance;
            }
        }

        // This code checks to see if there is a closest predator position
        // and assigns closest predator the value to pass to the avoid function
        if let Some(closest_predator) = closest_predator_position {
            avoid(&mut prey_position_size, closest_predator, PREY_SPEED);
        }

        // Prey "eats" the environment to regain life.
        for mut environment in environment_query.iter_mut() {
            if environment.energy_pool > 0 {
                environment.energy_pool -= 1;
                life.value += 10;
            }
        }
    }
}

fn update_environment(mut query: Query<&mut Environment>) {
    for mut environment in query.iter_mut() {
        environment.energy_pool = ((environment.energy_pool as f32 * ENVIRONMENT_GROW_RATE).round()
            as i32)
            .min(environment.max_energy);
    }
}

fn drain_life(
    mut query: Query<
        (&mut Mortal, &mut Life, Option<&Predator>, Option<&Prey>),
        Or<(With<Predator>, With<Prey>)>,
    >,
) {
    for (mut mortal, mut life, predator, prey) in query.iter_mut() {
        if predator.is_some() {
            life.value -= 2
        }
        if prey.is_some() {
            life.value -= 1
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

fn update_transform(mut query: Query<(&PositionSize, &mut Transform)>) {
    for (position_size, mut transform) in query.iter_mut() {
        transform.translation.x = position_size.x;
        transform.translation.y = position_size.y;
    }
}

fn window_collision(mut query: Query<&mut PositionSize>) {
    for mut position_size in query.iter_mut() {
        position_size.x = position_size.x.min(WINDOW_WIDTH / 2.0);
        position_size.x = position_size.x.max(WINDOW_WIDTH / -2.0);

        position_size.y = position_size.y.min(WINDOW_HEIGHT / 2.0);
        position_size.y = position_size.y.max(WINDOW_HEIGHT / -2.0);
    }
}

fn wiggle_squares(_time: Res<Time>, mut query: Query<&mut PositionSize>) {
    for mut position_size in query.iter_mut() {
        let random_x: f32 = rand::thread_rng().gen_range(-1.0..1.0);
        let random_y: f32 = rand::thread_rng().gen_range(-1.0..1.0);

        let wiggle_amount = Vec2::from_array((random_x, random_y).into());

        position_size.x += wiggle_amount.x;
        position_size.y += wiggle_amount.y;
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d::default());

    commands.spawn(Environment {
        energy_pool: ENVIRONMENT_MAX / 2,
        max_energy: ENVIRONMENT_MAX,
    });

    // Import font and use it to create ui text elements.
    let text_font: Handle<Font> = asset_server.load("fonts/SpaceMono-Regular.ttf");

    commands.spawn((
        Text::new("From an &str into a Text with the default font!"),
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

    // Spawn all the initial predators into the simulation
    for _i in 1..=PREDATOR_POPULATION {
        let random_x: f32 =
            rand::thread_rng().gen_range((-(WINDOW_WIDTH / 2.0).abs())..(WINDOW_WIDTH / 2.0).abs());
        let random_y: f32 = rand::thread_rng()
            .gen_range((-(WINDOW_HEIGHT / 2.0).abs())..(WINDOW_HEIGHT / 2.0).abs());

        commands.spawn((
            Predator,
            Mortal { dead: false },
            // Direction { x: 0.0, y: 0.0 },
            Life {
                value: PREDATOR_LIFE,
            },
            PositionSize {
                x: random_x,
                y: random_y,
                width: DEFAULT_DIMENSION,
                height: DEFAULT_DIMENSION,
            },
            Sprite {
                color: Color::srgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(DEFAULT_DIMENSION, DEFAULT_DIMENSION)),
                ..default()
            },
            Transform::from_xyz(random_x, random_y, 0.0),
        ));
    }

    // Spawn all the initial prey into the simulation
    for _i in 1..=PREY_POPULATION {
        let random_x: f32 = rand::thread_rng().gen_range(-500.0..500.0);
        let random_y: f32 = rand::thread_rng().gen_range(-300.0..300.0);

        commands.spawn((
            Prey,
            Mortal { dead: false },
            // Direction { x: 0.0, y: 0.0 },
            Life { value: PREY_LIFE },
            PositionSize {
                x: random_x,
                y: random_y,
                width: DEFAULT_DIMENSION,
                height: DEFAULT_DIMENSION,
            },
            Sprite {
                color: Color::srgb(0.0, 1.0, 0.0),
                custom_size: Some(Vec2::new(DEFAULT_DIMENSION, DEFAULT_DIMENSION)),
                ..default()
            },
            Transform::from_xyz(random_x, random_y, 0.0),
        ));
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Predator Prey Simulation".to_string(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
    ));

    app.add_systems(Startup, setup);
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
