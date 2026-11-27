use super::components::*;
use crate::gui::theme::AppTheme as T;
use fltk::{
    button::Button,
    enums::{Align, Event, FrameType, Key},
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
    pub fn new(pool: mysql::Pool) -> Self {
        let name =
            crate::db::operations::pharmacy_name(Some(&pool)).unwrap_or_else(|_| "Apotek".into());
        let mut win = Window::default()
            .with_size(420, 404)
            .with_label(&format!("Masuk · {name}"));
        win.set_color(T::BG_APP);
        panel(24, 24, 372, 356);
        let mut title = text(48, 44, 324, 56, &name, 22, true);
        title.set_align(Align::Left | Align::Inside | Align::Wrap);
        title.set_tooltip(&name);
        title.set_label_color(T::PRIMARY);
        text(
            48,
            108,
            324,
            20,
            "Masuk menggunakan akun operator Anda.",
            12,
            false,
        );
        let input_user = input(48, 146, 324, "Username", "admin");
        text(48, 218, 324, 18, "Password", 12, false);
        let mut input_pass = SecretInput::new(48, 242, 324, 34, None);
        input_pass.set_frame(FrameType::BorderBox);
        input_pass.set_color(fltk::enums::Color::White);
        input_pass.set_text_size(14);
        let mut lbl_status = text(48, 286, 324, 20, "", 12, false);
        lbl_status.set_label_color(T::DANGER_TEXT);
        let btn_login = button(48, 320, 324, "Masuk", true);
        win.end();
        win.set_callback(close_application_window);
        let mut login = btn_login.clone();
        let mut status = lbl_status.clone();
        let username = input_user.clone();
        let mut password = input_pass.clone();
        win.handle(move |window, event| {
            if event == Event::Show {
                password.set_value("");
                status.set_label("");
                match crate::db::operations::pharmacy_name(Some(&pool)) {
                    Ok(name) => {
                        title.set_label(&name);
                        title.set_tooltip(&name);
                        window.set_label(&format!("Masuk · {name}"));
                    }
                    Err(error) => status.set_label(&format!("Nama apotek gagal dimuat: {error}")),
                }
                let mut username = username.clone();
                fltk::app::add_timeout3(0.0, move |_| {
                    if username.visible_r() && fltk::app::modal().is_none() {
                        focus_input(&mut username);
                    }
                });
            }
            if window.visible_r()
                && fltk::app::modal().is_none()
                && matches!(event, Event::KeyDown | Event::Shortcut)
            {
                match fltk::app::event_key() {
                    Key::Escape => return true,
                    Key::Enter | Key::KPEnter => {
                        if fltk::app::focus().is_some_and(|widget| {
                            widget.as_widget_ptr() == username.as_widget_ptr()
                        }) {
                            focus_input(&mut password);
                        } else {
                            login.do_callback();
                        }
                        return true;
                    }
                    _ => (),
                }
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
