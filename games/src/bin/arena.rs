//! Arena: a 3D survival shooter, rendered with Ponko meshes, materials and lights.
use ponko::*;
use ponko_arcade::{
    arena::{Arena, Input, PILLARS},
    runtime::*,
};
fn pooled(ctx: &mut Ctx, mesh: Mesh, count: usize) -> Vec<Entity> {
    (0..count)
        .map(|_| {
            ctx.world.spawn((
                Transform3D {
                    scale: Vec3::ZERO,
                    ..Default::default()
                },
                Model::new(mesh),
            ))
        })
        .collect()
}
fn place(ctx: &mut Ctx, id: Entity, pos: Vec3, scale: Vec3, angle: f32) {
    if let Some(t) = ctx.world.get_mut::<Transform3D>(id) {
        t.position = pos;
        t.scale = scale;
        t.rotation = Quat::from_rotation_y(angle);
    }
}
fn main() {
    Game::new("Arena / Ponko", 960, 600).run(|ctx| {
        ctx.sky.atmosphere = false;
        ctx.lighting.sun_direction = vec3(-0.6, -0.8, -0.4);
        ctx.lighting.sun_intensity = 2.2;
        ctx.lighting.ambient_intensity = 0.35;
        ctx.lighting.shadows = true;
        ctx.post.bloom = 0.12;
        ctx.camera3d.position = vec3(0., 43., 32.);
        ctx.camera3d.look_at(vec3(0., 0., 0.));
        ctx.camera3d.far = 130.;
        let floor_mat = ctx.create_material(MaterialDesc {
            roughness: 0.8,
            ..MaterialDesc::color(Color::hex(0x263a46))
        });
        let stone = ctx.create_material(MaterialDesc {
            roughness: 0.55,
            metallic: 0.25,
            ..MaterialDesc::color(Color::hex(0x54636b))
        });
        let player_mat = ctx.create_material(MaterialDesc {
            roughness: 0.3,
            metallic: 0.4,
            ..MaterialDesc::color(Color::hex(0xc2e4c2))
        });
        let enemy_mat = ctx.create_material(MaterialDesc {
            roughness: 0.4,
            metallic: 0.2,
            ..MaterialDesc::color(Color::hex(0xd16c4d))
        });
        let shot_mat = ctx.create_material(MaterialDesc::color(Color::hex(0xf9dc91)));
        let floor = ctx.create_mesh(&MeshData::plane(47.), floor_mat);
        let block = ctx.create_mesh(&MeshData::cube(1.), stone);
        let drone = ctx.create_mesh(&MeshData::sphere(0.65, 16), enemy_mat);
        let pilot = ctx.create_mesh(&MeshData::cube(1.), player_mat);
        let bullet = ctx.create_mesh(&MeshData::cube(1.), shot_mat);
        ctx.world.spawn((Transform3D::default(), Model::new(floor)));
        for p in PILLARS {
            ctx.world.spawn((
                Transform3D {
                    scale: vec3(3., 3.6, 3.),
                    ..Transform3D::at(vec3(p.x, 1.8, p.y))
                },
                Model::new(block),
            ));
        }
        for (pos, scale) in [
            (vec3(0., 0.3, -23.), vec3(47., 0.6, 0.4)),
            (vec3(0., 0.3, 23.), vec3(47., 0.6, 0.4)),
            (vec3(-23., 0.3, 0.), vec3(0.4, 0.6, 47.)),
            (vec3(23., 0.3, 0.), vec3(0.4, 0.6, 47.)),
        ] {
            ctx.world.spawn((
                Transform3D {
                    scale,
                    ..Transform3D::at(pos)
                },
                Model::new(block),
            ));
        }
        // Floor seams make direction and speed legible from the fixed camera.
        for i in -11..=11 {
            for axis in 0..2 {
                let (p, s) = if axis == 0 {
                    (vec3(i as f32 * 2., 0.012, 0.), vec3(0.018, 0.012, 46.))
                } else {
                    (vec3(0., 0.012, i as f32 * 2.), vec3(46., 0.012, 0.018))
                };
                ctx.world.spawn((
                    Transform3D {
                        scale: s,
                        ..Transform3D::at(p)
                    },
                    Model::new(block),
                ));
            }
        }
        for (x, z) in [(-18., -18.), (18., -18.), (-18., 18.), (18., 18.)] {
            ctx.world.spawn((
                Transform3D::at(vec3(x, 4., z)),
                PointLight3D {
                    color: Color::hex(0x80b9d4),
                    intensity: 100.,
                    range: 19.,
                    ..Default::default()
                },
            ));
        }
        let player = ctx.world.spawn((Transform3D::default(), Model::new(pilot)));
        let gun = ctx.world.spawn((Transform3D::default(), Model::new(pilot)));
        let player_light = ctx.world.spawn((
            Transform3D::default(),
            PointLight3D {
                color: Color::hex(0xc5f7c6),
                intensity: 30.,
                range: 7.,
                ..Default::default()
            },
        ));
        let drones = pooled(ctx, drone, 200);
        let shots = pooled(ctx, bullet, 80);
        let debris = pooled(ctx, bullet, 512);
        let mut game = Arena::default();
        let mut accumulator = 0.;
        move |ctx: &mut Ctx| {
            fit(ctx);
            ctx.clear(Color::hex(0x10171e));
            let bits = touch();
            if ctx.key_pressed(Key::R) || bits & 32 != 0 {
                game.restart();
            }
            let movement = vec2(
                axis(
                    ctx.key_down(Key::D) || ctx.key_down(Key::Right) || bits & 2 != 0,
                    ctx.key_down(Key::A) || ctx.key_down(Key::Left) || bits & 1 != 0,
                ),
                axis(
                    ctx.key_down(Key::S) || ctx.key_down(Key::Down) || bits & 64 != 0,
                    ctx.key_down(Key::W) || ctx.key_down(Key::Up) || bits & 4 != 0,
                ),
            );
            let mut aim = game.aim;
            let mouse_fire = ctx.mouse_down(MouseButton::Left);
            if mouse_fire {
                let (origin, ray) = ctx.camera3d.screen_ray(ctx.mouse_pos());
                if ray.y.abs() > 0.001 {
                    let hit = origin + ray * ((0.7 - origin.y) / ray.y);
                    aim = vec2(hit.x, hit.z) - game.pos;
                }
            } else if let Some(nearest) = game.drones.iter().min_by(|a, b| {
                a.pos
                    .distance_squared(game.pos)
                    .total_cmp(&b.pos.distance_squared(game.pos))
            }) {
                aim = nearest.pos - game.pos;
            }
            let shoot = mouse_fire || ctx.key_down(Key::Space) || bits & 8 != 0;
            let dash = ctx.key_down(Key::LeftShift) || bits & 16 != 0;
            if !paused() {
                accumulator += ctx.dt().min(0.05);
                while accumulator >= 1. / 120. {
                    game.step(
                        1. / 120.,
                        Input {
                            movement,
                            aim,
                            shoot,
                            dash,
                        },
                    );
                    accumulator -= 1. / 120.;
                }
            }
            let angle = (-game.aim.x).atan2(-game.aim.y);
            let player_scale = if game.health == 0
                || game.invulnerable > 0. && ((ctx.time() * 12.) as u32).is_multiple_of(2)
            {
                Vec3::ZERO
            } else {
                vec3(1., 1.2, 1.)
            };
            place(
                ctx,
                player,
                vec3(game.pos.x, 0.8, game.pos.y),
                player_scale,
                angle,
            );
            place(
                ctx,
                gun,
                vec3(
                    game.pos.x + game.aim.x * 0.7,
                    1.,
                    game.pos.y + game.aim.y * 0.7,
                ),
                vec3(0.22, 0.3, 1.2),
                angle,
            );
            place(
                ctx,
                player_light,
                vec3(game.pos.x, 2.5, game.pos.y),
                Vec3::ONE,
                0.,
            );
            for (i, id) in drones.iter().enumerate() {
                if let Some(d) = game.drones.get(i) {
                    place(
                        ctx,
                        *id,
                        vec3(
                            d.pos.x,
                            0.8 + (ctx.time() as f32 * 3. + i as f32).sin() * 0.14,
                            d.pos.y,
                        ),
                        Vec3::ONE,
                        ctx.time() as f32 + i as f32,
                    );
                } else {
                    place(ctx, *id, Vec3::ZERO, Vec3::ZERO, 0.);
                }
            }
            for (i, id) in shots.iter().enumerate() {
                if let Some(s) = game.shots.get(i) {
                    place(
                        ctx,
                        *id,
                        vec3(s.pos.x, 0.8, s.pos.y),
                        vec3(0.13, 0.13, 0.8),
                        (-s.velocity.x).atan2(-s.velocity.y),
                    );
                } else {
                    place(ctx, *id, Vec3::ZERO, Vec3::ZERO, 0.);
                }
            }
            for (i, id) in debris.iter().enumerate() {
                if let Some(d) = game.debris.get(i) {
                    place(
                        ctx,
                        *id,
                        vec3(d.pos.x, 0.2 + (d.life * 5.).sin().abs(), d.pos.y),
                        Vec3::splat(d.life * 0.45),
                        d.life * 8.,
                    );
                } else {
                    place(ctx, *id, Vec3::ZERO, Vec3::ZERO, 0.);
                }
            }
            hud(ctx, game.score, game.health, game.wave);
            text(
                ctx,
                &format!("{} DRONES   /   SHIFT: DASH", game.drones.len()),
                vec2(25., 570.),
                16.,
                Color::hex(0xb8c7c7),
            );
            if game.health == 0 {
                message(ctx, "OVERRUN", "Press R to try again");
            }
            report(game.score, game.health, game.wave, game.drones.len() as u32);
        }
    });
}
