use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dump_path: Option<String>,
}
