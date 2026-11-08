fn trails_lines(
    game: &crate::game::Game<'_>,
    camera: [f64; 3],
    primitives: &mut crate::world_primitives::WorldPrimitives,
) {
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
    let mut state = trails::state();
    if !matches!(state.settings.mode, trails::Mode::Line) {
        return;
    }
    let frame = state.frame(
        camera,
        trails::Camera::from_angles(0.0, 0.0),
        &View(game),
        trails::now_ms(),
    );
    for line in frame.lines {
        primitives.line(line.from, line.to, line.argb, line.width);
    }
}
