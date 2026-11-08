// keeps the trails textures in the shared mod image slot
fn trails_frame(game: &mut crate::game::Game<'_>) {
    use crate::game::trails_mod as trails;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if !game.native_window() || game.native_player().is_none() || !game.native_on(trails::ID) {
        NEXT.store(0, Ordering::Relaxed);
        return;
    }
    let now = trails::now_ms();
    if now < NEXT.load(Ordering::Relaxed) {
        return;
    }
    let atlas = horizon_client::native_data!("trails", "atlas.rgba");
    let ok = game.native_mod_image(trails::ATLAS_WIDTH, trails::ATLAS_HEIGHT, atlas);
    NEXT.store(now + if ok { 5_000 } else { 1_000 }, Ordering::Relaxed);
}
