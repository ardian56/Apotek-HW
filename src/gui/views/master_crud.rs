use super::components::*;
use crate::{db, gui::theme::AppTheme as T};
use fltk::{enums::CallbackTrigger, input::InputType, prelude::*, window::Window};
use mysql::{params, prelude::*, Pool, Value};

pub struct MasterSimpleView {
    pub win: Window,
}

fn rows(pool: &Pool, table: &str, columns: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let sql = format!(
        "SELECT id,{} FROM {table} {} ORDER BY id DESC LIMIT 500",
        columns.join(","),
        if table == "users" {
            ""
        } else {
            "WHERE is_active=1"
        }
    );
    let data: Vec<mysql::Row> = pool
        .get_conn()
        .and_then(|mut c| c.query(sql))
        .map_err(|e| e.to_string())?;
    Ok(data
        .into_iter()
        .map(|r| {
            r.unwrap()
                .into_iter()
                .map(|v| match v {
                    Value::NULL => String::new(),
                    Value::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
                    Value::Int(n) => n.to_string(),
                    Value::UInt(n) => n.to_string(),
                    v => v.as_sql(true),
                })
                .collect()
        })
        .collect())
}

fn editor(
    pool: Pool,
    table: &'static str,
    columns: Vec<&'static str>,
    labels: Vec<&'static str>,
    existing: Option<Vec<String>>,
    mut updated: impl FnMut() + 'static,
) {
    let account = table == "users";
    let height = 144 + (labels.len() as i32 + if account { 1 } else { 0 }) * 76;
    let mut win = Window::default()
        .with_size(660, height)
        .with_label(if existing.is_some() {
            "Edit data"
        } else {
            "Tambah data"
        });
    win.set_color(T::BG_APP);
    let mut inputs = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        inputs.push(input(
            24,
            20 + i as i32 * 76,
            612,
            label,
            existing
                .as_ref()
                .and_then(|r| r.get(i + 1))
                .map(String::as_str)
                .unwrap_or(if columns[i] == "is_active" {
                    "1"
                } else if columns[i] == "role" {
                    "KASIR"
                } else {
                    ""
                }),
        ));
    }
    if account {
        let mut password = input(
            24,
            20 + labels.len() as i32 * 76,
            612,
            if existing.is_some() {
                "Password baru (kosong berarti tetap)"
            } else {
                "Password (minimal 8 karakter)"
            },
            "",
        );
        password.set_type(InputType::Secret);
        inputs.push(password);
    }
    let mut status = text(24, height - 102, 612, 28, "", 12, false);
    let mut save = button(24, height - 56, 200, "Simpan", true);
    let mut cancel = button(240, height - 56, 120, "Batal", false);
    let mut w = win.clone();
    cancel.set_callback(move |_| w.hide());
    let mut w = win.clone();
    save.set_callback(move |_|{
        let result=(||->Result<(),String>{
            let values=inputs.iter().take(columns.len()).map(|i|i.value().trim().to_string()).collect::<Vec<_>>();
            if values.first().map(|s|s.is_empty()).unwrap_or(true){return Err("Nama / username wajib diisi".into());}
            if account{
                if values[1].is_empty()||!matches!(values[2].as_str(),"ADMIN"|"KASIR"|"GUDANG")||!matches!(values[3].as_str(),"0"|"1"){return Err("Isi nama, role ADMIN/KASIR/GUDANG, dan status 0/1".into());}
            }
            let mut conn=pool.get_conn().map_err(|e|e.to_string())?;
            let mut tx=conn.start_transaction(mysql::TxOpts::default()).map_err(|e|e.to_string())?;
            let mut columns=columns.clone();let mut params=values.iter().map(|v|Value::from(v.as_str())).collect::<Vec<_>>();
            if account{
                let password=inputs.last().unwrap().value();
                if existing.is_none()||!password.is_empty(){if !(8..=72).contains(&password.len()){return Err("Password harus 8–72 byte".into());}columns.push("password_hash");params.push(Value::from(bcrypt::hash(password,12).map_err(|e|e.to_string())?));}
                if let Some(row)=&existing{
                    if row[3]=="ADMIN"&&(values[2]!="ADMIN"||values[3]!="1"){
                        let admins:Vec<i32>=tx.query("SELECT id FROM users WHERE role='ADMIN' AND is_active=1 FOR UPDATE").map_err(|e|e.to_string())?;
                        if admins.len()<=1{return Err("Minimal satu admin harus tetap aktif".into());}
                    }
                }
            }
            let sql=if let Some(row)=&existing{params.push(Value::from(row[0].as_str()));format!("UPDATE {table} SET {} WHERE id=?",columns.iter().map(|c|format!("{c}=?")).collect::<Vec<_>>().join(","))}
                else{format!("INSERT INTO {table} ({}) VALUES ({})",columns.join(","),vec!["?";columns.len()].join(","))};
            tx.exec_drop(sql,params).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())
        })();match result{Ok(())=>{updated();w.hide();},Err(e)=>status.set_label(&format!("Simpan gagal: {e}"))}
    });
    win.end();
    win.make_modal(true);
    win.show();
}

impl MasterSimpleView {
    fn build(
        pool: Option<Pool>,
        table: &'static str,
        title: &str,
        columns: Vec<&'static str>,
        labels: Vec<&'static str>,
    ) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        text(28, 20, 900, 32, title, 22, true);
        let mut search = input(28, 70, 540, "Cari nama / username", "");
        search.set_trigger(CallbackTrigger::EnterKey);
        let mut refresh = button(588, 94, 148, "Tampilkan", true);
        let mut add = button(756, 94, 180, "Tambah", true);
        let mut edit = button(952, 94, 212, "Edit data terpilih", false);
        let mut headers = vec!["ID"];
        headers.extend(labels.clone());
        let mut grid = Grid::new(28, 170, 1140, 352, &headers);
        let mut status = text(28, 132, 1140, 26, "", 12, false);
        let mut deactivate = button(28, 552, 200, "Nonaktifkan", false);
        if pool.is_none() {
            add.deactivate();
            edit.deactivate();
            deactivate.deactivate();
        }
        let update = {
            let pool = pool.clone();
            let columns = columns.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            let s = search.clone();
            move || {
                let Some(p) = &pool else {
                    g.set(vec![]);
                    st.set_label("Pratinjau • Hubungkan database untuk mengelola master data");
                    return;
                };
                match rows(p, table, &columns) {
                    Ok(data) => {
                        let query = s.value().to_lowercase();
                        let data = data
                            .into_iter()
                            .filter(|r| r.iter().any(|s| s.to_lowercase().contains(&query)))
                            .collect::<Vec<_>>();
                        st.set_label(&format!("{} baris • Maksimum 500 baris", data.len()));
                        g.set(data);
                    }
                    Err(e) => st.set_label(&format!("Data gagal dimuat: {e}")),
                }
            }
        };
        let mut u = update.clone();
        refresh.set_callback(move |_| u());
        let mut u = update.clone();
        search.set_callback(move |_| u());
        let p = pool.clone();
        let cols = columns.clone();
        let lbl = labels.clone();
        let u = update.clone();
        add.set_callback(move |_| {
            if let Some(p) = &p {
                editor(p.clone(), table, cols.clone(), lbl.clone(), None, u.clone());
            }
        });
        let p = pool.clone();
        let cols = columns.clone();
        let lbl = labels.clone();
        let u = update.clone();
        let g = grid.clone();
        let mut st = status.clone();
        edit.set_callback(move |_| {
            if let Some(p) = &p {
                if let Some(row) = g.selected() {
                    editor(
                        p.clone(),
                        table,
                        cols.clone(),
                        lbl.clone(),
                        Some(row),
                        u.clone(),
                    );
                } else {
                    st.set_label("Pilih satu baris pada tabel");
                }
            }
        });
        let p = pool.clone();
        let g = grid.clone();
        let mut st = status.clone();
        let mut u = update.clone();
        deactivate.set_callback(move |_| {
            let Some(p) = &p else {
                return;
            };
            let Some(row) = g.selected() else {
                st.set_label("Pilih satu baris pada tabel");
                return;
            };
            if table == "users" && row[3] == "ADMIN" {
                st.set_label(
                    "Ubah status admin melalui form edit agar minimal satu admin tetap aktif",
                );
                return;
            }
            if fltk::dialog::choice2_default(
                &format!("Nonaktifkan {}? Riwayat transaksi tetap tersimpan.", row[1]),
                "Batal",
                "Nonaktifkan",
                "",
            ) != Some(1)
            {
                return;
            }
            match p.get_conn().and_then(|mut c| {
                c.exec_drop(
                    format!("UPDATE {table} SET is_active=0 WHERE id=:id"),
                    params! {"id"=>&row[0]},
                )
            }) {
                Ok(()) => u(),
                Err(e) => st.set_label(&format!("Gagal menonaktifkan: {e}")),
            }
        });
        let mut u = update.clone();
        win.handle(move |_, event| {
            if event == fltk::enums::Event::Show {
                u();
            }
            false
        });
        let mut u = update;
        u();
        win.end();
        Self { win }
    }
    pub fn suplier(pool: Option<Pool>) -> Self {
        Self::build(
            pool,
            "suppliers",
            "Suplier",
            vec!["name", "phone", "address"],
            vec!["Nama suplier", "Telepon", "Alamat"],
        )
    }
    pub fn pelanggan(pool: Option<Pool>) -> Self {
        Self::build(
            pool,
            "customers",
            "Pelanggan / pasien",
            vec!["name", "phone", "address"],
            vec!["Nama pelanggan", "Telepon", "Alamat"],
        )
    }
    pub fn dokter(pool: Option<Pool>) -> Self {
        Self::build(
            pool,
            "doctors",
            "Dokter rekanan",
            vec!["name", "sip_no"],
            vec!["Nama dokter", "Nomor SIP"],
        )
    }
    pub fn karyawan(pool: Option<Pool>) -> Self {
        Self::build(
            pool,
            "users",
            "Operator dan hak akses",
            vec!["username", "full_name", "role", "is_active"],
            vec![
                "Username",
                "Nama lengkap",
                "Role (ADMIN / KASIR / GUDANG)",
                "Aktif (1 ya, 0 tidak)",
            ],
        )
    }
}
