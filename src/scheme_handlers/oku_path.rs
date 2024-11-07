use oku_fs::iroh::docs::AuthorId;
use std::{path::PathBuf, str::FromStr};

#[derive(PartialEq, Debug, Clone)]
pub enum OkuPath {
    Home,
    Me(Option<PathBuf>),
    Tag(String),
    Tags,
    User(AuthorId, Option<PathBuf>),
    ToggleFollow(AuthorId),
    ToggleBlock(AuthorId),
    Delete(PathBuf),
}

impl OkuPath {
    pub fn parse(path: impl AsRef<std::path::Path>) -> miette::Result<Self> {
        let url_components: Vec<_> = path
            .as_ref()
            .components()
            .map(|x| PathBuf::from(x.as_os_str()))
            .collect();
        let first_component = url_components
            .first()
            .map(|x| x.to_path_buf())
            .unwrap_or(PathBuf::from("home"));
        let second_component = url_components.get(1);
        let replica_path = second_component
            .map(|_x| path.as_ref().strip_prefix(first_component.clone()).ok())
            .flatten()
            .map(|x| x.to_path_buf());
        Ok(
            match first_component
                .as_os_str()
                .to_string_lossy()
                .to_string()
                .as_str()
            {
                "home" => OkuPath::Home,
                "tags" => OkuPath::Tags,
                "tag" => second_component
                    .map(|x| OkuPath::Tag(x.to_string_lossy().to_string()))
                    .unwrap_or(OkuPath::Tags),
                "me" => OkuPath::Me(replica_path),
                "follow" => OkuPath::ToggleFollow(
                    AuthorId::from_str(
                        second_component
                            .ok_or(miette::miette!("Missing author ID … "))?
                            .as_os_str()
                            .to_string_lossy()
                            .to_string()
                            .as_str(),
                    )
                    .map_err(|e| miette::miette!("{}", e))?,
                ),
                "block" => OkuPath::ToggleBlock(
                    AuthorId::from_str(
                        second_component
                            .ok_or(miette::miette!("Missing author ID … "))?
                            .as_os_str()
                            .to_string_lossy()
                            .to_string()
                            .as_str(),
                    )
                    .map_err(|e| miette::miette!("{}", e))?,
                ),
                "delete" => {
                    OkuPath::Delete(replica_path.ok_or(miette::miette!("Missing post path … "))?)
                }
                _ => OkuPath::User(
                    AuthorId::from_str(
                        first_component
                            .as_os_str()
                            .to_string_lossy()
                            .to_string()
                            .as_str(),
                    )
                    .map_err(|e| miette::miette!("{}", e))?,
                    replica_path,
                ),
            },
        )
    }
}
