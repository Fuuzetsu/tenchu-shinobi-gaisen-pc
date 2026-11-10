use super::*;
use bytemuck::Zeroable;
use glam::Mat4;
use tenchu_core::effects::Effect;

fn floor(center: Vec3, flags: u32) -> Vec<Vertex> {
    [
        [-12000.0, 0.0, -12000.0],
        [12000.0, 0.0, -12000.0],
        [-12000.0, 0.0, 12000.0],
        [-12000.0, 0.0, 12000.0],
        [12000.0, 0.0, -12000.0],
        [12000.0, 0.0, 12000.0],
    ]
    .map(|p| Vertex {
        position: (center + Vec3::from(p)).to_array(),
        normal: Vec3::NEG_Y.to_array(),
        flags,
        opacity: 1.0,
        ..Vertex::zeroed()
    })
    .to_vec()
}

#[test]
fn offscreen_effects_preserve_crossing_tails_and_interpolated_positions() {
    let camera = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 1.0, 2000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -1000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let options = crate::settings::Effects {
        leaves: true,
        scatter: true,
        splashes: true,
        sparks: true,
        ..Default::default()
    };
    for (kind, count) in [
        (Kind::Petal, 12),
        (Kind::Leaf, 12),
        (Kind::Snow, 6),
        (Kind::Drop, 6),
        (Kind::Spark, 6),
    ] {
        let mut contact = Contact::default();
        contact.particle(kind, Vec3::X * 220.0, Vec3::ZERO, [1.0; 3], 10.0);
        assert!(contact.geometry(camera, 1.0, &options).vertices.is_empty());
        contact.particles[0].previous = -contact.particles[0].position;
        let state = postcard::to_allocvec(&contact).unwrap();
        assert_eq!(
            contact.geometry(camera, 0.5, &options).vertices.len(),
            count
        );
        assert_eq!(postcard::to_allocvec(&contact).unwrap(), state);
        if matches!(kind, Kind::Spark | Kind::Drop) {
            contact.particles[0].velocity = Vec3::X * 700.0;
            assert_eq!(
                contact.geometry(camera, 1.0, &options).vertices.len(),
                count
            );
        }
    }
    let mut contact = Contact::default();
    contact.ripple(Vec3::X * 220.0, 1.0);
    assert!(contact.geometry(camera, 0.0, &options).vertices.is_empty());
    contact.rings[0].age = 0.6;
    assert_eq!(contact.geometry(camera, 0.0, &options).vertices.len(), 192);
    contact.rings[0].age = 1.2;
    assert!(contact.geometry(camera, 0.0, &options).vertices.is_empty());
}

#[test]
fn settled_litter_fades_and_weather_clear_keeps_foliage() {
    let mut contact = Contact::default();
    contact.particle(Kind::Petal, Vec3::ZERO, Vec3::ZERO, [1.0, 0.7, 0.8], 18.0);
    contact.particles[0].ground = Some(Vec3::NEG_Y);
    let options = crate::settings::Effects {
        leaves: true,
        scatter: true,
        ..Default::default()
    };
    let mut previous = 1.0;
    for remaining in [8.0, 6.0, 4.0, 2.0, 0.0] {
        contact.particles[0].age = contact.particles[0].life - remaining;
        let mesh = contact.geometry(Mat4::IDENTITY, 1.0, &options);
        if remaining == 0.0 {
            assert!(mesh.vertices.is_empty());
            continue;
        }
        let opacity = mesh.vertices[0].opacity;
        assert!(opacity <= previous);
        assert!((opacity - remaining / 8.0).abs() < 0.001);
        assert!(
            mesh.vertices
                .iter()
                .all(|v| Vec3::from(v.position).is_finite())
        );
        previous = opacity;
    }
    contact.burst(Kind::Snow, Vec3::ZERO, 30.0, 10, [1.0; 3]);
    contact.burst(Kind::Drop, Vec3::ZERO, 30.0, 10, [1.0; 3]);
    contact.clear_weather();
    assert_eq!(contact.particles.len(), 1);
    assert!(contact.particles[0].kind == Kind::Petal);
}

#[test]
#[ignore = "requires TENCHU_DISC; checks real model classifications, contact events and core-state independence"]
fn contact_effects_follow_contacts_without_changing_gameplay() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut graphics = Graphics::default();
    graphics.set_preset(8);
    for mission in [1, 3] {
        let game = crate::session::Options {
            mission,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        let models =
            crate::visuals::models::Models::build(&game.mission, &Default::default(), &graphics)
                .unwrap();
        let canopies = models.canopies(&game);
        assert!(!canopies.is_empty(), "mission {mission}: no seasonal trees");
        assert!(canopies.iter().any(|t| t.blossom == (mission == 1)));
        eprintln!("mission {mission}: {} seasonal canopies", canopies.len());
    }
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    for _ in 0..5 {
        game.step(0).unwrap();
    }
    let origin = point(game.player().position);
    let mut triangles = floor(origin, 0);
    triangles.extend(floor(origin - Vec3::Y * 500.0, render_mesh::FOLIAGE));
    let surfaces = Surfaces::new(&triangles);
    let mut contact = Contact::default();
    contact.particle(
        Kind::Petal,
        origin - Vec3::Y * 700.0,
        Vec3::Y * 20.0,
        [1.0, 0.7, 0.8],
        18.0,
    );
    let weather = super::super::weather::SurfaceWeather::default();
    for _ in 0..90 {
        game.tick += 1;
        contact.tick(&game, &surfaces, &graphics, weather, origin);
    }
    assert!(contact.particles[0].ground.is_some());
    assert!(
        (contact.particles[0].position.y - origin.y).abs() < 3.0,
        "leaf settled on foliage instead of ground"
    );
    let state = crate::session::tests::state(&game);
    let count = contact.particles.len();
    for alpha in [0.0, 0.5, 1.0] {
        contact.tick(&game, &surfaces, &graphics, weather, origin);
        contact.geometry(Mat4::IDENTITY, alpha, &graphics.effects);
    }
    assert_eq!(count, contact.particles.len());
    assert_eq!(state, crate::session::tests::state(&game));

    let mut strengths = Vec::new();
    for speed in [50, 180] {
        let mut tracker = steps::Tracker::default();
        game.roster.get_mut(game.player_id).position = origin.as_ivec3().to_array().into();
        game.roster.get_mut(game.player_id).map.height = 0;
        tracker.update(&game, &surfaces, origin);
        for _ in 0..12 {
            game.roster.get_mut(game.player_id).position.x += speed;
            if let Some(step) = tracker.update(&game, &surfaces, origin).first() {
                strengths.push(step.strength);
                break;
            }
        }
    }
    assert_eq!(strengths.len(), 2);
    assert!(
        strengths[1] > strengths[0] * 1.5,
        "running should disturb more than walking"
    );
    let mut tracker = steps::Tracker::default();
    game.roster.get_mut(game.player_id).position =
        (origin - Vec3::Y * 600.0).as_ivec3().to_array().into();
    game.roster.get_mut(game.player_id).map.height = 600;
    tracker.update(&game, &surfaces, origin);
    game.roster.get_mut(game.player_id).position.y += 400;
    tracker.update(&game, &surfaces, origin);
    game.roster.get_mut(game.player_id).position.y += 200;
    game.roster.get_mut(game.player_id).map.height = 0;
    let landing = tracker.update(&game, &surfaces, origin);
    assert_eq!(landing.len(), 1);
    assert!(landing[0].strength > strengths[1]);

    contact.clear();
    game.effects
        .splash(origin.as_ivec3().to_array().into(), [8192; 2], 1);
    game.tick += 1;
    contact.tick(&game, &surfaces, &graphics, weather, origin);
    assert_eq!(contact.rings.len(), 1);
    assert_eq!(contact.particles.len(), 12);
    for _ in 0..40 {
        game.tick += 1;
        contact.tick(&game, &surfaces, &graphics, weather, origin);
    }
    assert!(
        !contact.wet.is_empty(),
        "splash droplets should leave temporary wet marks on nearby land"
    );
    assert!(
        contact
            .wet
            .iter()
            .flat_map(|m| m.vertices.iter())
            .all(|v| v.flags & render_mesh::WET_CONTACT != 0)
    );
    let mut recorder = crate::rewind::Recorder::default();
    recorder.begin();
    contact.capture(&mut recorder).unwrap();
    let mut restored: Contact = recorder.finish().unwrap().decode().unwrap();
    for _ in 0..30 {
        game.tick += 1;
        for state in [&mut contact, &mut restored] {
            state.tick(&game, &surfaces, &graphics, weather, origin);
        }
        recorder.begin();
        contact.capture(&mut recorder).unwrap();
        assert_eq!(
            recorder.finish().unwrap().bytes().unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
    }

    contact.clear();
    game.effects.reset();
    let start = (origin - Vec3::Y * 3000.0).as_ivec3().to_array().into();
    let end = (origin + Vec3::Y * 100.0).as_ivec3().to_array().into();
    game.effects
        .fly_wire(start, end, &game.math, &mut game.random)
        .unwrap();
    for effect in game.effects.slots.iter_mut().flatten() {
        if let Effect::FlyWire(w) = effect {
            w.mode = 1;
            w.count = 1;
        }
    }
    game.tick += 1;
    contact.tick(&game, &surfaces, &graphics, weather, origin);
    assert_eq!(contact.flashes.len(), 1);
    assert!((contact.lights()[0].light.position.y - (origin.y - 40.0)).abs() < 1.0);
    let sparks = contact.particles.len();
    game.tick += 1;
    contact.tick(&game, &surfaces, &graphics, weather, origin);
    assert!(
        contact.particles.len() <= sparks,
        "one impact must not emit a new burst each frame"
    );
    let rope = super::rope::geometry(&game);
    assert!(!rope.vertices.is_empty());
    assert!(
        rope.vertices
            .iter()
            .all(|v| Vec3::from(v.position).is_finite() && Vec3::from(v.normal).is_finite())
    );
    for normal in [Vec3::Y, Vec3::X] {
        let rotation = glam::Quat::from_rotation_arc(Vec3::NEG_Y, normal);
        let mut triangles = floor(Vec3::ZERO, 0);
        for vertex in &mut triangles {
            vertex.position = (origin + rotation * Vec3::from(vertex.position)).to_array();
            vertex.normal = normal.to_array();
        }
        let surfaces = Surfaces::new(&triangles);
        contact.clear();
        game.effects.reset();
        game.effects
            .fly_wire(
                (origin + normal * 3000.0).as_ivec3().to_array().into(),
                (origin - normal * 100.0).as_ivec3().to_array().into(),
                &game.math,
                &mut game.random,
            )
            .unwrap();
        for effect in game.effects.slots.iter_mut().flatten() {
            if let Effect::FlyWire(w) = effect {
                w.mode = 1;
                w.count = 1;
            }
        }
        game.tick += 1;
        contact.tick(&game, &surfaces, &graphics, weather, origin);
        let sparks = contact.particles.len();
        assert!(sparks > 0);
        for _ in 0..6 {
            game.tick += 1;
            contact.tick(&game, &surfaces, &graphics, weather, origin);
        }
        assert_eq!(
            contact.particles.len(),
            sparks,
            "ceiling/wall impacts must spray into open space, not into the surface"
        );
        assert!(
            contact
                .particles
                .iter()
                .all(|p| (p.position - origin).dot(normal) > 40.0)
        );
    }
    eprintln!(
        "walk/run/landing strength: {strengths:?}/{}; splash and grapple events passed",
        landing[0].strength
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; clash lights cluster particles, expire and do not retrigger while paused"]
fn clash_lights_follow_effect_births_and_expire() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    game.effects.reset();
    let mut graphics = Graphics::default();
    graphics.effects.sparks = true;
    let mut contact = Contact::default();
    let surfaces = Surfaces::default();
    let weather = super::super::weather::SurfaceWeather::default();
    for x in 0..10 {
        game.effects
            .bleed([x * 5, 0, 2000].into(), [10, -20, 10], 30, [0, 127, 255]);
    }
    let state = postcard::to_allocvec(&game).unwrap();
    let initial_sparks = crate::lighting::sparks::sources(&game, 1.0, 100, std::iter::empty());
    assert_eq!(initial_sparks.len(), 1);
    assert!(crate::lighting::sparks::sources(&game, 1.0, 0, std::iter::empty()).is_empty());
    for _ in 0..5 {
        contact.tick(&game, &surfaces, &graphics, weather, Vec3::ZERO);
        assert_eq!(
            contact.lights().len(),
            1,
            "one flash per clash, not per particle/display frame"
        );
    }
    assert_eq!(postcard::to_allocvec(&game).unwrap(), state);
    for _ in 0..12 {
        game.step(0).unwrap();
        contact.tick(&game, &surfaces, &graphics, weather, Vec3::ZERO);
    }
    assert!(
        contact.lights().is_empty(),
        "the initial flash ends while fragments remain hot"
    );
    let moving_sparks = crate::lighting::sparks::sources(&game, 1.0, 100, std::iter::empty());
    assert!(!moving_sparks.is_empty());
    assert!(
        moving_sparks[0]
            .light
            .position
            .distance(initial_sparks[0].light.position)
            > 100.0
    );
    assert!(
        game.effects
            .active_slots()
            .iter()
            .flatten()
            .any(|e| matches!(e, Effect::Bleed(_)))
    );
    game.effects.slots[200] = game.effects.slots[0].clone();
    game.effects.slots[..200].fill(None);
    game.tick += 1;
    contact.tick(&game, &surfaces, &graphics, weather, Vec3::ZERO);
    assert!(
        contact.lights().is_empty(),
        "discarded slots cannot create lights"
    );
    assert!(crate::lighting::sparks::sources(&game, 1.0, 100, std::iter::empty()).is_empty());
    game.effects.impact_ex(tenchu_core::effects::Impact {
        parent: None,
        position: [0, -800, 2000].into(),
        size: [8192, 24576],
        color: [[220; 3], [0; 3]],
        rotation: 0,
        speed: 6,
        sprite: 1,
        count: 0,
        time: 9,
    });
    game.step(0).unwrap();
    contact.tick(&game, &surfaces, &graphics, weather, Vec3::ZERO);
    assert_eq!(
        contact.lights().len(),
        1,
        "a blocked hit lights up on contact"
    );
    for _ in 0..12 {
        game.step(0).unwrap();
        contact.tick(&game, &surfaces, &graphics, weather, Vec3::ZERO);
        assert!(
            contact.lights().len() <= 1,
            "one blocked hit cannot keep renewing its flash"
        );
    }
    assert!(contact.lights().is_empty());
}
