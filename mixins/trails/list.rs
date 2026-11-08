fn trails_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::trails_mod as trails;
    mods.push(crate::game::native_hooks::NativeMod {
        id: trails::ID.into(),
        title: format!("Trails {}", trails::VERSION),
        description: trails::DESCRIPTION.into(),
        keys: vec![[
            trails::KEY.into(),
            "Toggle trails".into(),
            trails::DEFAULT_KEY.into(),
        ]],
    });
}
