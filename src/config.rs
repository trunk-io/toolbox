// trunk-ignore-all(trunk-toolbox/do-not-land)
use confique::Config;

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub ifchange: IfChangeConf,

    #[config(nested)]
    pub donotland: PlsNotLandConf,
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
