fn trails_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::trails_mod as trails;
    if *ran || !game.native_on(trails::ID) {
        return;
    }
    let result = {
        let mut state = trails::state();
        let g = &*game;
        state.ensure_loaded(|k| g.native_setting(trails::ID, k));
        trails::command(&mut state, command)
            .map(|(lines, save)| (lines, save.then(|| state.settings.stored())))
    };
    let Some((lines, stored)) = result else {
        return;
    };
    *ran = true;
    for (key, value) in stored.unwrap_or_default() {
        game.native_set_setting(trails::ID, key, &value);
    }
    for line in lines {
        game.native_chat(&line);
    }
}
