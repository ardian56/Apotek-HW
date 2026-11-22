use fltk::app::Scheme;
use fltk::enums::{Color, Font, FrameType};

pub struct AppTheme;

impl AppTheme {
    // Palet Warna Clean Modern (Tailwind Slate / Emerald)
    pub const PRIMARY: Color = Color::from_hex(0x0D9488); // Teal 600 (Warna Farmasi Modern)
    pub const PRIMARY_HOVER: Color = Color::from_hex(0x0F766E); // Teal 700
    pub const PRIMARY_SOFT: Color = Color::from_hex(0xE7F3F1);
    pub const BG_APP: Color = Color::from_hex(0xF8FAFC); // Slate 50 (Latar Bersih)
    pub const BORDER: Color = Color::from_hex(0xE2E8F0); // Slate 200 (Garis Halus)
    pub const TEXT_TITLE: Color = Color::from_hex(0x0F172A); // Slate 900 (Teks Judul Tegas)
    pub const TEXT_BODY: Color = Color::from_hex(0x334155); // Slate 700 (Teks Isi)
    pub const TEXT_MUTED: Color = Color::from_hex(0x64748B); // Slate 500 (Teks Keterangan)

    // Status Badges (Pastel Halus)
    pub const DANGER_TEXT: Color = Color::from_hex(0xBE123C); // Rose 700
    pub const TABLE_HEADER: Color = Color::from_hex(0xF1F5F9); // Slate 100

    pub fn apply() {
        fltk::app::set_scheme(Scheme::Base);
        for frame in [
            FrameType::BorderBox,
            FrameType::UpBox,
            FrameType::DownBox,
            FrameType::ThinUpBox,
            FrameType::ThinDownBox,
        ] {
            fltk::app::set_frame_type_cb(
                frame,
                |x, y, w, h, c| {
                    fltk::draw::draw_rbox(x, y, w, h, 5, true, c);
                    fltk::draw::draw_rbox(x, y, w, h, 5, false, Self::BORDER);
                },
                1,
                1,
                2,
                2,
            );
        }
        fltk::app::set_frame_type_cb(
            FrameType::BorderFrame,
            |x, y, w, h, _| {
                fltk::draw::draw_rbox(x, y, w, h, 5, false, Self::BORDER);
            },
            1,
            1,
            2,
            2,
        );
        fltk::app::set_frame_type_cb(
            FrameType::RFlatBox,
            |x, y, w, h, c| {
                fltk::draw::draw_rbox(x, y, w, h, 5, true, c);
            },
            1,
            1,
            2,
            2,
        );
        fltk::app::set_font(Font::Helvetica);
        fltk::app::set_visible_focus(true);
        fltk::app::background(248, 250, 252);
        fltk::app::background2(255, 255, 255);
        fltk::app::foreground(15, 23, 42);
    }
}
