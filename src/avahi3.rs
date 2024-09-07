use crate::avahi_error::Error;
use std::any::Any;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use zeroconf::prelude::{TEventLoop, TMdnsBrowser};
use zeroconf::txt_record::TTxtRecord;
use zeroconf::{MdnsBrowser, ServiceDiscovery, ServiceType};

#[derive(Default, Debug)]
struct Context {
    service_discovery: Option<ServiceDiscovery>,
}

fn get_hostname(service_type: ServiceType) -> Result<ServiceDiscovery, Error> {
    let context: Arc<Mutex<Context>> = Arc::default();
    let mut browser = MdnsBrowser::new(service_type);
    browser.set_service_discovered_callback(Box::new(on_service_discovered));
    browser.set_context(Box::new(context.clone()));
    let event_loop = browser.browse_services()?;

    let timeout = Duration::from_secs(2);
    let start = std::time::Instant::now();

    while context
        .lock()
        .and_then(|res| match res.service_discovery {
            Some(_) => Ok(()),
            _ => Err(PoisonError::new(res)),
        })
        .is_err()
        && (Instant::now() - start) <= timeout
    {
        match event_loop.poll(Duration::from_secs(0)) {
            Ok(_) => {}
            Err(x) => println!("{}", x),
        }
    }

    let result = match &context.lock().unwrap().service_discovery {
        Some(x) => Ok(x.clone()),
        None => Err(Error::NoHostsFound),
    };
    result
}

fn on_service_discovered(
    result: zeroconf::Result<ServiceDiscovery>,
    context: Option<Arc<dyn Any>>,
) {
    if let Ok(sd) = result {
        if let Some(ctx) = context {
            if let Some(m) = ctx.downcast_ref::<Arc<Mutex<Context>>>() {
                if let Ok(mut ctx) = m.lock() {
                    ctx.service_discovery = Some(sd);
                }
            }
        }
    }
}

fn get_roap_service_type() -> ServiceType {
    ServiceType::new("raop", "tcp").unwrap()
}

pub fn get_receiver() -> Result<String, Error> {
    let sd = get_hostname(get_roap_service_type())?;
    if let Some(txt) = sd.txt() {
        for (_type, value) in txt.iter() {
            if value.contains("DENON") {
                return Ok(sd.host_name().clone());
            }
        }
    }
    Err(Error::NoHostsFound)
}

#[cfg(test)]
mod test {
    use super::{get_receiver, get_roap_service_type, on_service_discovered, Context};
    use crate::{avahi3::get_hostname, avahi_error::Error};
    use std::{
        net::TcpStream,
        sync::{Arc, Mutex},
    };
    use zeroconf::{error, prelude::BuilderDelegate, ServiceDiscovery, ServiceType};

    fn create_service_discovery() -> ServiceDiscovery {
        ServiceDiscovery::builder()
            .address(String::from("blub.local"))
            .name(String::from("bla"))
            .service_type(ServiceType::new("foo", "bar").unwrap())
            .domain(String::from("test"))
            .host_name(String::from("bla.local"))
            .port(123)
            .txt(None)
            .build()
            .unwrap()
    }

    #[test]
    fn get_receiver_may_return() {
        match get_receiver() {
            // TODO test sometimes gets address but fails to connect, why?
            // - one reason: not all computers with raop mDNS service have telnet (port 23) running
            Ok(address) => {
                let stream = TcpStream::connect((address.clone(), 23));
                println!("address == {}, stream == {:?}", address, stream);
                assert!(matches!(stream, Ok(_)))
            }
            Err(e) => assert!(matches!(e, Error::NoHostsFound)),
        }
    }

    #[test]
    fn get_hostname_returns() {
        match get_hostname(get_roap_service_type()) {
            Ok(address) => {
                let stream = TcpStream::connect((address.host_name().clone(), *address.port()));
                println!(
                    "address == {}, port == {}, stream == {:?}",
                    address.host_name(),
                    address.port(),
                    stream
                );
                assert!(matches!(stream, Ok(_)))
            }
            Err(e) => assert!(matches!(e, Error::NoHostsFound)),
        }
    }

    #[test]
    fn timeout() {
        let sn = ServiceType::new("does_not_exit", "tcp").unwrap();
        assert!(matches!(get_hostname(sn), Err(Error::NoHostsFound)));
    }

    #[test]
    fn on_service_discovered_works() {
        let sd = create_service_discovery();
        let context: Arc<Arc<Mutex<Context>>> = Arc::default();
        assert_eq!(context.lock().unwrap().service_discovery, None);
        on_service_discovered(Ok(sd.clone()), Some(context.clone()));
        assert_eq!(context.lock().unwrap().service_discovery, Some(sd.clone()));
    }

    #[test]
    fn on_service_discovered_does_nothing_on_no_service_discovery() {
        let context: Arc<Arc<Mutex<Context>>> = Arc::default();
        assert_eq!(context.lock().unwrap().service_discovery, None);
        on_service_discovered(
            Err(error::Error::new(String::from("blub"))),
            Some(context.clone()),
        );
        assert_eq!(context.lock().unwrap().service_discovery, None);
    }

    #[test]
    fn on_service_discovered_does_nothing_on_no_context() {
        let sd = create_service_discovery();
        let context: Arc<Arc<Mutex<Context>>> = Arc::default();
        assert_eq!(context.lock().unwrap().service_discovery, None);
        on_service_discovered(Ok(sd.clone()), None);
        assert_eq!(context.lock().unwrap().service_discovery, None);
    }

    #[test]
    fn on_service_discovered_does_nothing_on_context_with_different_type() {
        let sd = create_service_discovery();
        let context: Arc<Mutex<Context>> = Arc::default();
        assert_eq!(context.lock().unwrap().service_discovery, None);
        on_service_discovered(Ok(sd.clone()), Some(context.clone()));
        assert_eq!(context.lock().unwrap().service_discovery, None);
    }
}
