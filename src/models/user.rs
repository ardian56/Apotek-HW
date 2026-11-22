use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub full_name: String,
    pub role: String, // "ADMIN", "KASIR", "GUDANG"
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role.to_uppercase() == "ADMIN"
    }

    pub fn can_access_reports(&self) -> bool {
        matches!(self.role.to_uppercase().as_str(), "ADMIN" | "GUDANG")
    }
}
