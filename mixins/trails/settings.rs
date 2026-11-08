fn trails_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::trails_mod as trails;
    if mod_id != trails::ID {
        return;
    }
    let game = &*game;
    let mut state = trails::state();
    state.ensure_loaded(|k| game.native_setting(trails::ID, k));
    buttons.extend(state.settings.labels());
}
