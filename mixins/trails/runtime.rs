use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

pub const ID: &str = "trails";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Client-side paw prints, particles, lines and ribbons. Press K to toggle, /trails for options.";
pub const KEY: &str = "toggle";
pub const DEFAULT_KEY: &str = "key.keyboard.k";
pub const ATLAS_WIDTH: u32 = 512;
pub const ATLAS_HEIGHT: u32 = 128;
pub const SPRITE: &str = "horizon/modimage";
pub const FULL_BRIGHT: u32 = 0xF0 | 0xF0 << 8;

const MAX_TICK_MOVE_SQ: f64 = 16.0;
const MIN_MOVE_SQ: f64 = 1.0e-6;
const MAX_SAMPLES: usize = 4_096;
const MAX_NEW_PER_TICK: usize = 128;
const MAX_ROUTE_SEGMENTS: usize = 16_380;
const SURFACE_OFFSET: f64 = 0.006;
const HEIGHT_TOLERANCE: f64 = 1.0e-5;

pub type V3 = [f64; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len_sq(a: V3) -> f64 {
    dot(a, a)
}
fn lerp3(a: V3, b: V3, t: f64) -> V3 {
    add(a, mul(sub(b, a), t))
}
fn block_of(p: V3) -> [i32; 3] {
    [
        p[0].floor() as i32,
        p[1].floor() as i32,
        p[2].floor() as i32,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    CatPaws,
    Particle,
    Line,
    Ribbon,
}
impl Mode {
    const ALL: [Mode; 4] = [Mode::CatPaws, Mode::Particle, Mode::Line, Mode::Ribbon];
    fn key(self) -> &'static str {
        match self {
            Mode::CatPaws => "cat_paws",
            Mode::Particle => "particle",
            Mode::Line => "line",
            Mode::Ribbon => "ribbon",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Mode::CatPaws => "Cat paws",
            Mode::Particle => "Particle",
            Mode::Line => "Line",
            Mode::Ribbon => "Ribbon",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        match s.as_str() {
            "paws" | "paw" | "cat_paws" | "cat" => Some(Mode::CatPaws),
            "particle" | "particles" => Some(Mode::Particle),
            "line" | "lines" => Some(Mode::Line),
            "ribbon" | "ribbons" => Some(Mode::Ribbon),
            _ => None,
        }
    }
    fn route(self) -> bool {
        matches!(self, Mode::Line | Mode::Ribbon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Sparkle,
    Dust,
    Orbs,
    Hearts,
}
impl Style {
    const ALL: [Style; 4] = [Style::Sparkle, Style::Dust, Style::Orbs, Style::Hearts];
    fn key(self) -> &'static str {
        match self {
            Style::Sparkle => "sparkle",
            Style::Dust => "dust",
            Style::Orbs => "orbs",
            Style::Hearts => "hearts",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Style::Sparkle => "Sparkle",
            Style::Dust => "Dust",
            Style::Orbs => "Orbs",
            Style::Hearts => "Hearts",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Style::ALL
            .into_iter()
            .find(|v| v.key() == s.trim().to_ascii_lowercase())
    }
    fn height(self) -> f64 {
        match self {
            Style::Dust => 0.08,
            Style::Sparkle => 0.20,
            Style::Orbs => 0.32,
            Style::Hearts => 0.45,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Num {
    pub key: &'static str,
    pub label: &'static str,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub click: f64,
}

pub const OPACITY: Num = num("opacity", "Opacity", 0.78, 0.05, 1.0, 0.05, 0.1);
pub const FADE: Num = num("fadeDuration", "Fade duration", 6.0, 0.5, 60.0, 0.5, 2.0);
pub const DISTANCE: Num = num(
    "renderDistance",
    "Render distance",
    64.0,
    8.0,
    256.0,
    8.0,
    16.0,
);
pub const PAW_SIZE: Num = num("pawSize", "Paw size", 0.34, 0.14, 0.80, 0.02, 0.06);
pub const PAW_OPACITY: Num = num("pawOpacity", "Paw opacity", 0.72, 0.05, 1.0, 0.05, 0.1);
pub const STEP: Num = num("stepSpacing", "Step spacing", 0.58, 0.20, 1.50, 0.02, 0.1);
pub const FEET: Num = num(
    "footSeparation",
    "Foot separation",
    0.32,
    0.10,
    0.70,
    0.02,
    0.06,
);
pub const PAW_FADE: Num = num(
    "pawFadeDuration",
    "Paw fade duration",
    4.0,
    0.5,
    30.0,
    0.5,
    2.0,
);
pub const PARTICLE_SIZE: Num = num(
    "particleSize",
    "Particle size",
    0.12,
    0.03,
    0.60,
    0.01,
    0.03,
);
pub const DENSITY: Num = num("density", "Particle density", 4.0, 1.0, 12.0, 1.0, 1.0);
pub const PARTICLE_LIFE: Num = num(
    "particleLifetime",
    "Particle lifetime",
    3.0,
    0.5,
    30.0,
    0.5,
    1.0,
);
pub const WIDTH: Num = num("lineWidth", "Line / ribbon width", 3.0, 0.5, 12.0, 0.5, 1.0);
pub const SPACING: Num = num("pointSpacing", "Point spacing", 0.20, 0.05, 2.0, 0.05, 0.1);
pub const SMOOTHING: Num = num("smoothing", "Smoothing", 60.0, 0.0, 100.0, 5.0, 20.0);

const fn num(
    key: &'static str,
    label: &'static str,
    default: f64,
    min: f64,
    max: f64,
    step: f64,
    click: f64,
) -> Num {
    Num {
        key,
        label,
        default,
        min,
        max,
        step,
        click,
    }
}

const NUMS: [Num; 14] = [
    OPACITY,
    FADE,
    DISTANCE,
    PAW_SIZE,
    PAW_OPACITY,
    STEP,
    FEET,
    PAW_FADE,
    PARTICLE_SIZE,
    DENSITY,
    PARTICLE_LIFE,
    WIDTH,
    SPACING,
    SMOOTHING,
];

impl Num {
    pub fn sanitize(&self, v: f64) -> f64 {
        if !v.is_finite() {
            return self.default;
        }
        let c = v.clamp(self.min, self.max);
        let stepped = self.min + ((c - self.min) / self.step).round() * self.step;
        let stepped = stepped.clamp(self.min, self.max);
        (stepped * 1.0e6).round() / 1.0e6
    }
    fn next(&self, v: f64) -> f64 {
        if v >= self.max - self.step * 0.5 {
            self.min
        } else {
            self.sanitize((v + self.click).min(self.max))
        }
    }
}

pub const PALETTE: [(u32, &str); 10] = [
    (0xFFFF8FD2, "Pink"),
    (0xFFFFA8D8, "Soft pink"),
    (0xFFFFFFFF, "White"),
    (0xFFFF5555, "Red"),
    (0xFFFFAA33, "Orange"),
    (0xFFFFEE55, "Yellow"),
    (0xFF66EE77, "Green"),
    (0xFF55DDFF, "Cyan"),
    (0xFF5577FF, "Blue"),
    (0xFFAA66FF, "Purple"),
];

pub fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(s, 16).ok()?;
    match s.len() {
        6 => Some(0xFF00_0000 | v),
        8 => Some(v),
        _ => None,
    }
}
pub fn format_color(c: u32) -> String {
    if c >> 24 == 0xFF {
        format!("#{:06X}", c & 0xFF_FFFF)
    } else {
        format!("#{c:08X}")
    }
}
fn color_label(c: u32) -> String {
    PALETTE
        .iter()
        .find(|(p, _)| *p == c)
        .map_or_else(|| format_color(c), |(_, n)| (*n).to_owned())
}
fn next_color(c: u32) -> u32 {
    let i = PALETTE.iter().position(|(p, _)| *p == c);
    PALETTE[i.map_or(0, |i| (i + 1) % PALETTE.len())].0
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    pub mode: Mode,
    pub persistent: bool,
    pub color: u32,
    pub glow: bool,
    pub rainbow: bool,
    pub paw_color: u32,
    pub style: Style,
    pub shrink: bool,
    nums: [f64; 14],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: Mode::CatPaws,
            persistent: false,
            color: 0xFFFF8FD2,
            glow: false,
            rainbow: false,
            paw_color: 0xFFFFA8D8,
            style: Style::Sparkle,
            shrink: true,
            nums: NUMS.map(|n| n.sanitize(n.default)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Enabled,
    Mode,
    Lifetime,
    Rainbow,
    Glow,
    Color,
    PawColor,
    Style,
    Shrink,
    Num(usize),
    Clear,
    Reset,
}

fn on(b: bool) -> &'static str {
    if b { "ON" } else { "OFF" }
}

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl Settings {
    pub fn get(&self, n: Num) -> f64 {
        NUMS.iter()
            .position(|x| x.key == n.key)
            .map_or(n.default, |i| self.nums[i])
    }
    fn set_num(&mut self, i: usize, v: f64) {
        self.nums[i] = NUMS[i].sanitize(v);
    }

    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut s = Self::default();
        let flag = |k: &str, d: bool| get(k).map_or(d, |v| v.trim() == "true");
        s.enabled = flag("enabled", s.enabled);
        s.persistent =
            get("lifetimeMode").is_some_and(|v| v.trim().eq_ignore_ascii_case("persistent"));
        s.glow = flag("glow", s.glow);
        s.rainbow = flag("rainbow", s.rainbow);
        s.shrink = flag("particleFade", s.shrink);
        if let Some(m) = get("mode").and_then(|v| Mode::parse(&v)) {
            s.mode = m;
        }
        if let Some(v) = get("particleStyle").and_then(|v| Style::parse(&v)) {
            s.style = v;
        }
        if let Some(c) = get("color").and_then(|v| parse_color(&v)) {
            s.color = c;
        }
        if let Some(c) = get("pawColor").and_then(|v| parse_color(&v)) {
            s.paw_color = c;
        }
        for (i, n) in NUMS.iter().enumerate() {
            if let Some(v) = get(n.key).and_then(|v| v.trim().parse::<f64>().ok()) {
                s.set_num(i, v);
            }
        }
        s
    }

    pub fn stored(&self) -> Vec<(&'static str, String)> {
        let mut out = vec![
            ("enabled", self.enabled.to_string()),
            ("mode", self.mode.key().to_owned()),
            (
                "lifetimeMode",
                if self.persistent {
                    "persistent"
                } else {
                    "fade"
                }
                .to_owned(),
            ),
            ("color", format_color(self.color)),
            ("glow", self.glow.to_string()),
            ("rainbow", self.rainbow.to_string()),
            ("pawColor", format_color(self.paw_color)),
            ("particleStyle", self.style.key().to_owned()),
            ("particleFade", self.shrink.to_string()),
        ];
        for (i, n) in NUMS.iter().enumerate() {
            out.push((n.key, fmt_num(self.nums[i])));
        }
        out
    }

    fn actions(&self) -> Vec<Action> {
        let idx = |n: Num| NUMS.iter().position(|x| x.key == n.key).unwrap_or(0);
        let mut a = vec![
            Action::Enabled,
            Action::Mode,
            Action::Lifetime,
            Action::Rainbow,
        ];
        match self.mode {
            Mode::CatPaws => {
                a.push(Action::PawColor);
                for n in [PAW_SIZE, PAW_OPACITY, STEP, FEET, PAW_FADE] {
                    a.push(Action::Num(idx(n)));
                }
            }
            Mode::Particle => {
                a.extend([Action::Style, Action::Color, Action::Glow]);
                for n in [OPACITY, PARTICLE_SIZE, DENSITY, PARTICLE_LIFE] {
                    a.push(Action::Num(idx(n)));
                }
                a.push(Action::Shrink);
            }
            Mode::Line | Mode::Ribbon => {
                a.extend([Action::Color, Action::Glow]);
                for n in [OPACITY, WIDTH, SPACING, SMOOTHING, FADE] {
                    a.push(Action::Num(idx(n)));
                }
            }
        }
        a.extend([Action::Num(idx(DISTANCE)), Action::Clear, Action::Reset]);
        a
    }

    fn label(&self, a: Action) -> String {
        match a {
            Action::Enabled => format!("Trails: {}", on(self.enabled)),
            Action::Mode => format!("Mode: {}", self.mode.label()),
            Action::Lifetime => format!(
                "Lifetime: {}",
                if self.persistent {
                    "Persistent"
                } else {
                    "Fade"
                }
            ),
            Action::Rainbow => format!("Rainbow / chroma: {}", on(self.rainbow)),
            Action::Glow => format!("Glow: {}", on(self.glow)),
            Action::Color => format!("Color: {}", color_label(self.color)),
            Action::PawColor => format!("Paw color: {}", color_label(self.paw_color)),
            Action::Style => format!("Particle style: {}", self.style.label()),
            Action::Shrink => format!("Shrink while fading: {}", on(self.shrink)),
            Action::Num(i) => format!("{}: {}", NUMS[i].label, fmt_num(self.nums[i])),
            Action::Clear => "Clear trail".to_owned(),
            Action::Reset => "Reset defaults".to_owned(),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        self.actions().into_iter().map(|a| self.label(a)).collect()
    }

    fn apply(&mut self, a: Action) -> Effect {
        match a {
            Action::Enabled => {
                self.enabled = !self.enabled;
                return Effect::Clear;
            }
            Action::Mode => {
                let i = Mode::ALL.iter().position(|m| *m == self.mode).unwrap_or(0);
                self.mode = Mode::ALL[(i + 1) % Mode::ALL.len()];
            }
            Action::Lifetime => self.persistent = !self.persistent,
            Action::Rainbow => self.rainbow = !self.rainbow,
            Action::Glow => self.glow = !self.glow,
            Action::Color => self.color = next_color(self.color),
            Action::PawColor => self.paw_color = next_color(self.paw_color),
            Action::Style => {
                let i = Style::ALL
                    .iter()
                    .position(|s| *s == self.style)
                    .unwrap_or(0);
                self.style = Style::ALL[(i + 1) % Style::ALL.len()];
            }
            Action::Shrink => self.shrink = !self.shrink,
            Action::Num(i) => self.set_num(i, NUMS[i].next(self.nums[i])),
            Action::Clear => return Effect::Clear,
            Action::Reset => {
                *self = Self::default();
                return Effect::Clear;
            }
        }
        Effect::None
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let key_lc = key.to_ascii_lowercase();
        let bool_of = |v: &str| match v.trim().to_ascii_lowercase().as_str() {
            "on" | "true" | "yes" | "1" => Ok(true),
            "off" | "false" | "no" | "0" => Ok(false),
            _ => Err(format!("expected on/off, got '{v}'")),
        };
        match key_lc.as_str() {
            "enabled" => self.enabled = bool_of(value)?,
            "mode" => {
                self.mode = Mode::parse(value).ok_or("modes: paws, particle, line, ribbon")?
            }
            "lifetime" | "lifetimemode" => {
                self.persistent = match value.trim().to_ascii_lowercase().as_str() {
                    "persistent" => true,
                    "fade" => false,
                    _ => return Err("lifetime: fade or persistent".into()),
                }
            }
            "color" => self.color = parse_color(value).ok_or("color: #RRGGBB or #AARRGGBB")?,
            "pawcolor" => {
                self.paw_color = parse_color(value).ok_or("color: #RRGGBB or #AARRGGBB")?
            }
            "glow" => self.glow = bool_of(value)?,
            "rainbow" => self.rainbow = bool_of(value)?,
            "style" | "particlestyle" => {
                self.style = Style::parse(value).ok_or("styles: sparkle, dust, orbs, hearts")?
            }
            "shrink" | "particlefade" => self.shrink = bool_of(value)?,
            _ => {
                let i = NUMS
                    .iter()
                    .position(|n| n.key.eq_ignore_ascii_case(key))
                    .ok_or_else(|| format!("unknown setting '{key}'"))?;
                let v: f64 = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("'{value}' is not a number"))?;
                self.set_num(i, v);
            }
        }
        Ok(())
    }

    pub fn keys() -> Vec<String> {
        let mut k: Vec<String> = [
            "enabled", "mode", "lifetime", "color", "pawColor", "glow", "rainbow", "style",
            "shrink",
        ]
        .map(String::from)
        .to_vec();
        k.extend(
            NUMS.iter()
                .map(|n| format!("{} ({}..{})", n.key, n.min, n.max)),
        );
        k
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Clear,
}

#[derive(Clone, Debug)]
struct Paw {
    pos: V3,
    forward: V3,
    born: u64,
    support: [i32; 3],
}
#[derive(Clone, Debug)]
struct Spark {
    pos: V3,
    born: u64,
    phase: f64,
}
#[derive(Clone, Debug)]
struct RoutePoint {
    pos: V3,
    born: u64,
    break_before: bool,
}
#[derive(Clone, Debug)]
struct Segment {
    from: V3,
    to: V3,
    from_born: u64,
    to_born: u64,
}

pub trait World {
    fn loaded(&self, block: [i32; 3]) -> bool;
    fn boxes(&self, block: [i32; 3]) -> Vec<[f64; 6]>;
    fn light(&self, block: [i32; 3]) -> (u8, u8);
}

pub fn surface_below<W: World + ?Sized>(
    w: &W,
    x: f64,
    z: f64,
    from: f64,
    to: f64,
) -> Option<(f64, [i32; 3])> {
    let (bx, bz) = (x.floor() as i32, z.floor() as i32);
    let (lx, lz) = (x - bx as f64, z - bz as f64);
    let mut by = from.floor() as i32;
    let bottom = to.floor() as i32;
    while by >= bottom {
        let block = [bx, by, bz];
        if !w.loaded(block) {
            return None;
        }
        let top = w
            .boxes(block)
            .into_iter()
            .filter(|b| lx >= b[0] && lx <= b[3] && lz >= b[2] && lz <= b[5])
            .map(|b| by as f64 + b[4])
            .filter(|&t| t <= from + 1.0e-7 && t >= to)
            .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))));
        if let Some(t) = top {
            return Some((t, block));
        }
        by -= 1;
    }
    None
}

fn supported<W: World + ?Sized>(
    w: &W,
    paw: &Paw,
    cache: &mut HashMap<[i32; 3], Vec<[f64; 6]>>,
) -> bool {
    let contact = paw.pos[1] - SURFACE_OFFSET;
    let (lx, lz) = (
        paw.pos[0] - paw.support[0] as f64,
        paw.pos[2] - paw.support[2] as f64,
    );
    let boxes = cache
        .entry(paw.support)
        .or_insert_with(|| w.boxes(paw.support));
    boxes.iter().any(|b| {
        lx >= b[0]
            && lx <= b[3]
            && lz >= b[2]
            && lz <= b[5]
            && (paw.support[1] as f64 + b[4] - contact).abs() <= HEIGHT_TOLERANCE
    })
}

pub struct PlayerInput<'a> {
    pub pos: V3,
    pub yaw: f32,
    pub dimension: &'a str,
}

#[derive(Default)]
pub struct Trails {
    pub settings: Settings,
    loaded: bool,
    paws: VecDeque<Paw>,
    sparks: VecDeque<Spark>,
    route: VecDeque<RoutePoint>,
    dimension: Option<String>,
    mode: Option<Mode>,
    previous: Option<V3>,
    distance: f64,
    right_foot: bool,
    sequence: u64,
    added: usize,
    revision: u64,
    cache_key: Option<(u64, u32)>,
    cache: Vec<Segment>,
}

pub fn state() -> MutexGuard<'static, Trails> {
    static STATE: OnceLock<Mutex<Trails>> = OnceLock::new();
    STATE
        .get_or_init(|| Mutex::new(Trails::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn noise(value: u64) -> f64 {
    let mut m = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    m = (m ^ (m >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    m = (m ^ (m >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    m ^= m >> 31;
    ((m >> 40) & 0xFF_FFFF) as f64 / 8_388_607.5 - 1.0
}

fn millis(seconds: f64) -> u64 {
    (seconds * 1000.0).round().max(1.0) as u64
}

fn life(persistent: bool, age: u64, lifetime: u64) -> f64 {
    if persistent {
        return 1.0;
    }
    let p = (age as f64 / lifetime.max(1) as f64).clamp(0.0, 1.0);
    1.0 - p * p * (3.0 - 2.0 * p)
}

pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> u32 {
    let i = ((h * 6.0).floor() as i64).rem_euclid(6);
    let f = h * 6.0 - (h * 6.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    let c = |x: f64| ((x * 255.0).round() as u32).min(255);
    c(r) << 16 | c(g) << 8 | c(b)
}

pub fn tint(base: u32, opacity: f64, rainbow: bool, now: u64, pos: V3) -> u32 {
    let mut rgb = base & 0xFF_FFFF;
    if rainbow {
        let hue = (now as f64 / 5000.0 + (pos[0] + pos[1] + pos[2]) * 0.018).rem_euclid(1.0);
        rgb = hsv_to_rgb(hue, 0.55, 1.0);
    }
    let a = ((255.0 * (base >> 24) as f64 / 255.0 * opacity).round() as i64).clamp(0, 255) as u32;
    a << 24 | rgb
}

pub fn pack_light((block, sky): (u8, u8)) -> u32 {
    (u32::from(block.min(15)) << 4) | (u32::from(sky.min(15)) << 4) << 8
}

fn catmull(p0: V3, p1: V3, p2: V3, p3: V3, t: f64) -> V3 {
    let (t2, t3) = (t * t, t * t * t);
    let f = |a: f64, b: f64, c: f64, d: f64| {
        0.5 * ((2.0 * b)
            + (-a + c) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };
    [
        f(p0[0], p1[0], p2[0], p3[0]),
        f(p0[1], p1[1], p2[1], p3[1]),
        f(p0[2], p1[2], p2[2], p3[2]),
    ]
}

fn smooth(points: &VecDeque<RoutePoint>, smoothing: f64) -> Vec<Segment> {
    let mut out = Vec::new();
    if points.len() < 2 {
        return out;
    }
    let subdivisions = if smoothing <= 0.0 {
        1
    } else {
        1 + (smoothing * 3.0).ceil() as usize
    };
    for i in 1..points.len() {
        if out.len() >= MAX_ROUTE_SEGMENTS {
            break;
        }
        let (a, b) = (&points[i - 1], &points[i]);
        if b.break_before {
            continue;
        }
        let before = if i >= 2 && !a.break_before {
            &points[i - 2]
        } else {
            a
        };
        let after = if i + 1 < points.len() && !points[i + 1].break_before {
            &points[i + 1]
        } else {
            b
        };
        let (mut last, mut last_born) = (a.pos, a.born);
        for step in 1..=subdivisions {
            if out.len() >= MAX_ROUTE_SEGMENTS {
                break;
            }
            let t = step as f64 / subdivisions as f64;
            let linear = lerp3(a.pos, b.pos, t);
            let curved = catmull(before.pos, a.pos, b.pos, after.pos, t);
            let pos = lerp3(linear, curved, smoothing);
            let born = (a.born as f64 + (b.born as f64 - a.born as f64) * t).round() as u64;
            out.push(Segment {
                from: last,
                to: pos,
                from_born: last_born,
                to_born: born,
            });
            last = pos;
            last_born = born;
        }
    }
    out
}

fn trim<T>(q: &mut VecDeque<T>) -> bool {
    let mut trimmed = false;
    while q.len() > MAX_SAMPLES {
        q.pop_front();
        trimmed = true;
    }
    trimmed
}

impl Trails {
    pub fn ensure_loaded(&mut self, get: impl Fn(&str) -> Option<String>) {
        if !self.loaded {
            self.settings = Settings::read(get);
            self.loaded = true;
        }
    }

    pub fn clear(&mut self) {
        self.paws.clear();
        self.sparks.clear();
        self.route.clear();
        self.dimension = None;
        self.mode = None;
        self.previous = None;
        self.distance = 0.0;
        self.right_foot = false;
        self.sequence = 0;
        self.added = 0;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn click(&mut self, button: usize) -> bool {
        let Some(a) = self.settings.actions().get(button).copied() else {
            return false;
        };
        if self.settings.apply(a) == Effect::Clear {
            self.clear();
        }
        true
    }

    pub fn toggle(&mut self) -> bool {
        self.settings.enabled = !self.settings.enabled;
        self.clear();
        self.settings.enabled
    }

    pub fn tick<W: World + ?Sized>(
        &mut self,
        player: Option<PlayerInput<'_>>,
        world: &W,
        now: u64,
    ) {
        let Some(player) = player.filter(|_| self.settings.enabled) else {
            if self.previous.is_some()
                || !self.paws.is_empty()
                || !self.route.is_empty()
                || !self.sparks.is_empty()
            {
                self.clear();
            }
            return;
        };
        let mode = self.settings.mode;
        if self.dimension.as_deref() != Some(player.dimension) || self.mode != Some(mode) {
            self.clear();
            self.dimension = Some(player.dimension.to_owned());
            self.mode = Some(mode);
        }
        self.added = 0;
        self.expire(now);
        if !self.paws.is_empty() {
            let mut cache = HashMap::new();
            self.paws
                .retain(|p| !world.loaded(p.support) || supported(world, p, &mut cache));
        }
        let current = player.pos;
        let Some(previous) = self.previous else {
            self.previous = Some(current);
            if mode.route() {
                self.add_route(add(current, [0.0, 0.065, 0.0]), now, true);
            }
            return;
        };
        let movement = sub(current, previous);
        if len_sq(movement) > MAX_TICK_MOVE_SQ {
            self.distance = 0.0;
            if mode.route() {
                self.add_route(add(current, [0.0, 0.065, 0.0]), now, true);
            }
            self.previous = Some(current);
            return;
        }
        match mode {
            Mode::CatPaws => self.update_paws(world, previous, current, now),
            Mode::Particle => self.update_sparks(previous, current, player.yaw, now),
            Mode::Line | Mode::Ribbon => {
                let length = len_sq(movement).sqrt();
                if length * length > MIN_MOVE_SQ {
                    let spacing = self.settings.get(SPACING);
                    for s in self.samples(previous, current, length, spacing) {
                        self.add_route(add(s, [0.0, 0.065, 0.0]), now, false);
                    }
                }
            }
        }
        self.previous = Some(current);
    }

    fn samples(&mut self, start: V3, end: V3, length: f64, spacing: f64) -> Vec<V3> {
        let mut out = Vec::new();
        if self.distance >= spacing {
            self.distance %= spacing;
        }
        let dir = mul(sub(end, start), 1.0 / length);
        let mut cursor = start;
        let mut remaining = length;
        let mut until = spacing - self.distance;
        while remaining + 1.0e-7 >= until && self.added + out.len() < MAX_NEW_PER_TICK {
            cursor = add(cursor, mul(dir, until));
            out.push(cursor);
            remaining -= until;
            self.distance = 0.0;
            until = spacing;
        }
        if self.added + out.len() >= MAX_NEW_PER_TICK {
            self.distance = 0.0;
        } else {
            self.distance += remaining;
        }
        out
    }

    fn update_paws<W: World + ?Sized>(&mut self, world: &W, previous: V3, current: V3, now: u64) {
        let on_ground = surface_below(
            world,
            current[0],
            current[2],
            current[1] + 0.01,
            current[1] - 0.06,
        )
        .is_some();
        let jumping = (current[1] - previous[1]).abs() > 0.3;
        if !on_ground || jumping {
            self.distance = 0.0;
            return;
        }
        let horizontal = [current[0] - previous[0], 0.0, current[2] - previous[2]];
        let length = len_sq(horizontal).sqrt();
        if length * length <= MIN_MOVE_SQ {
            return;
        }
        let forward = mul(horizontal, 1.0 / length);
        let right = [-forward[2], 0.0, forward[0]];
        let spacing = self.settings.get(STEP);
        let separation = self.settings.get(FEET);
        for sample in self.samples(previous, current, length, spacing) {
            let side = if self.right_foot { 0.5 } else { -0.5 } * separation;
            let foot = add(sample, mul(right, side));
            if let Some((y, support)) =
                surface_below(world, foot[0], foot[2], foot[1] + 0.40, foot[1] - 1.60)
                && self.added < MAX_NEW_PER_TICK
            {
                self.paws.push_back(Paw {
                    pos: [foot[0], y + SURFACE_OFFSET, foot[2]],
                    forward,
                    born: now,
                    support,
                });
                self.added += 1;
                trim(&mut self.paws);
            }
            self.right_foot = !self.right_foot;
        }
    }

    fn update_sparks(&mut self, previous: V3, current: V3, yaw: f32, now: u64) {
        let movement = sub(current, previous);
        let length = len_sq(movement).sqrt();
        if length * length <= MIN_MOVE_SQ {
            return;
        }
        let spacing = (1.0 / self.settings.get(DENSITY)).max(0.08);
        let horizontal = [movement[0], 0.0, movement[2]];
        let forward = if len_sq(horizontal) > MIN_MOVE_SQ {
            mul(horizontal, 1.0 / len_sq(horizontal).sqrt())
        } else {
            let y = f64::from(yaw).to_radians();
            [-y.sin(), 0.0, y.cos()]
        };
        let right = [-forward[2], 0.0, forward[0]];
        let height = self.settings.style.height();
        for sample in self.samples(previous, current, length, spacing) {
            let seq = self.sequence;
            self.sequence += 1;
            let lateral = noise(seq * 2 + 1) * 0.12;
            let vertical = noise(seq * 2 + 2) * 0.05;
            let pos = add(
                add(add(sample, mul(forward, -0.24)), mul(right, lateral)),
                [0.0, height + vertical, 0.0],
            );
            if self.added < MAX_NEW_PER_TICK {
                self.sparks.push_back(Spark {
                    pos,
                    born: now,
                    phase: (noise(seq + 17) + 1.0) * std::f64::consts::PI,
                });
                self.added += 1;
                trim(&mut self.sparks);
            }
        }
    }

    fn add_route(&mut self, pos: V3, now: u64, break_before: bool) {
        if self.added >= MAX_NEW_PER_TICK {
            return;
        }
        self.route.push_back(RoutePoint {
            pos,
            born: now,
            break_before,
        });
        self.added += 1;
        self.revision = self.revision.wrapping_add(1);
        trim(&mut self.route);
    }

    fn expire(&mut self, now: u64) {
        if self.settings.persistent {
            return;
        }
        let cut = |s: f64| now.saturating_sub(millis(s));
        let paw_cut = cut(self.settings.get(PAW_FADE));
        while self.paws.front().is_some_and(|p| p.born < paw_cut) {
            self.paws.pop_front();
        }
        let spark_cut = cut(self.settings.get(PARTICLE_LIFE));
        while self.sparks.front().is_some_and(|p| p.born < spark_cut) {
            self.sparks.pop_front();
        }
        let route_cut = cut(self.settings.get(FADE));
        while self.route.front().is_some_and(|p| p.born < route_cut) {
            self.route.pop_front();
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn segments(&mut self) -> &[Segment] {
        let smoothing = self.settings.get(SMOOTHING) / 100.0;
        let key = (self.revision, (smoothing as f32).to_bits());
        if self.cache_key != Some(key) {
            self.cache = smooth(&self.route, smoothing);
            self.cache_key = Some(key);
        }
        &self.cache
    }

    pub fn frame<W: World + ?Sized>(
        &mut self,
        camera: V3,
        view: Camera,
        world: &W,
        now: u64,
    ) -> Frame {
        let mut frame = Frame::default();
        if !self.settings.enabled {
            return frame;
        }
        let s = self.settings.clone();
        let max_sq = s.get(DISTANCE).powi(2);
        let mut lights: HashMap<[i32; 3], Option<u32>> = HashMap::new();
        let mut light_at = |p: V3| -> Option<u32> {
            let b = block_of(p);
            *lights.entry(b).or_insert_with(|| {
                world.loaded(b).then(|| {
                    if s.glow {
                        FULL_BRIGHT
                    } else {
                        pack_light(world.light(b))
                    }
                })
            })
        };
        match s.mode {
            Mode::CatPaws => {
                let size = s.get(PAW_SIZE);
                let lifetime = millis(s.get(PAW_FADE));
                let mut cache = HashMap::new();
                for paw in &self.paws {
                    if len_sq(sub(paw.pos, camera)) > max_sq || !world.loaded(paw.support) {
                        continue;
                    }
                    if !supported(world, paw, &mut cache) {
                        continue;
                    }
                    let b = block_of(add(paw.pos, [0.0, 0.04, 0.0]));
                    if !world.loaded(b) {
                        continue;
                    }
                    let light = pack_light(world.light(b));
                    let l = life(s.persistent, now.saturating_sub(paw.born), lifetime);
                    let color = tint(s.paw_color, s.get(PAW_OPACITY) * l, s.rainbow, now, paw.pos);
                    if color >> 24 == 0 {
                        continue;
                    }
                    let c = sub(paw.pos, camera);
                    let half = size * 0.5;
                    let f = mul(paw.forward, half);
                    let r = mul([-paw.forward[2], 0.0, paw.forward[0]], half);
                    let front_left = sub(add(c, f), r);
                    let back_left = sub(sub(c, f), r);
                    let back_right = add(sub(c, f), r);
                    let front_right = add(add(c, f), r);
                    frame.push(
                        [front_left, back_left, back_right, front_right],
                        Tex::Paw,
                        color,
                        light,
                    );
                }
            }
            Mode::Particle => {
                let base = s.get(PARTICLE_SIZE);
                let lifetime = millis(s.get(PARTICLE_LIFE));
                let (right, up) = (view.right, view.up);
                let tex = if s.style == Style::Hearts {
                    Tex::Heart
                } else {
                    Tex::Glow
                };
                for p in &self.sparks {
                    let age = now.saturating_sub(p.born);
                    let secs = age as f64 / 1000.0;
                    let bob = match s.style {
                        Style::Orbs => (p.phase + secs * 2.2).sin() * 0.035,
                        Style::Hearts => (p.phase + secs * 1.7).sin() * 0.025,
                        _ => 0.0,
                    };
                    let pos = add(p.pos, [0.0, bob, 0.0]);
                    if len_sq(sub(pos, camera)) > max_sq {
                        continue;
                    }
                    let Some(light) = light_at(pos) else { continue };
                    let l = life(s.persistent, age, lifetime);
                    let mut size = base;
                    if s.shrink && !s.persistent {
                        size *= 0.55 + 0.45 * l.sqrt();
                    }
                    let color = tint(s.color, s.get(OPACITY) * l, s.rainbow, now, p.pos);
                    if color >> 24 == 0 {
                        continue;
                    }
                    let c = sub(pos, camera);
                    match s.style {
                        Style::Sparkle => {
                            let a = p.phase + secs * 1.8;
                            let rr = add(mul(right, a.cos()), mul(up, a.sin()));
                            let ru = sub(mul(up, a.cos()), mul(right, a.sin()));
                            frame.billboard(c, rr, ru, size * 2.0, size * 0.32, tex, color, light);
                            frame.billboard(c, rr, ru, size * 0.32, size * 2.0, tex, color, light);
                        }
                        Style::Dust => frame.billboard(
                            c,
                            right,
                            up,
                            size * 0.55,
                            size * 0.55,
                            tex,
                            color,
                            light,
                        ),
                        Style::Orbs => {
                            frame.billboard(c, right, up, size * 1.2, size * 1.2, tex, color, light)
                        }
                        Style::Hearts => frame.billboard(
                            c,
                            right,
                            up,
                            size * 1.35,
                            size * 1.35,
                            tex,
                            color,
                            light,
                        ),
                    }
                }
            }
            Mode::Line | Mode::Ribbon => {
                let ribbon = s.mode == Mode::Ribbon;
                let lifetime = millis(s.get(FADE));
                let width = s.get(WIDTH);
                let opacity = s.get(OPACITY);
                let segments = self.segments().to_vec();
                for seg in segments {
                    let fl = life(s.persistent, now.saturating_sub(seg.from_born), lifetime);
                    let tl = life(s.persistent, now.saturating_sub(seg.to_born), lifetime);
                    if fl <= 0.0 && tl <= 0.0 {
                        continue;
                    }
                    let mid = mul(add(seg.from, seg.to), 0.5);
                    if len_sq(sub(mid, camera)) > max_sq {
                        continue;
                    }
                    let (Some(a), Some(b)) = (light_at(seg.from), light_at(seg.to)) else {
                        continue;
                    };
                    if ribbon {
                        let d = sub(seg.to, seg.from);
                        let side = [-d[2], 0.0, d[0]];
                        let side = if len_sq(side) < 1.0e-8 {
                            [1.0, 0.0, 0.0]
                        } else {
                            mul(side, 1.0 / len_sq(side).sqrt())
                        };
                        let off = mul(side, width * 0.020);
                        let (from, to) = (sub(seg.from, camera), sub(seg.to, camera));
                        let color = tint(s.color, opacity * (fl + tl) * 0.5, s.rainbow, now, mid);
                        if color >> 24 == 0 {
                            continue;
                        }
                        frame.push(
                            [sub(from, off), add(from, off), add(to, off), sub(to, off)],
                            Tex::White,
                            color,
                            average_light(a, b),
                        );
                    } else {
                        let color = tint(s.color, opacity * (fl + tl) * 0.5, s.rainbow, now, mid);
                        if color >> 24 == 0 {
                            continue;
                        }
                        let shade = if s.glow {
                            1.0
                        } else {
                            brightness(average_light(a, b))
                        };
                        frame.lines.push(Line {
                            from: seg.from,
                            to: seg.to,
                            argb: darken(color, shade),
                            width: width as f32,
                        });
                    }
                }
            }
        }
        frame
    }
}

fn average_light(a: u32, b: u32) -> u32 {
    let block = ((a & 0xFF) + (b & 0xFF)) / 2;
    let sky = (((a >> 8) & 0xFF) + ((b >> 8) & 0xFF)) / 2;
    block | sky << 8
}

pub fn brightness(light: u32) -> f64 {
    let level = ((light & 0xFF).max((light >> 8) & 0xFF) >> 4) as f64;
    0.25 + 0.75 * (level / 15.0)
}

pub fn darken(argb: u32, shade: f64) -> u32 {
    let c =
        |shift: u32| ((((argb >> shift) & 0xFF) as f64 * shade).round() as u32).min(255) << shift;
    (argb & 0xFF00_0000) | c(16) | c(8) | c(0)
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub right: V3,
    pub up: V3,
}
impl Camera {
    pub fn from_angles(yaw_deg: f32, pitch_deg: f32) -> Self {
        let (y, p) = (
            f64::from(yaw_deg).to_radians(),
            f64::from(pitch_deg).to_radians(),
        );
        let forward = [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()];
        let right = [-y.cos(), 0.0, -y.sin()];
        Self {
            right,
            up: cross(right, forward),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tex {
    Paw,
    Glow,
    Heart,
    White,
}
impl Tex {
    pub fn uv(self) -> [f32; 4] {
        let cell = 16.0 * 128.0 / 1024.0;
        let x = match self {
            Tex::Paw => 0.0,
            Tex::Glow => cell,
            Tex::Heart => cell * 2.0,
            Tex::White => return [cell * 3.25, cell * 0.25, cell * 3.75, cell * 0.75],
        };
        [x, 0.0, x + cell, cell]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quad {
    pub corners: [[f32; 3]; 4],
    pub uv: [f32; 4],
    pub rgba: [u8; 4],
    pub light: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub from: V3,
    pub to: V3,
    pub argb: u32,
    pub width: f32,
}

#[derive(Default, Debug)]
pub struct Frame {
    pub quads: Vec<Quad>,
    pub lines: Vec<Line>,
}

impl Frame {
    fn push(&mut self, c: [V3; 4], tex: Tex, argb: u32, light: u32) {
        let normal = cross(sub(c[1], c[0]), sub(c[2], c[1]));
        let center = mul(add(add(c[0], c[1]), add(c[2], c[3])), 0.25);
        let [u0, v0, u1, v1] = tex.uv();
        let (corners, uv) = if dot(normal, mul(center, -1.0)) >= 0.0 {
            (c, [u0, v0, u1, v1])
        } else {
            ([c[3], c[2], c[1], c[0]], [u1, v0, u0, v1])
        };
        let f = |v: V3| [v[0] as f32, v[1] as f32, v[2] as f32];
        self.quads.push(Quad {
            corners: corners.map(f),
            uv,
            rgba: [
                (argb >> 16) as u8,
                (argb >> 8) as u8,
                argb as u8,
                (argb >> 24) as u8,
            ],
            light,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn billboard(
        &mut self,
        c: V3,
        right: V3,
        up: V3,
        w: f64,
        h: f64,
        tex: Tex,
        argb: u32,
        light: u32,
    ) {
        let hr = mul(right, w * 0.5);
        let hu = mul(up, h * 0.5);
        let top_left = add(sub(c, hr), hu);
        let bottom_left = sub(sub(c, hr), hu);
        let bottom_right = sub(add(c, hr), hu);
        let top_right = add(add(c, hr), hu);
        self.push(
            [top_left, bottom_left, bottom_right, top_right],
            tex,
            argb,
            light,
        );
    }
}

pub fn help() -> Vec<String> {
    vec![
        "Trails: /trails on|off|toggle, /trails mode <paws|particle|line|ribbon>".into(),
        "/trails set <setting> <value>, /trails settings, /trails clear, /trails reset".into(),
    ]
}

pub fn command(t: &mut Trails, input: &str) -> Option<(Vec<String>, bool)> {
    let mut parts = input.split_whitespace();
    if !parts.next()?.eq_ignore_ascii_case("trails") {
        return None;
    }
    let args: Vec<&str> = parts.collect();
    let reply = |s: String| Some((vec![s], true));
    match args.as_slice() {
        [] | ["help"] => Some((help(), false)),
        ["on"] | ["off"] | ["toggle"] => {
            let want = match args[0] {
                "on" => true,
                "off" => false,
                _ => !t.settings.enabled,
            };
            t.settings.enabled = want;
            t.clear();
            reply(format!("Trails {}", on(want)))
        }
        ["clear"] => {
            t.clear();
            Some((vec!["Trail cleared".into()], false))
        }
        ["reset"] => {
            t.settings = Settings::default();
            t.clear();
            reply("Trail settings reset".into())
        }
        ["settings"] | ["get"] => {
            let mut lines = vec![format!("Settings: {}", Settings::keys().join(", "))];
            lines.push(
                t.settings
                    .stored()
                    .into_iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            Some((lines, false))
        }
        ["mode", m] => match Mode::parse(m) {
            Some(mode) => {
                t.settings.mode = mode;
                reply(format!("Trail mode: {}", mode.label()))
            }
            None => Some((vec!["Modes: paws, particle, line, ribbon".into()], false)),
        },
        ["set", key, value @ ..] if !value.is_empty() => {
            let value = value.join(" ");
            match t.settings.set(key, &value) {
                Ok(()) => {
                    if key.eq_ignore_ascii_case("enabled") {
                        t.clear();
                    }
                    reply(format!("Trails: {key} = {value}"))
                }
                Err(e) => Some((vec![format!("Trails: {e}")], false)),
            }
        }
        _ => Some((help(), false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat {
        top: f64,
    }
    impl World for Flat {
        fn loaded(&self, _: [i32; 3]) -> bool {
            true
        }
        fn boxes(&self, b: [i32; 3]) -> Vec<[f64; 6]> {
            if b[1] as f64 + 1.0 <= self.top + 1e-9 && b[1] as f64 + 1.0 > self.top - 1.0 {
                vec![[0.0, 0.0, 0.0, 1.0, self.top - b[1] as f64, 1.0]]
            } else {
                vec![]
            }
        }
        fn light(&self, _: [i32; 3]) -> (u8, u8) {
            (0, 15)
        }
    }

    fn walk(t: &mut Trails, w: &Flat, steps: usize, dz: f64) {
        for i in 0..steps {
            let pos = [0.5, w.top, 0.5 + i as f64 * dz];
            t.tick(
                Some(PlayerInput {
                    pos,
                    yaw: 0.0,
                    dimension: "minecraft:overworld",
                }),
                w,
                i as u64 * 50,
            );
        }
    }

    #[test]
    fn paws_land_on_the_surface_and_alternate() {
        let w = Flat { top: 64.0 };
        let mut t = Trails {
            loaded: true,
            ..Default::default()
        };
        walk(&mut t, &w, 20, 0.2);
        assert!(t.paws.len() >= 5);
        for p in &t.paws {
            assert!((p.pos[1] - (64.0 + SURFACE_OFFSET)).abs() < 1e-9);
        }
        let xs: Vec<f64> = t.paws.iter().map(|p| p.pos[0]).collect();
        assert!(xs.windows(2).all(|w| (w[0] - w[1]).abs() > 0.2));
    }

    #[test]
    fn paws_disappear_when_support_changes() {
        let mut t = Trails {
            loaded: true,
            ..Default::default()
        };
        walk(&mut t, &Flat { top: 64.0 }, 20, 0.2);
        assert!(!t.paws.is_empty());
        walk(&mut t, &Flat { top: 63.5 }, 1, 0.0);
        assert!(t.paws.is_empty());
    }

    #[test]
    fn half_slab_surface_is_found() {
        let w = Flat { top: 64.5 };
        assert_eq!(
            surface_below(&w, 0.3, 0.7, 65.0, 63.0).map(|s| s.0),
            Some(64.5)
        );
    }

    #[test]
    fn dimension_change_clears() {
        let w = Flat { top: 64.0 };
        let mut t = Trails::default();
        walk(&mut t, &w, 20, 0.2);
        assert!(!t.paws.is_empty());
        t.tick(
            Some(PlayerInput {
                pos: [0.5, 64.0, 5.0],
                yaw: 0.0,
                dimension: "minecraft:the_nether",
            }),
            &w,
            2000,
        );
        assert!(t.paws.is_empty());
    }

    #[test]
    fn fade_expires_and_persistent_keeps() {
        let w = Flat { top: 64.0 };
        let mut t = Trails::default();
        walk(&mut t, &w, 20, 0.2);
        let n = t.paws.len();
        t.expire(1_000_000);
        assert!(t.paws.is_empty() && n > 0);
        let mut t = Trails::default();
        t.settings.persistent = true;
        walk(&mut t, &w, 20, 0.2);
        t.expire(1_000_000);
        assert!(!t.paws.is_empty());
    }

    #[test]
    fn route_breaks_on_teleport_and_smooths() {
        let w = Flat { top: 64.0 };
        let mut t = Trails::default();
        t.settings.mode = Mode::Line;
        walk(&mut t, &w, 10, 0.3);
        t.tick(
            Some(PlayerInput {
                pos: [100.0, 64.0, 100.0],
                yaw: 0.0,
                dimension: "minecraft:overworld",
            }),
            &w,
            600,
        );
        let segs = t.segments().to_vec();
        assert!(!segs.is_empty());
        assert!(segs.iter().all(|s| len_sq(sub(s.to, s.from)) < 1.0));
    }

    #[test]
    fn line_frame_emits_primitives_and_ribbon_quads() {
        let w = Flat { top: 64.0 };
        let mut t = Trails::default();
        t.settings.mode = Mode::Line;
        walk(&mut t, &w, 10, 0.3);
        let cam = Camera::from_angles(0.0, 30.0);
        let f = t.frame([0.5, 66.0, 0.0], cam, &w, 600);
        assert!(!f.lines.is_empty() && f.quads.is_empty());
        t.settings.mode = Mode::Ribbon;
        let f = t.frame([0.5, 66.0, 0.0], cam, &w, 600);
        assert!(f.lines.is_empty() && !f.quads.is_empty());
    }

    #[test]
    fn quads_face_the_camera() {
        let w = Flat { top: 64.0 };
        let mut t = Trails::default();
        walk(&mut t, &w, 20, 0.2);
        for camera in [[0.5, 70.0, 0.0], [0.5, 60.0, 0.0]] {
            let f = t.frame(camera, Camera::from_angles(0.0, 0.0), &w, 900);
            assert!(!f.quads.is_empty());
            for q in &f.quads {
                let c = q
                    .corners
                    .map(|v| [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]);
                let n = cross(sub(c[1], c[0]), sub(c[2], c[1]));
                let center = mul(add(add(c[0], c[1]), add(c[2], c[3])), 0.25);
                assert!(dot(n, mul(center, -1.0)) >= 0.0);
            }
        }
    }

    #[test]
    fn camera_basis_matches_minecraft_yaw() {
        let c = Camera::from_angles(0.0, 0.0);
        assert!((c.right[0] + 1.0).abs() < 1e-9 && (c.up[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn settings_round_trip_and_click() {
        let mut s = Settings {
            mode: Mode::Ribbon,
            color: 0x80112233,
            ..Default::default()
        };
        s.set("lineWidth", "7.3").unwrap();
        let stored: HashMap<_, _> = s.stored().into_iter().collect();
        let back = Settings::read(|k| stored.get(k).cloned());
        assert_eq!(back, s);
        assert_eq!(back.get(WIDTH), 7.5);
        let mut t = Trails::default();
        let before = t.settings.mode;
        assert!(t.click(1));
        assert_ne!(t.settings.mode, before);
        assert_eq!(t.settings.labels().len(), t.settings.actions().len());
    }

    #[test]
    fn number_click_wraps() {
        let mut v = DENSITY.default;
        for _ in 0..20 {
            v = DENSITY.next(v);
            assert!((DENSITY.min..=DENSITY.max).contains(&v));
        }
        assert_eq!(DENSITY.next(DENSITY.max), DENSITY.min);
    }

    #[test]
    fn commands_parse() {
        let mut t = Trails::default();
        assert!(command(&mut t, "time set day").is_none());
        let (_, save) = command(&mut t, "trails mode ribbon").unwrap();
        assert!(save && t.settings.mode == Mode::Ribbon);
        command(&mut t, "trails set color #00ff00").unwrap();
        assert_eq!(t.settings.color, 0xFF00FF00);
        let (msg, save) = command(&mut t, "trails set bogus 1").unwrap();
        assert!(!save && msg[0].contains("unknown"));
    }

    #[test]
    fn tint_and_light_packing() {
        assert_eq!(tint(0xFFFF0000, 0.5, false, 0, [0.0; 3]) >> 24, 128);
        assert_eq!(pack_light((15, 15)), FULL_BRIGHT);
        assert_eq!(darken(0xFF808080, 0.5), 0xFF404040);
    }
}
