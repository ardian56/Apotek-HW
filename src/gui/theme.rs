use fltk::enums::{Color, Font};

pub struct AppTheme;

impl AppTheme {
    // Palet Warna Flat Modern
    pub const PRIMARY: Color = Color::from_hex(0x1E88E5);      // Biru Apotek
    pub const PRIMARY_DARK: Color = Color::from_hex(0x1565C0);
    pub const ACCENT: Color = Color::from_hex(0x00897B);        // Hijau Teal
    pub const BG_APP: Color = Color::from_hex(0xF4F6F9);        // Abu-abu terang bersih
    pub const BG_CARD: Color = Color::from_hex(0xFFFFFF);       // Putih bersih
    pub const BORDER: Color = Color::from_hex(0xCFD8DC);        // Garis batas halus
    pub const TEXT_MAIN: Color = Color::from_hex(0x263238);     // Teks gelap tegas
    pub const TEXT_MUTED: Color = Color::from_hex(0x78909C);    // Teks abu keterangan
    pub const SUCCESS: Color = Color::from_hex(0x43A047);       // Hijau sukses
    pub const WARNING_BG: Color = Color::from_hex(0xFFF9C4);    // Kuning highlight stok tipis
    pub const DANGER_BG: Color = Color::from_hex(0xFFCDD2);     // Merah highlight expired / habis
    pub const DANGER_TEXT: Color = Color::from_hex(0xC62828);

    pub fn apply() {
        fltk::app::set_font(Font::Helvetica);
        fltk::app::set_visible_focus(false);
        fltk::app::background(244, 246, 249);
        fltk::app::background2(255, 255, 255);
        fltk::app::foreground(38, 50, 56);
    }
}
