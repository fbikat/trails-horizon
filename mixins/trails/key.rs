fn trails_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::trails_mod as trails;
    if !pressed || !game.native_on(trails::ID) || !game.native_playing() {
        return;
    }
    if key != game.native_mod_key(trails::ID, trails::KEY, trails::DEFAULT_KEY) {
        return;
    }
    let enabled = {
        let mut state = trails::state();
        let g = &*game;
        state.ensure_loaded(|k| g.native_setting(trails::ID, k));
        state.toggle()
    };
    game.native_set_setting(trails::ID, "enabled", &enabled.to_string());
    game.native_chat(if enabled { "Trails ON" } else { "Trails OFF" });
}
