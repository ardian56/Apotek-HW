use super::components::*;
use crate::gui::theme::AppTheme as T;
use fltk::{
    button::Button,
    enums::{Event, FrameType, Key},
    frame::Frame,
    input::{Input, SecretInput},
    prelude::*,
    window::Window,
};

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
            .with_size(420, 372)
            .with_label("Masuk · Apotek POS");
        win.set_color(T::BG_APP);
        panel(24, 24, 372, 324);
        let mut title = text(48, 44, 324, 32, "Apotek POS", 22, true);
        title.set_label_color(T::PRIMARY);
        text(
            48,
            82,
            324,
            20,
            "Masuk menggunakan akun operator Anda.",
            12,
            false,
        );
        let input_user = input(48, 120, 324, "Username", "admin");
        text(48, 190, 324, 18, "Password", 12, false);
        let mut input_pass = SecretInput::new(48, 214, 324, 34, None);
        input_pass.set_frame(FrameType::BorderBox);
        input_pass.set_color(fltk::enums::Color::White);
        input_pass.set_text_size(14);
        let mut lbl_status = text(48, 258, 324, 20, "", 12, false);
        lbl_status.set_label_color(T::DANGER_TEXT);
        let btn_login = button(48, 292, 324, "Masuk", true);
        win.end();
        let mut login = btn_login.clone();
        win.handle(move |_, event| {
            if event == Event::KeyDown && fltk::app::event_key() == Key::Enter {
                login.do_callback();
                return true;
            }
            false
        });
        Self {
            win,
            input_user,
            input_pass,
            btn_login,
            lbl_status,
        }
    }
}
