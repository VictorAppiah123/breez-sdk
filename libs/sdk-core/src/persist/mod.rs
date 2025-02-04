pub(crate) mod cache;
pub(crate) mod channels;
pub(crate) mod db;
pub(crate) mod migrations;
pub(crate) mod settings;
pub(crate) mod swap;
pub(crate) mod transactions;

#[cfg(test)]
mod test_utils {
    use std::fs;

    use rand::Rng;

    pub fn create_test_sql_dir() -> String {
        let mut tmp_file = std::env::temp_dir();
        let mut rng = rand::thread_rng();
        tmp_file.push(rng.gen::<u32>().to_string());
        let path = tmp_file.as_path();
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
        fs::create_dir_all(&tmp_file).unwrap();
        let s = path.to_str().unwrap();
        String::from(s)
    }
}
