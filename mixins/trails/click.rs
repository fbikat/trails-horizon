fn trails_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::trails_mod as trails;
    if mod_id != trails::ID {
        return;
    }
    let stored = {
        let mut state = trails::state();
        let g = &*game;
        state.ensure_loaded(|k| g.native_setting(trails::ID, k));
        state.click(button).then(|| state.settings.stored())
    };
    for (key, value) in stored.unwrap_or_default() {
        game.native_set_setting(trails::ID, key, &value);
    }
}
