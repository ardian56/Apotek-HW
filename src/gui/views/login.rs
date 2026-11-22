use crate::gui::theme::AppTheme;
use fltk::button::Button;
use fltk::enums::{Align, Color, FrameType};
use fltk::frame::Frame;
use fltk::input::{Input, SecretInput};
use fltk::prelude::*;
use fltk::window::Window;

pub struct LoginView {
    pub win: Window,
    pub input_user: Input,
    pub input_pass: SecretInput,
    pub btn_login: Button,
    pub lbl_status: Frame,
}

impl LoginView {
    pub fn new() -> Self {
        let mut win = Window::default()
            .with_size(420, 360)
            .with_label("Login - POS Apotek Sehat");
        win.set_color(AppTheme::BG_APP);

        let mut card = Frame::new(30, 30, 360, 300, "");
        card.set_frame(FrameType::RFlatBox);
        card.set_color(AppTheme::BG_CARD);

        // Header Judul
        let mut title = Frame::new(50, 50, 320, 40, "APOTEK SEHAT");
        title.set_label_size(20);
        title.set_label_color(AppTheme::PRIMARY);
        title.set_label_font(fltk::enums::Font::HelveticaBold);

        let mut subtitle = Frame::new(50, 85, 320, 20, "Silakan login untuk memulai transaksi");
        subtitle.set_label_size(12);
        subtitle.set_label_color(AppTheme::TEXT_MUTED);

        // Input Username
        let mut lbl_u = Frame::new(50, 115, 320, 20, "Nama Pengguna:");
        lbl_u.set_align(Align::Left | Align::Inside);
        lbl_u.set_label_size(12);
        lbl_u.set_label_color(AppTheme::TEXT_MAIN);

        let mut input_user = Input::new(50, 138, 320, 34, "");
        input_user.set_value("admin");
        input_user.set_text_size(14);
        input_user.set_color(Color::White);
        input_user.set_frame(FrameType::FlatBox);

        // Input Password
        let mut lbl_p = Frame::new(50, 180, 320, 20, "Kata Sandi:");
        lbl_p.set_align(Align::Left | Align::Inside);
        lbl_p.set_label_size(12);
        lbl_p.set_label_color(AppTheme::TEXT_MAIN);

        let mut input_pass = SecretInput::new(50, 203, 320, 34, "");
        input_pass.set_value("admin123");
        input_pass.set_text_size(14);
        input_pass.set_color(Color::White);
        input_pass.set_frame(FrameType::FlatBox);

        // Status Label (Warning/Error)
        let mut lbl_status = Frame::new(50, 242, 320, 20, "");
        lbl_status.set_label_size(12);
        lbl_status.set_label_color(AppTheme::DANGER_TEXT);

        // Tombol Login
        let mut btn_login = Button::new(50, 270, 320, 40, "MASUK KE SISTEM");
        btn_login.set_color(AppTheme::PRIMARY);
        btn_login.set_label_color(Color::White);
        btn_login.set_label_font(fltk::enums::Font::HelveticaBold);
        btn_login.set_frame(FrameType::FlatBox);

        win.end();
        win.make_resizable(false);

        Self {
            win,
            input_user,
            input_pass,
            btn_login,
            lbl_status,
        }
    }
}
