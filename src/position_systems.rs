/*
    This file includes all systems & child functions used for
    anything involving moving the important entities (predators & prey) around on the screen.

    Examples:
    - Functions that draw the entity squares at said position
    - Functions that get predators or prey to move based on random behavior,
      hunting/defending, etc.
    - Functions that restrict entity movement to the bounds of the window
*/

use bevy::prelude::*;
use rand::Rng;

#[derive(Reflect, Component)]
#[reflect(Component)]
pub struct PositionSize {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn is_colliding(entity1: &PositionSize, entity2: &PositionSize) -> bool {
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

pub fn avoid(entity: &mut PositionSize, target: &PositionSize, speed: f32) {
    // This sweet answer obtained from
    // https://math.stackexchange.com/questions/707673/find-angle-in-degrees-from-one-point-to-another-in-2d-space
    let angle = (target.y - entity.y).atan2(target.x - entity.x);

    entity.x += angle.cos() * -1.0 * speed;
    entity.y += angle.sin() * -1.0 * speed;
}

pub fn move_towards(entity: &mut PositionSize, target: &PositionSize, speed: f32) {
    // This sweet answer obtained from
    // https://math.stackexchange.com/questions/707673/find-angle-in-degrees-from-one-point-to-another-in-2d-space
    let angle = (target.y - entity.y).atan2(target.x - entity.x);

    entity.x += angle.cos() * speed;
    entity.y += angle.sin() * speed;
}

pub fn in_detection_range(
    entity1: &PositionSize,
    entity2: &PositionSize,
    detection_range: f32,
) -> (bool, f32) {
    // Formula from https://www.calculator.net/distance-calculator.html
    let distance = ((entity1.x - entity2.x).powf(2.0) + (entity1.y - entity2.y).powf(2.0)).sqrt();

    return (distance <= detection_range, distance);
}

pub fn wiggle_squares(_time: Res<Time>, mut query: Query<&mut PositionSize>) {
    for mut position_size in query.iter_mut() {
        let random_x: f32 = rand::thread_rng().gen_range(-1.0..1.0);
        let random_y: f32 = rand::thread_rng().gen_range(-1.0..1.0);

        let wiggle_amount = Vec2::from_array((random_x, random_y).into());

        position_size.x += wiggle_amount.x;
        position_size.y += wiggle_amount.y;
    }
}

pub fn window_collision(mut query: Query<&mut PositionSize>, windows: Query<&Window>) {
    let window = windows.get_single().unwrap();
    let window_width: f32 = window.width();
    let window_height: f32 = window.height();

    for mut position_size in query.iter_mut() {
        position_size.x = position_size.x.min(window_width / 2.0);
        position_size.x = position_size.x.max(window_width / -2.0);

        position_size.y = position_size.y.min(window_height / 2.0);
        position_size.y = position_size.y.max(window_height / -2.0);
    }
}

pub fn update_transform(mut query: Query<(&PositionSize, &mut Transform, &mut Sprite)>) {
    for (position_size, mut transform, mut sprite) in query.iter_mut() {
        // Make sure the transform components line up with their entities position
        transform.translation.x = position_size.x;
        transform.translation.y = position_size.y;

        // Shouldn't be used regularly, but if the size of PositionSize changes, it will be updated in the sprite
        sprite.custom_size = Some(Vec2::new(
            position_size.width.abs(),
            position_size.height.abs(),
        ));
    }
}