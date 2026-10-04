use crate::ModelAttributes;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_native_model_decode_upgrade_body(attrs: &ModelAttributes) -> TokenStream {
    let native_model_from = attrs.from.clone();
    let native_model_try_from = attrs.try_from.clone();

    let model_from_or_try_from = if let Some(from) = native_model_from {
        quote! {
            #from::native_model_decode_upgrade_body(data, id, version).map(|a| a.into())
        }
    } else if let Some((try_from, error_try_from)) = native_model_try_from {
        quote! {
            let result = #try_from::native_model_decode_upgrade_body(data, id, version).map(|b| {
                b.try_into()
                    .map_err(|e: #error_try_from| native_model::UpgradeError {
                        msg: format!("{}", e),
                        source: e.into(),
                    })
            })??;
            Ok(result)
        }
    } else {
        quote! {
            Err(native_model::Error::UpgradeNotSupported {
                from: version,
                to: Self::native_model_version(),
            })
        }
    };

    let gen = quote! {
        fn native_model_decode_upgrade_body(data: Vec<u8>, id: u32, version: u32) -> native_model::Result<Self> {
            if version == Self::native_model_version() {
                let result = Self::native_model_decode_body(data, id)?;
                Ok(result)
            } else if version < Self::native_model_version() {
                #model_from_or_try_from
            } else {
                Err(native_model::Error::UpgradeNotSupported {
                    from: version,
                    to: Self::native_model_version(),
                })
            }
        }
    };

    gen
}
