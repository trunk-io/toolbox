use confique::Config;

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub ifchange: IfChangeConf,

    #[config(nested)]
    pub plsnoland: PlsNoLandConf,
}

#[derive(Config)]
pub struct IfChangeConf {
    #[config(default = true)]
    pub enabled: bool,
}

#[derive(Config)]
pub struct PlsNoLandConf {
    #[config(default = true)]
    pub enabled: bool,
}
