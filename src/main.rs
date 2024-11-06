use bevy::prelude::*;
use rand::Rng;

// Check this resource out for intersections https://silentmatt.com/rectangle-intersection/

// The starting window dimensions
pub const WINDOW_WIDTH: f32 = 1000.0;
pub const WINDOW_HEIGHT: f32 = 600.0;

// Initial values to impact the simulation
pub const PREDATOR_POPULATION: i32 = 30;
pub const PREY_POPULATION: i32 = 1500;

// Speed that the two populations can go when avoiding and hunting
pub const PREDATOR_SPEED: f32 = 2.0;
pub const PREY_SPEED: f32 = 1.0;

// Range that the populations can "see" each other
pub const DETECTION_RANGE: f32 = 60.0;

pub const DEFAULT_DIMENSION: f32 = 4.0;

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

fn is_colliding(entity1: &PositionSize, entity2: &PositionSize) -> bool {
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
    // # Formula from https://www.calculator.net/distance-calculator.html
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
        //  and assigns closest prey the value to pass to the move_towards function
        if let Some(closest_prey) = closest_prey_position {
            move_towards(&mut predator_position_size, closest_prey, PREDATOR_SPEED);
        }
    }
}

fn update_preys(
    mut preys: Query<&mut PositionSize, (With<Prey>, Without<Predator>)>,
    predators: Query<&PositionSize, (With<Predator>, Without<Prey>)>,
) {
    for mut prey_position_size in preys.iter_mut() {
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
        //  and assigns closest predator the value to pass to the avoid function
        if let Some(closest_predator) = closest_predator_position {
            avoid(&mut prey_position_size, closest_predator, PREY_SPEED);
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
    predator_query: Query<&PositionSize, With<Predator>>,
) {
    for (prey_posision_size, mut mortal) in prey_query.iter_mut() {
        for predator_position_size in predator_query.iter() {
            if is_colliding(prey_posision_size, predator_position_size) {
                mortal.dead = true;
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

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Spawn all the initial predators into the simulation
    for _i in 1..=PREDATOR_POPULATION {
        let random_x: f32 = rand::thread_rng().gen_range(-500.0..500.0);
        let random_y: f32 = rand::thread_rng().gen_range(-300.0..300.0);

        commands.spawn((
            Predator,
            // Direction { x: 0.0, y: 0.0 },
            PositionSize {
                x: random_x,
                y: random_y,
                width: DEFAULT_DIMENSION,
                height: DEFAULT_DIMENSION,
            },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.0, 0.0),
                    custom_size: Some(Vec2::new(DEFAULT_DIMENSION, DEFAULT_DIMENSION)),
                    ..default()
                },
                transform: Transform::from_xyz(random_x, random_y, 0.0),
                ..default()
            },
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
            PositionSize {
                x: random_x,
                y: random_y,
                width: DEFAULT_DIMENSION,
                height: DEFAULT_DIMENSION,
            },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.0, 1.0, 0.0),
                    custom_size: Some(Vec2::new(DEFAULT_DIMENSION, DEFAULT_DIMENSION)),
                    ..default()
                },
                transform: Transform::from_xyz(random_x, random_y, 0.0),
                ..default()
            },
        ));
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Predator Prey Simulation".to_string(),
            resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
            ..default()
        }),
        ..default()
    }));

    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            wiggle_squares,
            update_transform,
            window_collision,
            handle_collisions,
            update_predators,
            remove_dead,
            update_preys,
        ),
    );

    app.run();
}
