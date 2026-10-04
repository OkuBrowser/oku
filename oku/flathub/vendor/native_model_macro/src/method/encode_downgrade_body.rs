use crate::ModelAttributes;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_native_model_encode_downgrade_body(attrs: &ModelAttributes) -> TokenStream {
    let native_model_from = attrs.from.clone();
    let native_model_try_from = attrs.try_from.clone();

    let model_from_or_try_from = if let Some(from) = native_model_from {
        quote! {
            #from::native_model_encode_downgrade_body(self.into(), version)
        }
    } else if let Some((try_from, error_try_from)) = native_model_try_from {
        quote! {
            let result = #try_from::native_model_encode_downgrade_body(
                self.try_into()
                    .map_err(|e: #error_try_from| native_model::DowngradeError {
                        msg: format!("{}", e),
                        source: e.into(),
                    })?,
                version,
            )?;
            Ok(result)
        }
    } else {
        quote! {
            Err(native_model::Error::DowngradeNotSupported {
                from: version,
                to: Self::native_model_version(),
            })
        }
    };

    let gen = quote! {
        fn native_model_encode_downgrade_body(self, version: u32) -> native_model::Result<Vec<u8>> {
            if version == Self::native_model_version() {
                let result = self.native_model_encode_body()?;
                Ok(result)
            } else if version < Self::native_model_version() {
                #model_from_or_try_from
            } else {
                Err(native_model::Error::DowngradeNotSupported {
                    from: version,
                    to: Self::native_model_version(),
                })
            }
        }
    };

    gen
}

// #[error("Wrong type id expected: {}, actual: {}", expected, actual)]
// WrongTypeId { expected: u32, actual: u32 },
