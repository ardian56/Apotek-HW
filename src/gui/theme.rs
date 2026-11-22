use fltk::app::Scheme;
use fltk::enums::{Color, Font};

pub struct AppTheme;

impl AppTheme {
    // Palet Warna Clean Modern (Tailwind Slate / Emerald)
    pub const HEADER_BG: Color = Color::from_hex(0xFFFFFF);
    pub const HEADER_ACTIVE: Color = Color::from_hex(0xE2E8F0);
    pub const PRIMARY: Color = Color::from_hex(0x0D9488); // Teal 600 (Warna Farmasi Modern)
    pub const PRIMARY_HOVER: Color = Color::from_hex(0x0F766E); // Teal 700
    pub const BG_APP: Color = Color::from_hex(0xF8FAFC); // Slate 50 (Latar Bersih)
    pub const CARD_BG: Color = Color::from_hex(0xFFFFFF); // Pure White
    pub const BG_CARD: Color = Color::from_hex(0xFFFFFF); // Alias Pure White
    pub const BORDER: Color = Color::from_hex(0xE2E8F0); // Slate 200 (Garis Halus)
    pub const TEXT_TITLE: Color = Color::from_hex(0x0F172A); // Slate 900 (Teks Judul Tegas)
    pub const TEXT_MAIN: Color = Color::from_hex(0x0F172A); // Alias Slate 900
    pub const TEXT_BODY: Color = Color::from_hex(0x334155); // Slate 700 (Teks Isi)
    pub const TEXT_MUTED: Color = Color::from_hex(0x64748B); // Slate 500 (Teks Keterangan)

    // Status Badges (Pastel Halus)
    pub const SUCCESS: Color = Color::from_hex(0x10B981); // Emerald 500
    pub const WARNING: Color = Color::from_hex(0xF59E0B); // Amber 500
    pub const BADGE_GREEN_BG: Color = Color::from_hex(0xDCFCE7); // Emerald 100
    pub const BADGE_GREEN_TEXT: Color = Color::from_hex(0x15803D); // Emerald 700
    pub const BADGE_AMBER_BG: Color = Color::from_hex(0xFEF3C7); // Amber 100
    pub const BADGE_AMBER_TEXT: Color = Color::from_hex(0xB45309); // Amber 700
    pub const BADGE_RED_BG: Color = Color::from_hex(0xFEE2E2); // Rose 100
    pub const BADGE_RED_TEXT: Color = Color::from_hex(0xBE123C); // Rose 700
    pub const WARNING_BG: Color = Color::from_hex(0xFEF3C7); // Amber 100 (Highlight Lembut)
    pub const WARNING_TEXT: Color = Color::from_hex(0xB45309); // Amber 700
    pub const DANGER_BG: Color = Color::from_hex(0xFEE2E2); // Rose 100 (Highlight Kritis Lembut)
    pub const DANGER_TEXT: Color = Color::from_hex(0xBE123C); // Rose 700
    pub const TABLE_HEADER: Color = Color::from_hex(0xF1F5F9); // Slate 100

    pub fn apply() {
        fltk::app::set_scheme(Scheme::Gtk); // Menghilangkan bevel 3D Windows 95 jadul!
        fltk::app::set_font(Font::Helvetica);
        fltk::app::set_visible_focus(true);
        fltk::app::background(248, 250, 252);
        fltk::app::background2(255, 255, 255);
        fltk::app::foreground(15, 23, 42);
    }
}
