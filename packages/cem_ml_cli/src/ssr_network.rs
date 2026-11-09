//! Exact, endpoint-pinned network reads for the CLI SSR resource purpose.
use cem_ml::{
    resolver::{
        ssr_resource_url, ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead,
        ResolvedWrite, ResolverDiagnostic, ResolverRegistry, ResourceResolver, SsrReadBudget,
    },
    schema::registry::content_type_essence,
};
use reqwest::{header, StatusCode, Url};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::{poll_fn, Future},
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    task::Poll,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkReadGrant {
    url: Url,
    endpoint: IpAddr,
}

impl FromStr for NetworkReadGrant {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (raw_url, endpoint) = value
            .rsplit_once('=')
            .ok_or("Expected an SSR network grant as URL=IP")?;
        if raw_url.len() > 4096
            || raw_url.trim() != raw_url
            || raw_url.contains('\\')
            || raw_url.chars().any(char::is_control)
        {
            return Err("SSR network grant requires a bounded explicit URL".into());
        }
        let endpoint: IpAddr = endpoint
            .parse()
            .map_err(|_| "SSR network grant requires a literal IP address")?;
        let url = Url::parse(raw_url).map_err(|_| "Invalid SSR network grant URL")?;
        if url.query().is_some()
            || url.fragment().is_some()
            || ssr_resource_url(raw_url).ok().as_deref() != Some(url.as_str())
        {
            return Err("SSR network grants require an HTTP(S) URL without credentials, a query or a fragment".into());
        }
        if endpoint.is_unspecified() || endpoint.is_multicast() {
            return Err("SSR network grant requires a unicast endpoint".into());
        }
        if url.domain().is_none() {
            let literal = url
                .host_str()
                .unwrap_or_default()
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .ok();
            if literal != Some(endpoint) {
                return Err("SSR network grant for an IP URL must pin that same address".into());
            }
        }
        if url.scheme() == "http" && (url.domain().is_some() || !endpoint.is_loopback()) {
            return Err("Plain HTTP SSR grants require a literal loopback URL and endpoint".into());
        }
        Ok(Self { url, endpoint })
    }
}

pub(crate) fn register(
    registry: &mut ResolverRegistry,
    grants: &[NetworkReadGrant],
    mirrored: &BTreeSet<String>,
) {
    for scheme in ["http", "https"] {
        let mut endpoints = BTreeMap::<String, BTreeSet<IpAddr>>::new();
        for grant in grants.iter().filter(|grant| grant.url.scheme() == scheme) {
            endpoints
                .entry(grant.url.as_str().into())
                .or_default()
                .insert(grant.endpoint);
        }
        if endpoints.is_empty() {
            continue;
        }
        let mut stamp = String::from("ssr-exact-pinned-get/1");
        for (url, addresses) in &endpoints {
            use std::fmt::Write;
            for address in addresses {
                let _ = write!(stamp, ":{}:{url}={address}", url.len());
            }
        }
        let mirror = if mirrored.contains(scheme) {
            registry.resolver_for(scheme, ResolvePurpose::SsrResource, ResolveDirection::Read)
        } else {
            None
        };
        registry.register(
            scheme,
            ResolvePurpose::SsrResource,
            ResolveDirection::Read,
            NetworkReadResolver {
                endpoints,
                stamp,
                mirror,
            },
        );
    }
}

struct NetworkReadResolver {
    endpoints: BTreeMap<String, BTreeSet<IpAddr>>,
    stamp: String,
    mirror: Option<Arc<dyn ResourceResolver>>,
}

impl ResourceResolver for NetworkReadResolver {
    fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        Err(unsupported(request))
    }

    fn write(
        &self,
        request: &ResolveRequest,
        _: &[u8],
    ) -> Result<ResolvedWrite, ResolverDiagnostic> {
        Err(unsupported(request))
    }

    fn ssr_read_policy_stamp(&self) -> Option<&str> {
        Some(&self.stamp)
    }

    fn read_ssr_resource(
        &self,
        request: &ResolveRequest,
        budget: &SsrReadBudget<'_>,
    ) -> Result<ResolvedRead, ResolverDiagnostic> {
        if request.purpose != ResolvePurpose::SsrResource
            || request.direction != ResolveDirection::Read
        {
            return Err(unsupported(request));
        }
        budget.check(request)?;
        if let Some(mirror) = &self.mirror {
            match mirror.read_ssr_resource(request, budget) {
                Err(ResolverDiagnostic::UnsupportedResolver { .. }) => {}
                result => return result,
            }
        }
        let addresses = self
            .endpoints
            .get(&request.uri)
            .ok_or_else(|| failure(request, "SSR network read requires an exact URL grant"))?;
        if addresses.len() != 1 {
            return Err(failure(
                request,
                "Conflicting IP pins for an SSR network URL",
            ));
        }
        let endpoint = *addresses.first().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| failure(request, "Cannot initialize SSR network runtime"))?;
        runtime.block_on(async {
            // Run HTTP and TLS on the original measured thread. A timer wakes
            // stalled reads so cancellation and the SSR deadline can be observed.
            let mut fetch = std::pin::pin!(fetch(request, budget, endpoint));
            let mut tick = tokio::time::interval(Duration::from_millis(25));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            poll_fn(|cx| {
                if tick.poll_tick(cx).is_ready() {
                    if let Err(error) = budget.check(request) {
                        return Poll::Ready(Err(error));
                    }
                    let _ = tick.poll_tick(cx);
                }
                fetch.as_mut().poll(cx)
            })
            .await
        })
    }
}

struct NoAmbientDns;

impl reqwest::dns::Resolve for NoAmbientDns {
    fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async { Err("SSR network read has no DNS authority".into()) })
    }
}

async fn fetch(
    request: &ResolveRequest,
    budget: &SsrReadBudget<'_>,
    endpoint: IpAddr,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    let url = Url::parse(&request.uri).map_err(|_| failure(request, "Invalid SSR network URL"))?;
    let expected_type = request
        .content_type_hint
        .as_deref()
        .ok_or_else(|| failure(request, "SSR network read requires an expected media type"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| failure(request, "Missing SSR network port"))?;
    let pinned = SocketAddr::new(endpoint, port);
    let deadline = budget.remaining_wall_time(request)?;
    let mut client = reqwest::Client::builder()
        .no_proxy()
        .dns_resolver(Arc::new(NoAmbientDns))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .http1_only()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .timeout(deadline);
    if let Some(domain) = url.domain() {
        client = client.resolve(domain, pinned);
    }
    let client = client
        .build()
        .map_err(|_| failure(request, "Cannot initialize SSR HTTP client"))?;
    let transport_error = |error: reqwest::Error| {
        let message = if error.is_timeout() {
            "Original SSR resource wall-time budget exhausted"
        } else {
            "SSR network transport failed"
        };
        failure(request, message)
    };
    let mut response = client
        .get(url)
        .header(header::ACCEPT, expected_type)
        .header(header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(transport_error)?;
    budget.check(request)?;
    if response.url().as_str() != request.uri {
        return Err(failure(
            request,
            "SSR network response changed the public URL",
        ));
    }
    if response.remote_addr() != Some(pinned) {
        return Err(failure(
            request,
            "SSR network response arrived from an unpinned endpoint",
        ));
    }
    if response.status() != StatusCode::OK || response.headers().contains_key(header::CONTENT_RANGE)
    {
        return Err(failure(
            request,
            &format!(
                "SSR network read requires a complete HTTP 200 response, received {}",
                response.status()
            ),
        ));
    }
    if !response
        .headers()
        .get_all(header::CONTENT_ENCODING)
        .iter()
        .all(|value| value.as_bytes().eq_ignore_ascii_case(b"identity"))
    {
        return Err(failure(
            request,
            "SSR network reads require identity content encoding",
        ));
    }
    let mut types = response.headers().get_all(header::CONTENT_TYPE).iter();
    let content_type = types
        .next()
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.contains(','))
        .filter(|value| content_type_essence(value) == expected_type)
        .filter(|_| types.next().is_none())
        .ok_or_else(|| {
            failure(
                request,
                "SSR network response requires the requested Content-Type",
            )
        })?
        .to_owned();
    let byte_limit = || failure(request, "Original SSR resource byte budget exceeded");
    if response
        .content_length()
        .is_some_and(|length| length > budget.max_bytes as u64)
    {
        return Err(byte_limit());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        budget.check(request)?;
        if chunk.len() > budget.max_bytes - bytes.len() {
            return Err(byte_limit());
        }
        bytes.extend_from_slice(&chunk);
    }
    budget.check(request)?;
    Ok(ResolvedRead {
        uri: request.uri.clone(),
        bytes,
        content_type: Some(content_type),
    })
}

fn failure(request: &ResolveRequest, message: &str) -> ResolverDiagnostic {
    ResolverDiagnostic::Io {
        uri: request.uri.clone(),
        message: message.into(),
    }
}

fn unsupported(request: &ResolveRequest) -> ResolverDiagnostic {
    ResolverDiagnostic::UnsupportedResolver {
        uri: request.uri.clone(),
        purpose: request.purpose,
        direction: request.direction,
    }
}

#[cfg(all(test, any(target_os = "linux", windows)))]
mod tests {
    use super::*;
    use cem_ml::operation_control::{
        cpu::NativeCpuAccounting, OperationControl, ROOT_EXECUTION_SCOPE_ID,
    };
    use std::{io::Read, net::TcpListener, thread};

    #[test]
    fn exact_grants_refuse_ambiguous_or_unpinned_urls() {
        for grant in [
            "https://user@fixture.test/card.html=192.0.2.1",
            "https://fixture.test/card.html#card=192.0.2.1",
            "https://fixture.test/card.html?v=1=192.0.2.1",
            "https://fixture.test/card.html=fixture.test",
            "http://fixture.test/card.html=127.0.0.1",
            "http://192.0.2.1/card.html=192.0.2.1",
            "http://127.0.0.1/card.html=127.0.0.2",
        ] {
            assert!(grant.parse::<NetworkReadGrant>().is_err(), "{grant}");
        }
        let grant: NetworkReadGrant = "https://fixture.test/card.html=192.0.2.1".parse().unwrap();
        assert_eq!(grant.url.as_str(), "https://fixture.test/card.html");
        assert_eq!(grant.endpoint, "192.0.2.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn named_https_uses_the_pin_and_requires_a_tls_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let uri = format!(
            "https://ssr-fixture.invalid:{}/card.html",
            listener.local_addr().unwrap().port()
        );
        let server = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(4);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut record = [0; 2];
                        stream.read_exact(&mut record).unwrap();
                        assert_eq!(record, [22, 3]);
                        return;
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("Pinned HTTPS connection did not arrive: {error}"),
                }
            }
        });
        let mut registry = ResolverRegistry::new();
        register(
            &mut registry,
            &[format!("{uri}=127.0.0.1").parse().unwrap()],
            &BTreeSet::new(),
        );
        let request = ResolveRequest::new(uri, ResolvePurpose::SsrResource, ResolveDirection::Read)
            .with_content_type_hint("text/html");
        let control = OperationControl::default();
        let cpu = Arc::new(NativeCpuAccounting::new(&control, 5000).unwrap());
        let guard = cpu.begin(&control).unwrap();
        let budget = SsrReadBudget {
            control: &control,
            cpu: &cpu,
            scope: ROOT_EXECUTION_SCOPE_ID,
            max_bytes: 1024,
            max_wall_time_ms: 5000,
        };
        let error = registry.read_ssr_resource(&request, &budget).unwrap_err();
        assert!(
            error.to_string().contains("SSR network transport failed"),
            "{error}"
        );
        server.join().unwrap();
        guard.finish().unwrap();
    }

    #[test]
    fn stalled_network_read_observes_original_cancellation() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let uri = format!("http://{}/payload.cem", listener.local_addr().unwrap());
        let mut registry = ResolverRegistry::new();
        register(
            &mut registry,
            &[format!("{uri}=127.0.0.1").parse().unwrap()],
            &BTreeSet::new(),
        );
        let request = ResolveRequest::new(uri, ResolvePurpose::SsrResource, ResolveDirection::Read)
            .with_content_type_hint("text/cem-ml");
        let control = OperationControl::default();
        let cpu = Arc::new(NativeCpuAccounting::new(&control, 5000).unwrap());
        let guard = cpu.begin(&control).unwrap();
        let abort = control.abort_signal().clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            assert!(stream.read(&mut [0; 256]).unwrap() > 0);
            abort.abort();
            // Keep the socket open until the client drops its pending response.
            while stream.read(&mut [0; 256]).is_ok_and(|len| len > 0) {}
        });
        let budget = SsrReadBudget {
            control: &control,
            cpu: &cpu,
            scope: ROOT_EXECUTION_SCOPE_ID,
            max_bytes: 1024,
            max_wall_time_ms: 5000,
        };
        let error = registry.read_ssr_resource(&request, &budget).unwrap_err();
        assert!(
            matches!(error, ResolverDiagnostic::Cancelled { .. }),
            "{error}"
        );
        server.join().unwrap();
        guard.finish().unwrap();
    }
}
