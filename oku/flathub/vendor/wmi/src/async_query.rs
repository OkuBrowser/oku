use crate::{
    WMIResult,
    connection::WMIConnection,
    query::{FilterValue, build_query},
    query_sink::{AsyncQueryResultStream, AsyncQueryResultStreamInner, QuerySink},
    result_enumerator::IWbemClassWrapper,
};
use futures::stream::{Stream, StreamExt, TryStreamExt};
use serde::de;
use std::collections::HashMap;
use windows::Win32::System::Wmi::{IWbemObjectSink, WBEM_FLAG_BIDIRECTIONAL};
use windows::core::BSTR;

///
/// ### Additional async methods
///
impl WMIConnection {
    /// Wrapper for the [ExecQueryAsync](https://docs.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemservices-execqueryasync)
    /// method. Provides safety checks, and returns results
    /// as a Stream instead of the original Sink.
    ///
    pub fn exec_query_async(
        &self,
        query: impl AsRef<str>,
    ) -> WMIResult<impl Stream<Item = WMIResult<IWbemClassWrapper>>> {
        let query_language = BSTR::from("WQL");
        let query = BSTR::from(query.as_ref());

        let stream = AsyncQueryResultStreamInner::new();
        // The internal RefCount has initial value = 1.
        let p_sink = QuerySink {
            stream: stream.clone(),
        };
        let p_sink_handle: IWbemObjectSink = p_sink.into();

        unsafe {
            // As p_sink's RefCount = 1 before this call,
            // p_sink won't be dropped at the end of ExecQueryAsync
            self.svc.ExecQueryAsync(
                &query_language,
                &query,
                WBEM_FLAG_BIDIRECTIONAL,
                &self.ctx.0,
                &p_sink_handle,
            )?;
        }

        Ok(AsyncQueryResultStream::new(
            stream,
            self.clone(),
            p_sink_handle,
        ))
    }

    /// Async version of [`raw_query`](WMIConnection#method.raw_query)
    /// Execute a free-text query and deserialize the results.
    /// Can be used either with a struct (like `query` and `filtered_query`),
    /// but also with a generic map.
    ///
    /// ```edition2018
    /// # use wmi::*;
    /// # use std::collections::HashMap;
    /// # use futures::executor::block_on;
    /// # fn main() -> WMIResult<()> {
    /// #   block_on(exec_async_query())?;
    /// #   Ok(())
    /// # }
    /// #
    /// # async fn exec_async_query() -> WMIResult<()> {
    /// # let con = WMIConnection::new()?;
    /// use futures::stream::TryStreamExt;
    /// let results: Vec<HashMap<String, Variant>> = con.async_raw_query("SELECT Name FROM Win32_OperatingSystem").await?;
    /// #   Ok(())
    /// # }
    /// ```
    pub async fn async_raw_query<T>(&self, query: impl AsRef<str>) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        self.exec_query_async(query)?
            .map(|item| match item {
                Ok(wbem_class_obj) => wbem_class_obj.into_desr(),
                Err(e) => Err(e),
            })
            .try_collect::<Vec<_>>()
            .await
    }

    /// Query all the objects of type T.
    ///
    /// ```edition2018
    /// # use wmi::*;
    /// # use std::collections::HashMap;
    /// # use futures::executor::block_on;
    /// # fn main() -> WMIResult<()> {
    /// #   block_on(exec_async_query())?;
    /// #   Ok(())
    /// # }
    /// #
    /// # async fn exec_async_query() -> WMIResult<()> {
    /// # let con = WMIConnection::new()?;
    /// use serde::Deserialize;
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_Process {
    ///     Name: String,
    /// }
    ///
    /// let procs: Vec<Win32_Process> = con.async_query().await?;
    /// #   Ok(())
    /// # }
    /// ```
    pub async fn async_query<T>(&self) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        let query_text = build_query::<T>(None)?;

        self.async_raw_query(&query_text).await
    }

    /// Query all the objects of type T, while filtering according to `filters`.
    ///
    pub async fn async_filtered_query<T>(
        &self,
        filters: &HashMap<String, FilterValue>,
    ) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        let query_text = build_query::<T>(Some(filters))?;

        self.async_raw_query(&query_text).await
    }
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[cfg(test)]
mod tests {
    use crate::{Variant, tests::fixtures::*};
    use futures::stream::{self, StreamExt};
    use serde::Deserialize;
    use std::collections::HashMap;

    #[async_std::test]
    async fn async_it_works_async() {
        let wmi_con = wmi_con();

        let result = wmi_con
            .exec_query_async("SELECT OSArchitecture FROM Win32_OperatingSystem")
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn async_it_works_async_tokio() {
        let wmi_con = wmi_con();

        let result = wmi_con
            .exec_query_async("SELECT OSArchitecture FROM Win32_OperatingSystem")
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(result.len(), 1);
    }

    #[async_std::test]
    async fn async_it_handles_invalid_query() {
        let wmi_con = wmi_con();

        let result = wmi_con
            .exec_query_async("invalid query")
            .unwrap()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(result.len(), 0);
    }

    #[async_std::test]
    async fn async_it_provides_raw_query_result() {
        let wmi_con = wmi_con();

        let results: Vec<HashMap<String, Variant>> = wmi_con
            .async_raw_query("SELECT * FROM Win32_GroupUser")
            .await
            .unwrap();

        for res in results {
            match res.get("GroupComponent") {
                Some(Variant::String(s)) => assert_ne!(s, ""),
                _ => assert!(false),
            }

            match res.get("PartComponent") {
                Some(Variant::String(s)) => assert_ne!(s, ""),
                _ => assert!(false),
            }
        }
    }

    #[tokio::test]
    async fn async_it_works_async_tokio_concurrent() {
        let wmi_con = wmi_con();

        // We want to actually consume a bunch of data from WMI.
        #[allow(unused)]
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Name: String,
            SerialNumber: String,
            OSArchitecture: String,
            BootDevice: String,
            MUILanguages: Vec<String>,
        }

        // Using buffer_unordered(1) will take 2 seconds instead of 0.2 seconds.
        let results: Vec<Win32_OperatingSystem> = stream::iter(0..150)
            .map(|_| async {
                let result: Vec<Win32_OperatingSystem> = wmi_con.async_query().await.unwrap();

                result.into_iter().next().unwrap()
            })
            .buffer_unordered(50)
            .collect()
            .await;

        assert_eq!(results.len(), 150);
    }
}
