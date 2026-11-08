fn trails_world(game: &crate::game::Game<'_>, draw: &mut crate::game::NativeWorldDraw) {
    use crate::game::trails_mod as trails;
    if !game.native_window() || !game.native_on(trails::ID) {
        return;
    }
    struct View<'a, 'g>(&'a crate::game::Game<'g>);
    impl trails::World for View<'_, '_> {
        fn loaded(&self, b: [i32; 3]) -> bool {
            self.0.world.loaded(b[0], b[2])
        }
        fn boxes(&self, b: [i32; 3]) -> Vec<[f64; 6]> {
            self.0.native_block_collision(b)
        }
        fn light(&self, b: [i32; 3]) -> (u8, u8) {
            self.0.native_light(b)
        }
    }
    let (yaw, pitch) = match game.native_camera() {
        Some(c) => (c.yaw, c.pitch),
        None => match game.native_player() {
            Some(p) => (p.yaw, p.pitch),
            None => return,
        },
    };
    let mut state = trails::state();
    if matches!(state.settings.mode, trails::Mode::Line) {
        return;
    }
    let frame = state.frame(
        draw.camera,
        trails::Camera::from_angles(yaw, pitch),
        &View(game),
        trails::now_ms(),
    );
    let light = draw.light;
    for quad in &frame.quads {
        draw.light = quad.light;
        game.native_world_quad(draw, trails::SPRITE, quad.corners, quad.uv, quad.rgba);
    }
    draw.light = light;
}
