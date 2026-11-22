use mysql::*;

#[derive(Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub db_name: String,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3306,
            user: "apotek".to_string(),
            pass: "apotek123".to_string(),
            db_name: "apotek_db".to_string(),
        }
    }
}

impl DbConfig {
    pub fn to_url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            self.user, self.pass, self.host, self.port, self.db_name
        )
    }
}

pub fn create_pool(config: &DbConfig) -> Result<Pool> {
    let opts = Opts::from_url(&config.to_url())?;
    let pool_opts = PoolOpts::default()
        .with_constraints(PoolConstraints::new(1, 5).unwrap()); // Hemat RAM pool
    let builder = OptsBuilder::from_opts(opts).pool_opts(pool_opts);
    Pool::new(builder)
}
