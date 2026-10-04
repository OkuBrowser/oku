use askama::Template;

#[derive(Template)]
#[template(path = "fuse_driver.rs.j2")]
pub struct FuseDriverTemplate<'a> {
    pub mode: &'a str,
}

#[derive(Template)]
#[template(path = "fuse_handler.rs.j2")]
pub struct FuseHandlerTemplate<'a> {
    pub mode: &'a str,
}

#[derive(Template)]
#[template(path = "mounting.rs.j2")]
pub struct MountingTemplate<'a> {
    #[allow(unused)]
    pub mode: &'a str,
}

#[derive(Template)]
#[template(path = "fuse_lib.rs.j2")]
pub struct FuseLibTemplate<'a> {
    pub mode: &'a str,
}
