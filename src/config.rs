// trunk-ignore-all(trunk-toolbox/do-not-land,trunk-toolbox/todo)
use confique::toml::{self, FormatOptions};
use confique::Config;

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub ifchange: IfChangeConf,

    #[config(nested)]
    pub donotland: PlsNotLandConf,

    #[config(nested)]
    pub todo: TodoConf,

    #[config(nested)]
    pub neveredit: NeverEditConf,

    #[config(nested)]
    pub nocurlyquotes: NoCurlyQuotesConf,
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

#[derive(Config)]
pub struct TodoConf {
    #[config(default = false)]
    pub enabled: bool,
}

#[derive(Config)]
pub struct NeverEditConf {
    #[config(default = false)]
    pub enabled: bool,
    #[config(default = [])]
    pub paths: Vec<String>,
}

#[derive(Config)]
pub struct NoCurlyQuotesConf {
    #[config(default = false)]
    pub enabled: bool,
}
