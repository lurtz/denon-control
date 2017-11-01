#![allow(dead_code)]

mod avahi {
    use avahi_sys;
    pub use avahi_error::AvahiError;
    use libc::{c_void, c_int, c_char, timeval};
    use std::{ffi, time, ptr};
    use std::sync::mpsc::Sender;
    use std::sync::Mutex;
    use std::rc::Rc;
    use std::cell::Cell;
    use std::time::{Duration, SystemTime};
    use std::mem::transmute;

    type ClientState = avahi_sys::AvahiClientState;
    type ServiceResolver = avahi_sys::AvahiServiceResolver;
    type IfIndex = avahi_sys::AvahiIfIndex;
    type Protocol = avahi_sys::AvahiProtocol;
    type LookupFlags = avahi_sys::AvahiLookupFlags;
    type ResolverEvent = avahi_sys::AvahiResolverEvent;
    type Address = avahi_sys::AvahiAddress;
    type StringList = avahi_sys::AvahiStringList;
    type BrowserEvent = avahi_sys::AvahiBrowserEvent;
    type LookupResultFlags = avahi_sys::AvahiLookupResultFlags;
    type ServiceBrowserMessage = (IfIndex, Protocol, String, String, String);

    macro_rules! callback_types {
        ($name:ident, [$($module:path),*], $function:ty, $ccallback:ty, $callback_fn:path) => {pub mod $name {
            $(use $module;)*

            pub type CallbackFn = $function;
            pub type CallbackBoxed = Box<CallbackFn>;
            pub type CallbackBoxed2 = ::std::rc::Rc<CallbackBoxed>;
            pub type Callback = Option<CallbackBoxed2>;
            pub type CCallback = $ccallback;

            pub const CALLBACK_FN: CCallback = Some($callback_fn);

            pub fn get_callback_with_data(user_callback: &Callback) -> (CCallback, *mut ::libc::c_void) {
                let callback;
                let userdata;
                if let Some(ref cb_box) = *user_callback {
                    callback = CALLBACK_FN;
                    userdata = &(*(*cb_box)) as * const CallbackBoxed as * mut CallbackBoxed as * mut ::libc::c_void;
                } else {
                    callback = None;
                    userdata = ::std::ptr::null_mut();
                }

                return (callback, userdata);
            }
        }}
    }

    pub fn time_in_millis(timer: &time::Instant) -> u64 {
        let elapsed = timer.elapsed();
        let el_s = elapsed.as_secs() * 1000;
        let el_n = (elapsed.subsec_nanos() / 1_000_000) as u64;
        el_s + el_n
    }

    fn time_in(dur: Duration) -> Result<Duration, AvahiError> {
        let now = SystemTime::now();
        let time_since_epoch = now.duration_since(time::UNIX_EPOCH)?;
        Ok(dur + time_since_epoch)
    }

    fn create_timeval(time: Duration) -> timeval {
        let seconds = time.as_secs();
        let useconds = time.subsec_nanos() / 1000;
        timeval{tv_sec: seconds as i64, tv_usec: useconds as i64}
    }

    unsafe fn init_if_not_null(cstr: * const c_char) -> String {
        if ptr::null() != cstr {
            ffi::CStr::from_ptr(cstr).to_string_lossy().into_owned()
        } else {
            String::from("")
        }
    }

    pub struct Poller {
        poller : Cell<* mut avahi_sys::AvahiSimplePoll>,
    }

    impl Poller {
        pub fn new() -> Result<Poller, AvahiError> {
            unsafe {
                let poller = avahi_sys::avahi_simple_poll_new();
                if ptr::null() != poller {
                    return Ok(Poller{poller: Cell::new(poller)});
                } else {
                    return Err(AvahiError::PollerNew);
                }
            }
        }

        unsafe fn get(&self) -> * const avahi_sys::AvahiPoll {
            avahi_sys::avahi_simple_poll_get(self.poller.get())
        }

        pub fn looop(&self) -> i32 {
            unsafe {
                avahi_sys::avahi_simple_poll_loop(self.poller.get())
            }
        }

        pub fn quit(&self) {
            unsafe {
                avahi_sys::avahi_simple_poll_quit(self.poller.get())
            }
        }

        pub fn timeout(&self, dur: Duration) -> Result<Timeout, AvahiError> {
            let target = time_in(dur)?;
            let tv = create_timeval(target);

            unsafe {
                let poll = self.get();
                let timeout_new = (*poll).timeout_new;
                if let Some(tn) = timeout_new {
                    let atimeout = tn(poll, &tv, Some(timeout_fn), self.poller.get() as * mut c_void);
                    assert!(ptr::null() != atimeout);
                    return Ok(Timeout{timeout: atimeout, poller: self});
                }
            }
            Err(AvahiError::Timeout)
        }
    }

    pub struct Timeout<'a> {
        timeout: * mut avahi_sys::AvahiTimeout,
        poller: &'a Poller,
    }

    impl<'a> Drop for Timeout<'a> {
        fn drop(&mut self) {
            unsafe {
                let poll = self.poller.get();
                let free_func = (*poll).timeout_free;
                if let Some(ff) = free_func {
                    ff(self.timeout);
                }
            }
        }
    }

    unsafe extern "C" fn timeout_fn(_timeout: *mut avahi_sys::AvahiTimeout,
                                  userdata: *mut c_void) {
        let spoll: * mut avahi_sys::AvahiSimplePoll = transmute(userdata);
        avahi_sys::avahi_simple_poll_quit(spoll);
    }

    impl Drop for Poller {
        fn drop(&mut self) {
            unsafe {
                avahi_sys::avahi_simple_poll_free(self.poller.get());
            }
        }
    }

    pub struct Client {
        poller: Rc<Poller>,
        client : * mut avahi_sys::AvahiClient,
        service_browser: Option<ServiceBrowser>,
    }

    impl Client {
        pub fn new(poller: Rc<Poller>) -> Result<Client, AvahiError> {
            unsafe {
                let mut err: c_int = 0;
                let client = avahi_sys::avahi_client_new(
                                      poller.get(),
                                      avahi_sys::AvahiClientFlags(0),
                                      None,
                                      ptr::null_mut(),
                                      &mut err);
                if 0 == err {
                    return Ok(Client{poller, client: client, service_browser: None});
                }
                return Err(AvahiError::ClientNew);
            }
        }

        fn errno(&self) -> i32 {
            unsafe {
                avahi_sys::avahi_client_errno(self.client)
            }
        }

        pub fn create_service_browser(&mut self, service_type: &str, callback: service_browser_callback::CallbackBoxed2) -> Result<(), AvahiError> {
            let service_browser = self.create_service_browser2(service_type, callback);
            match service_browser {
                Ok(sb) => {
                    self.service_browser = Some(sb);
                    Ok(())},
                Err(err) => {
                    self.service_browser = None;
                    Err(err)},
            }
        }

        fn create_service_browser2(&self, service_type: &str, user_callback: service_browser_callback::CallbackBoxed2) -> Result<ServiceBrowser, AvahiError> {
            let cb_option = Some(user_callback);

            unsafe {
                let flag = transmute(0);

                let ctype = ffi::CString::new(service_type)?;
                let (callback, userdata) = service_browser_callback::get_callback_with_data(&cb_option);
                let sb = avahi_sys::avahi_service_browser_new(self.client, -1, -1, ctype.as_ptr(), ptr::null(), flag, callback, userdata);
                if ptr::null() != sb {
                    Ok(ServiceBrowser::new(sb, cb_option.unwrap()))
                } else {
                    Err(AvahiError::CreateServiceBrowser(String::from("error while creating service browser"), self.errno()))
                }
            }
        }

        pub fn create_service_resolver(&self, ifindex: IfIndex, prot: Protocol, name: &str, type_: &str, domain: &str, cb: resolver_callback::CallbackBoxed2) {
            unsafe {
                let cb_option = Some(cb);
                let (callback, userdata) = resolver_callback::get_callback_with_data(&cb_option);

                let name_string = ffi::CString::new(name);
                let type_string = ffi::CString::new(type_);
                let domain_string = ffi::CString::new(domain);

                if name_string.is_ok() && type_string.is_ok() && domain_string.is_ok() {
                    avahi_sys::avahi_service_resolver_new(self.client, ifindex, prot, name_string.unwrap().as_ptr(), type_string.unwrap().as_ptr(), domain_string.unwrap().as_ptr(), -1, transmute(0), callback, userdata);
                }
            }
        }
    }

    impl Drop for Client {
        fn drop(&mut self) {
            self.service_browser = None;
            unsafe {
                avahi_sys::avahi_client_free(self.client);
            }
        }
    }

    callback_types![
        service_browser_callback,
        [avahi_sys, avahi2::avahi],
        Fn(avahi::IfIndex, avahi::Protocol, avahi::BrowserEvent, &str, &str, &str, avahi::LookupResultFlags),
        avahi_sys::AvahiServiceBrowserCallback,
        avahi::service_browser_callback_fn];

    unsafe extern "C" fn service_browser_callback_fn(
        _service_browser: *mut avahi_sys::AvahiServiceBrowser, ifindex: avahi_sys::AvahiIfIndex, protocol: avahi_sys::AvahiProtocol, event: avahi_sys::AvahiBrowserEvent, name: *const c_char, typee: *const c_char, domain: *const c_char, flags: avahi_sys::AvahiLookupResultFlags, userdata: *mut c_void) {
        let functor : &service_browser_callback::CallbackBoxed = transmute(userdata);

        let name_string = init_if_not_null(name);
        let type_string = init_if_not_null(typee);
        let domain_string = init_if_not_null(domain);

        functor(ifindex, protocol, event, &name_string, &type_string, &domain_string, flags);
    }

    struct ServiceBrowser {
        service_browser: * mut avahi_sys::AvahiServiceBrowser,
        callback : service_browser_callback::CallbackBoxed2,
    }

    impl ServiceBrowser {
        fn new(service_browser: * mut avahi_sys::AvahiServiceBrowser, callback : service_browser_callback::CallbackBoxed2
            ) -> ServiceBrowser {
            ServiceBrowser{service_browser: service_browser, callback: callback}
        }

    }

    impl Drop for ServiceBrowser {
        fn drop(&mut self) {
            unsafe {
                avahi_sys::avahi_service_browser_free(self.service_browser);
            }
        }
    }

    pub fn create_service_browser_callback(client: Rc<Mutex<Client>>, poller: Rc<Poller>, tx: Sender<String>, name_to_filter: &str) -> service_browser_callback::CallbackBoxed2 {
        let filter_name = String::from(name_to_filter);

        let scrcb: resolver_callback::CallbackBoxed2 = Rc::new(Box::new(move |host_name| {
            if let Some(hostname) = host_name {
                tx.send(hostname).ok();
                poller.quit();
            }
        }));

        let sbcb: service_browser_callback::CallbackBoxed2 = Rc::new(Box::new(
            move |_ifindex, _protocol, _event, name_string, type_string, domain_string, _flags| {
                if avahi_sys::AvahiBrowserEvent::AVAHI_BROWSER_NEW == _event && name_string.contains(&filter_name) {
                    if let Ok(client_locked) = client.lock() {
                        client_locked.create_service_resolver(_ifindex, _protocol, name_string, type_string, domain_string, scrcb.clone());
                    }
                }
            }));

        sbcb
    }

    unsafe extern "C" fn callback_fn_resolver(
        r: *mut ServiceResolver,
           _interface: IfIndex,
           _protocol: Protocol,
           event: ResolverEvent,
           _name: *const c_char,
           _type_: *const c_char,
           _domain: *const c_char,
           host_name: *const c_char,
           _a: *const Address,
           _port: u16,
           _txt: *mut StringList,
           _flags: LookupResultFlags,
           userdata: *mut c_void) {
        if avahi_sys::AvahiResolverEvent::AVAHI_RESOLVER_FOUND == event {
            let functor : &resolver_callback::CallbackBoxed = transmute(userdata);

            let host_name_string;
            if ptr::null() != host_name {
                host_name_string = Some(ffi::CStr::from_ptr(host_name).to_string_lossy().into_owned())
            } else {
                host_name_string = None
            }

            functor(host_name_string);
        }
        avahi_sys::avahi_service_resolver_free(r);
    }

    callback_types![
        resolver_callback,
        [avahi2::avahi, avahi_sys],
        Fn(Option<String>),
        avahi_sys::AvahiServiceResolverCallback,
        avahi::callback_fn_resolver];

    #[cfg(test)]
    mod test {
        use libc::c_void;
        use std::rc::Rc;
        use std::ptr;
        use avahi2::avahi::service_browser_callback::{
            get_callback_with_data, CallbackBoxed, CallbackBoxed2, CCallback, CALLBACK_FN};

        #[test]
        fn get_callback_with_data_without_callback_works() {
            let (c_callback, data) = get_callback_with_data(&None);
            assert_eq!(None, c_callback);
            assert_eq!(None, c_callback);
            assert_eq!(ptr::null_mut(), data);
        }

        #[test]
        fn get_callback_with_data_with_callback_works() {
            let cb: CallbackBoxed2 = Rc::new(Box::new(|_,_,_,_,_,_,_| {}));
            let expected_userdata_cfn = &*cb as * const CallbackBoxed;
            let expected_userdata_mfn = expected_userdata_cfn as * mut CallbackBoxed;
            let expected_userdata_mv = expected_userdata_mfn as * mut c_void;
            assert!(0x0 != expected_userdata_mv as usize);
            assert!(0x1 != expected_userdata_mv as usize);
            let (c_callback, data) = get_callback_with_data(&Some(cb));
            assert!(c_callback.is_some());

            let expected_callback = CALLBACK_FN.unwrap() as * const CCallback;
            let actual_callback = c_callback.unwrap() as * const CCallback;
            assert_eq!(expected_callback, actual_callback);

            assert_eq!(expected_userdata_mv, data);
        }
    }
}

#[cfg(test)]
mod test {
    use avahi2::avahi::{Client, Poller};
    use avahi2;

    use std::rc::Rc;
    use std::sync::mpsc::channel;
    use libc::c_void;

    #[test]
    fn address_of_closures() {
        type ClosureFn = Fn(u32);
        type BoxedClosure = Box<ClosureFn>;

        let bla: Box<u32> = Box::new(42);
        println!("ptr of int on heap {:p}", bla);
        let (tx, rx) = channel();
        let cb: Rc<BoxedClosure> = Rc::new(Box::new(move |n| {
            tx.send(n).unwrap();
        }));

        println!("address of static function {:?}", address_of_closures as * const fn());
        println!("stack address of cb {:?}", &cb as * const Rc<BoxedClosure>);
        println!("pointer of callback on heap via :p {:p}", cb);

        let cb_ref : &BoxedClosure = &*cb;
        let cb_ptr = cb_ref as * const BoxedClosure;
        println!("pointer of callback on heap via cast {:?}", cb_ptr);
        println!("pointer of callback on heap casted without intermediaries {:?}",  &*cb as &BoxedClosure as * const BoxedClosure);
        println!("pointer of callback on heap casted without intermediaries {:?}",  &*cb as &BoxedClosure as * const BoxedClosure as * const c_void);

        unsafe {
            (*cb_ptr)(3);
        }

        assert!(3 == rx.recv().unwrap());
    }

    #[test]
    fn constructor_without_callback_works() {
        let poller = Rc::new(Poller::new().ok().unwrap());
        let _ = Client::new(poller);
    }

    #[test]
    fn create_service_browser_with_callback() {
        let receiver = avahi2::get_receiver();
        assert!("DENON-AVR-1912.local" == receiver.unwrap());
    }

    #[test]
    fn get_hostname() {
        let host = avahi2::get_hostname("_presence._tcp", "");
        assert!("barcas.local" == host.unwrap());
    }
}

use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::time::Duration;
use std::rc::Rc;

pub fn get_hostname(type_: &str, filter: &str) -> Result<String, avahi::AvahiError> {
    let poller = Rc::new(avahi::Poller::new()?);
    let client = Rc::new(Mutex::new(avahi::Client::new(poller.clone())?));

    let (tx_host, rx_host) = channel();

    let sbcb = avahi::create_service_browser_callback(client.clone(), poller.clone(), tx_host, filter);

    let sb1 = client.lock()?.create_service_browser(type_, sbcb);
    assert!(sb1.is_ok());

    let _timeout = poller.timeout(Duration::from_millis(2000))?;
    poller.looop();

    match rx_host.try_recv() {
        Ok(hostname) => return Ok(hostname),
        Err(_) => return Err(avahi::AvahiError::NoHostsFound),
    }
}

pub fn get_receiver() -> Result<String, avahi::AvahiError> {
    get_hostname("_raop._tcp", "DENON")
}

