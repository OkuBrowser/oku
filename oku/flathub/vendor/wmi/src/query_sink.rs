use crate::{WMIConnection, WMIError, WMIResult, result_enumerator::IWbemClassWrapper};
use futures::Stream;
use log::trace;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};
use windows::Win32::Foundation::E_POINTER;
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemObjectSink, IWbemObjectSink_Impl, WBEM_STATUS_COMPLETE,
};
use windows::core::{BSTR, HRESULT, Ref, Result as WinResult, implement};

#[derive(Default)]
pub struct AsyncQueryResultStreamImpl {
    buf: VecDeque<WMIResult<IWbemClassWrapper>>,
    is_done: bool,
    waker: Option<Waker>,
}

/// We wrap the internal objects to ensure that the waker is correctly called when new data is available or when the query is done.
///
/// If the waker is still `None`, we know that `poll_next` has not been called yet.
/// We can fill the buffer, and once `poll_next` is called, it'll return `Poll::Ready` and there's no need to wake the stream manually.
///
/// Once the internal buffer is fully consumed (or empty to begin with) and `poll_next` is called, it'll set the waker and return `Poll::Pending`.
/// Because the waker is set, we can wake the stream.
impl AsyncQueryResultStreamImpl {
    pub fn extend(&mut self, iter: impl IntoIterator<Item = WMIResult<IWbemClassWrapper>>) {
        self.buf.extend(iter);

        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }

    pub fn set_done(&mut self) {
        self.is_done = true;

        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

/// A stream of WMI query results.
///
/// When dropped, the stream is properly cancelled and the resources freed.
pub struct AsyncQueryResultStream {
    inner: AsyncQueryResultStreamInner,
    connection: WMIConnection,
    sink: IWbemObjectSink,
}

impl AsyncQueryResultStream {
    pub fn new(
        inner: AsyncQueryResultStreamInner,
        connection: WMIConnection,
        sink: IWbemObjectSink,
    ) -> Self {
        Self {
            inner,
            connection,
            sink,
        }
    }
}

impl Drop for AsyncQueryResultStream {
    fn drop(&mut self) {
        let _r = unsafe { self.connection.svc.CancelAsyncCall(&self.sink) };
    }
}

/// We use a mutex to synchronize the consumer and the calls from the WMI-managed thread.
/// A blocking mutex is used because we want to be runtime agnostic
/// and because according to [`tokio::sync::Mutex`](https://docs.rs/tokio/tokio/tokio/sync/struct.Mutex.html):
/// > The primary use case for the async mutex is to provide shared mutable access to IO resources such as a database connection. If the value behind the mutex is just data, it’s usually appropriate to use a blocking mutex
#[derive(Default, Clone)]
pub struct AsyncQueryResultStreamInner(Arc<Mutex<AsyncQueryResultStreamImpl>>);

impl AsyncQueryResultStreamInner {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(AsyncQueryResultStreamImpl::default())))
    }

    fn extend(&self, iter: impl IntoIterator<Item = WMIResult<IWbemClassWrapper>>) {
        let mut lock = self.0.lock().unwrap();
        lock.extend(iter);
    }

    fn set_done(&self) {
        let mut lock = self.0.lock().unwrap();
        lock.set_done();
    }
}

impl Stream for AsyncQueryResultStream {
    type Item = WMIResult<IWbemClassWrapper>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let waker = cx.waker();
        let mut inner = self.inner.0.lock().unwrap();

        if !inner
            .waker
            .as_ref()
            .map(|current_waker| waker.will_wake(current_waker))
            .unwrap_or(false)
        {
            inner.waker.replace(waker.clone());
        }

        let next = inner.buf.pop_back();

        match next {
            Some(item) => {
                trace!("poll_next: item found");
                Poll::Ready(Some(item))
            }
            None => {
                if inner.is_done {
                    trace!("poll_next: done");
                    Poll::Ready(None)
                } else {
                    trace!("poll_next: item not found");
                    Poll::Pending
                }
            }
        }
    }
}

#[implement(IWbemObjectSink)]
pub struct QuerySink {
    pub stream: AsyncQueryResultStreamInner,
}

/// Implementation for [IWbemObjectSink](https://docs.microsoft.com/en-us/windows/win32/api/wbemcli/nn-wbemcli-iwbemobjectsink).
/// This [Sink](https://en.wikipedia.org/wiki/Sink_(computing))
/// receives asynchronously the result of the query, through Indicate calls.
/// When finished,the SetStatus method is called.
/// # <https://docs.microsoft.com/fr-fr/windows/win32/wmisdk/example--getting-wmi-data-from-the-local-computer-asynchronously>
impl IWbemObjectSink_Impl for QuerySink_Impl {
    fn Indicate(
        &self,
        lObjectCount: i32,
        apObjArray: *const Option<IWbemClassObject>,
    ) -> WinResult<()> {
        trace!("Indicate call with {} objects", lObjectCount);
        // Case of an incorrect or too restrictive query
        if lObjectCount <= 0 {
            return Ok(());
        }

        let lObjectCount = lObjectCount as usize;
        let mut res = Ok(());

        // Safety:
        //
        // The safety points are mainly guaranteed by the contract of the Indicate API.
        // `apObjArray` is an array pointer to `IWbemClassObject`, whose length is provided by
        // lObjectCount. Hence:
        // - `apObjArray` is is valid for lObjectCount * <ptr_size> reads. `IWbemClassObject` is
        //   a wrapper on a NonNull pointer. The Option makes it nullable, but it uses the right
        //   alignment and size.
        // - `apObjArray` points to lObjectCount consecutive pointers.
        // - the memory behind this pointer is not modified while the slice is alive
        let objs = unsafe { std::slice::from_raw_parts(apObjArray, lObjectCount) };
        self.stream.extend(objs.iter().map(|obj| match obj {
            Some(p_el) => Ok(IWbemClassWrapper::new(p_el.clone())),
            None => {
                res = Err(E_POINTER.into());
                Err(WMIError::NullPointerResult)
            }
        }));

        res
    }

    fn SetStatus(
        &self,
        lFlags: i32,
        _hResult: HRESULT,
        _strParam: &BSTR,
        _pObjParam: Ref<IWbemClassObject>,
    ) -> WinResult<()> {
        // SetStatus is called only once as flag=WBEM_FLAG_BIDIRECTIONAL in ExecQueryAsync
        // https://docs.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemobjectsink-setstatus
        // If you do not specify WBEM_FLAG_SEND_STATUS when calling your provider or service method,
        // you are guaranteed to receive one and only one call to SetStatus

        if lFlags == WBEM_STATUS_COMPLETE.0 {
            trace!("End of async result, closing transmitter");
            self.stream.set_done();
        }
        Ok(())
    }
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::*;
    use futures::StreamExt;
    use windows::core::{IUnknown, Interface};

    #[async_std::test]
    async fn async_it_should_send_result() {
        let con = wmi_con();
        let stream = AsyncQueryResultStreamInner::new();
        let sink = QuerySink {
            stream: stream.clone(),
        };
        let p_sink: IWbemObjectSink = sink.into();
        let mut stream = AsyncQueryResultStream::new(stream, con.clone(), p_sink.clone());

        let raw_os = con
            .get_object(r#"\\.\root\cimv2:Win32_OperatingSystem=@"#)
            .unwrap();
        let raw_os2 = con
            .get_object(r#"\\.\root\cimv2:Win32_OperatingSystem=@"#)
            .unwrap();

        // tests on ref count before Indicate call
        unsafe {
            let test_ptr: IUnknown = raw_os.inner.clone().cast().unwrap();
            let refcount = (test_ptr.vtable().AddRef)(std::mem::transmute_copy(&test_ptr));
            // 1 from p_sink + 1 from test_ptr + 1 from AddRef
            assert_eq!(refcount, 3);
            let refcount = (test_ptr.vtable().Release)(std::mem::transmute_copy(&test_ptr));
            // 1 from p_sink + 1 from test_ptr
            assert_eq!(refcount, 2);
        }

        unsafe {
            p_sink
                .Indicate(&[Some(raw_os.inner.clone()), Some(raw_os2.inner.clone())])
                .unwrap();
        }
        // tests on ref count after Indicate call
        unsafe {
            let test_ptr: IUnknown = raw_os.inner.clone().cast().unwrap();
            let refcount = (test_ptr.vtable().AddRef)(std::mem::transmute_copy(&test_ptr));
            // 1 from p_sink + 1 from test_ptr + 1 from AddRef + 1 from the Indicate call
            assert_eq!(refcount, 4);
            let refcount = (test_ptr.vtable().Release)(std::mem::transmute_copy(&test_ptr));
            assert_eq!(refcount, 3);
        }

        let first = stream.next().await.unwrap().unwrap();

        assert_eq!(first.class().unwrap().as_str(), "Win32_OperatingSystem");

        let second = stream.next().await.unwrap().unwrap();
        assert_eq!(second.class().unwrap().as_str(), "Win32_OperatingSystem");
    }

    #[async_std::test]
    async fn async_it_should_complete_after_set_status_call() {
        let con = wmi_con();
        let stream = AsyncQueryResultStreamInner::new();
        let sink = QuerySink {
            stream: stream.clone(),
        };
        let p_sink: IWbemObjectSink = sink.into();
        let stream = AsyncQueryResultStream::new(stream, con.clone(), p_sink.clone());

        unsafe {
            p_sink
                .SetStatus(WBEM_STATUS_COMPLETE.0, HRESULT(0), &BSTR::new(), None)
                .unwrap();
        }

        let results: Vec<_> = stream.collect().await;

        assert!(results.is_empty());
    }

    #[async_std::test]
    async fn async_it_should_return_e_pointer_after_indicate_call_with_null_pointer() {
        let con = wmi_con();
        let stream = AsyncQueryResultStreamInner::new();
        let sink = QuerySink {
            stream: stream.clone(),
        };
        let p_sink: IWbemObjectSink = sink.into();
        let mut stream = AsyncQueryResultStream::new(stream, con.clone(), p_sink.clone());

        let arr = vec![None];

        let result = unsafe { p_sink.Indicate(&arr) };
        assert_eq!(result.unwrap_err().code(), E_POINTER);

        let item = stream.next().await.unwrap();

        match item {
            Err(WMIError::NullPointerResult) => assert!(true),
            _ => assert!(false),
        }
    }

    #[async_std::test]
    async fn async_test_notification() {
        let con = wmi_con();
        let inner = AsyncQueryResultStreamInner::new();
        let sink = QuerySink {
            stream: inner.clone(),
        };
        let p_sink: IWbemObjectSink = sink.into();

        // Exec a notification to setup the sink properly
        let query_language = BSTR::from("WQL");
        let query = BSTR::from(
            "SELECT * FROM __InstanceModificationEvent \
             WHERE TargetInstance ISA 'Win32_LocalTime'",
        );

        unsafe {
            // As p_sink's RefCount = 1 before this call,
            // p_sink won't be dropped at the end of ExecNotificationQueryAsync
            con.svc
                .ExecNotificationQueryAsync(
                    &query_language,
                    &query,
                    Default::default(),
                    None,
                    &p_sink,
                )
                .unwrap()
        };

        // lets cheat by keeping the inner stream locally, before dropping the stream object,
        // which will cancel the notification
        let mut stream = AsyncQueryResultStream::new(inner.clone(), con, p_sink);

        let elem = stream.next().await;
        assert!(elem.is_some());

        assert_eq!(inner.0.lock().unwrap().is_done, false);
        // end the stream by dropping it
        drop(stream);

        // Check the "is_done" flag has been set as the SetStatus member was called.
        // This is not necessarily done on the same thread, wait a bit for the SetStatus function
        // to be called.
        for _ in 0..5 {
            if inner.0.lock().unwrap().is_done {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        assert_eq!(inner.0.lock().unwrap().is_done, true);
    }
}
