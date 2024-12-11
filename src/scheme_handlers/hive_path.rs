use oku_fs::iroh_docs::{DocTicket, NamespaceId};
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Clone)]
pub enum HivePath {
    ByTicket(Box<DocTicket>, PathBuf),
    ById(NamespaceId, PathBuf),
}

impl HivePath {
    pub fn parse(path: impl AsRef<std::path::Path>) -> miette::Result<Self> {
        let url_components: Vec<_> = path
            .as_ref()
            .components()
            .map(|x| PathBuf::from(x.as_os_str()))
            .collect();
        let first_component = url_components.first().ok_or(miette::miette!(
            "{:?} does not contain a replica ID or ticket … ",
            path.as_ref()
        ))?;
        let second_component = url_components.get(1);
        let replica_path = second_component
            .and_then(|_x| path.as_ref().strip_prefix(first_component.clone()).ok())
            .map(|x| PathBuf::from("/").join(x))
            .unwrap_or("/".into());
        if let Ok(ticket) = DocTicket::from_str(&first_component.to_string_lossy()) {
            Ok(Self::ByTicket(Box::new(ticket), replica_path))
        } else if let Ok(namespace_id_bytes) = oku_fs::iroh_base::base32::parse_array_hex_or_base32::<
            32,
        >(&first_component.to_string_lossy())
        {
            Ok(Self::ById(
                NamespaceId::from(namespace_id_bytes),
                replica_path,
            ))
        } else {
            Err(miette::miette!(
                "{:?} does not contain a replica ID or ticket … ",
                path.as_ref()
            ))
        }
    }
}
