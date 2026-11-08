fn trails_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::trails_mod as trails;
    if !game.native_window() {
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
    let game = &*game;
    let mut state = trails::state();
    state.ensure_loaded(|key| game.native_setting(trails::ID, key));
    let on = game.native_on(trails::ID);
    let player = game.native_player().filter(|_| on);
    let input = player.as_ref().map(|p| trails::PlayerInput {
        pos: [p.x, p.y, p.z],
        yaw: p.yaw,
        dimension: p.dimension,
    });
    state.tick(input, &View(game), trails::now_ms());
}
