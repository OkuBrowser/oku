use crate::error::Error;
use crate::path::IpfsPath;
#[cfg(feature = "dns")]
use connexa::prelude::transport::dns::DnsResolver;

use tracing_futures::Instrument;

#[cfg(not(target_arch = "wasm32"))]
pub async fn resolve<'a>(
    resolver: DnsResolver,
    domain: &str,
    mut path: impl Iterator<Item = &'a str>,
) -> Result<IpfsPath, Error> {
    use hickory_resolver::Resolver;
    use hickory_resolver::net::runtime::TokioRuntimeProvider;
    use hickory_resolver::proto::rr::RData;
    use std::borrow::Cow;
    use std::str::FromStr;

    let span = tracing::trace_span!("dnslink", %domain);

    async move {
        // allow using non fqdn names (using the local search path suffices)
        let searched = Some(Cow::Borrowed(domain));

        let prefix = "_dnslink.";
        let prefixed = if !domain.starts_with(prefix) {
            let mut next = String::with_capacity(domain.len() + prefix.len());
            next.push_str(prefix);
            next.push_str(domain);
            Some(Cow::Owned(next))
        } else {
            None
        };

        let searched = searched.into_iter().chain(prefixed);

        // FIXME: this uses caching trust-dns resolver even though it's discarded right away
        // when trust-dns support lands in future libp2p-dns investigate if we could share one, no need
        // to have multiple related caches.
        let (config, opt) = resolver.into();
        let resolver = Resolver::builder_with_config(config, TokioRuntimeProvider::default())
            .with_options(opt)
            .build()?;

        // previous implementation searched $domain and _dnslink.$domain concurrently. not sure did
        // `domain` assume fqdn names or not, but local suffices were not being searched on windows at
        // least. they are probably waste of time most of the time.
        for domain in searched {
            let res = match resolver.txt_lookup(&*domain).await {
                Ok(res) => res,
                Err(e) => {
                    tracing::debug!("resolving dnslink of {:?} failed: {}", domain, e);
                    continue;
                }
            };

            let mut paths =
                res.answers()
                    .iter()
                    .filter_map(|record| match &record.data {
                        RData::TXT(txt) => Some(txt),
                        _ => None,
                    })
                    .flat_map(|txt| txt.txt_data.iter())
                    .filter_map(|txt| {
                        if txt.starts_with(b"dnslink=") {
                            Some(&txt[b"dnslink=".len()..])
                        } else {
                            None
                        }
                    })
                    .map(|suffix| {
                        std::str::from_utf8(suffix)
                            .map_err(Error::from)
                            .and_then(IpfsPath::from_str)
                            .and_then(|mut internal_path| {
                                internal_path.path.push_split(path.by_ref()).map_err(|_| {
                                    crate::path::IpfsPathError::InvalidPath("".into())
                                })?;
                                Ok(internal_path)
                            })
                    });

            if let Some(Ok(x)) = paths.next() {
                tracing::trace!("dnslink found for {:?}", domain);
                return Ok(x);
            }

            tracing::trace!("zero TXT records found for {:?}", domain);
        }

        Err(anyhow::anyhow!("failed to resolve {:?}", domain))
    }
    .instrument(span)
    .await
}

#[cfg(target_arch = "wasm32")]
const TXT_RECORD: u16 = 16;

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer", default)]
    answer: Vec<DohAnswer>,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}

#[cfg(target_arch = "wasm32")]
fn unquote(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
}

#[cfg(target_arch = "wasm32")]
pub async fn resolve<'a>(
    resolver: DnsResolver,
    domain: &str,
    path: impl Iterator<Item = &'a str>,
) -> Result<IpfsPath, Error> {
    use send_wrapper::SendWrapper;
    use std::str::FromStr;

    let span = tracing::trace_span!("dnslink", %domain);

    let endpoint = match resolver {
        DnsResolver::Google => "https://dns.google/resolve",
        DnsResolver::Cloudflare | DnsResolver::Local => "https://cloudflare-dns.com/dns-query",
        _ => {
            return Err(anyhow::anyhow!("no DNS resolver configured for {domain}"));
        }
    };

    let subpath = path.collect::<Vec<_>>();

    let prefixed = (!domain.starts_with("_dnslink.")).then(|| format!("_dnslink.{domain}"));
    let candidates = std::iter::once(domain.to_string()).chain(prefixed);

    SendWrapper::new(
        async move {
            for name in candidates {
                let url = format!("{endpoint}?name={name}&type=TXT");

                let resp = match gloo_net::http::Request::get(&url)
                    .header("Accept", "application/dns-json")
                    .send()
                    .await
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        tracing::debug!("resolving dnslink of {name:?} failed: {e}");
                        continue;
                    }
                };

                let doh = match resp.json::<DohResponse>().await {
                    Ok(doh) => doh,
                    Err(e) => {
                        tracing::debug!("decoding dnslink of {name:?} failed: {e}");
                        continue;
                    }
                };

                for answer in doh.answer {
                    if answer.kind != TXT_RECORD {
                        continue;
                    }

                    let Some(link) = unquote(&answer.data).strip_prefix("dnslink=") else {
                        continue;
                    };

                    let resolved = IpfsPath::from_str(link).and_then(|mut internal_path| {
                        internal_path
                            .path
                            .push_split(subpath.iter().copied())
                            .map_err(|_| {
                                crate::path::IpfsPathError::InvalidPath(link.to_string())
                            })?;
                        Ok(internal_path)
                    });

                    if let Ok(resolved) = resolved {
                        tracing::trace!("dnslink found for {name:?}");
                        return Ok(resolved);
                    }
                }

                tracing::trace!("zero dnslink TXT records found for {name:?}");
            }

            Err(anyhow::anyhow!("failed to resolve {domain:?}"))
        }
        .instrument(span),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use connexa::prelude::transport::dns::DnsResolver;

    #[tokio::test]
    async fn resolve_ipfs_io() {
        tracing_subscriber::fmt::init();
        let res = resolve(DnsResolver::Cloudflare, "ipfs.io", std::iter::empty())
            .await
            .unwrap()
            .to_string();
        assert_eq!(res, "/ipns/website.ipfs.io");
    }

    #[tokio::test]
    async fn resolve_website_ipfs_io() {
        let res = resolve(
            DnsResolver::Cloudflare,
            "website.ipfs.io",
            std::iter::empty(),
        )
        .await
        .unwrap();

        assert!(
            matches!(res.root(), crate::path::PathRoot::Ipld(_)),
            "expected an /ipfs/cid path"
        );
    }
}
