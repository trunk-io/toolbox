// trunk-ignore-all(trunk-toolbox/do-not-land)
use confique::toml::{self, FormatOptions};
use confique::Config;

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub ifchange: IfChangeConf,

    #[config(nested)]
    pub donotland: PlsNotLandConf,
}

impl Conf {
    pub fn print_default() {
        let default_config = toml::template::<Conf>(FormatOptions::default());
        println!("{}", default_config);
    }
}

#[derive(Config)]
pub struct IfChangeConf {
    #[config(default = true)]
    pub enabled: bool,
}

#[derive(Config)]
pub struct PlsNotLandConf {
    #[config(default = true)]
    pub enabled: bool,
}
