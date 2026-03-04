//! Experiments on client use
//!
//! Everything in here is in decoupled from the rest of coapcore, and linked only by the underlying
//! libraries they share.
//!
//! The functionality provided through here will be moved in with the rest on the long run.

use core::marker::PhantomData;

use coap_message::MinimalWritableMessage;
use coap_request::{Request, Stack};

/// Wrapper around any CoAP requester, which wraps all data in EDHOC.
///
/// This uses raw public keys as UCCS everywhere.
///
/// Creating this does not yet start EDHOC requests; those only get exchanged on demand.
pub struct ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    wire: WS,
    security_config: SecurityConfig,
    state: (),
    _phantom: PhantomData<&'ws ()>, // FIXME or other?
}

struct SecurityConfig {
    own_credential: lakers::Credential,
    own_key: lakers::BytesP256ElemLen,
    own_credential_transfer: lakers::CredentialTransfer,
    peer_credential: lakers::Credential,
}

impl<'ws, WS> ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    /// Creates all the client state needed to send requests, and stores the credentials.
    pub fn new(
        wire: WS,
        own_credential: lakers::Credential,
        own_key: lakers::BytesP256ElemLen,
        own_credential_transfer: lakers::CredentialTransfer,
        peer_credential: lakers::Credential,
    ) -> Self {
        Self {
            wire,
            security_config: SecurityConfig {
                own_credential,
                own_key,
                own_credential_transfer,
                peer_credential,
            },
            state: (),
            _phantom: PhantomData,
        }
    }
}

impl<'ws, WS> Stack for ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    type RequestUnionError = <liboscore::ProtectedMessage as MinimalWritableMessage>::UnionError; // WS::RequestUnionError; // FIXME: probably more …

    type RequestMessage<'a>
        = liboscore::ProtectedMessage
    where
        Self: 'a;

    type ResponseMessage<'a>
        = liboscore::ProtectedMessage
    where
        Self: 'a;

    type TransportError = core::convert::Infallible; // FIXME: probably more

    async fn request<Req: Request<Self>>(
        &mut self,
        request: Req,
    ) -> Result<Req::Output, Self::TransportError> {
        self.wire
            .request(RequestWrapper {
                security_config: self.security_config,
                request,
                _phantom: PhantomData,
            })
            .await
    }
}

struct RequestWrapper<'ws, WS, R: Request<WS>>
where
    R: Request<WS>,
    WS: Stack + 'ws,
{
    security_config: SecurityConfig,
    request: R,
    // FIXME can we do w/o?
    _phantom: PhantomData<&'ws WS>,
}

impl<'ws, WS, R: Request<WS>> Request<ClientSecurityWrapper<'ws, WS>> for RequestWrapper<'ws, WS, R>
where
    R: Request<WS>,
    WS: Stack,
{
    type Output = R::Output; // FIXME or more?

    type Carry = R::Carry; // FIXME or more?

    fn build_request(
        &mut self,
        request: &mut <ClientSecurityWrapper<'ws, WS> as Stack>::RequestMessage<'_>,
    ) -> impl Future<
        Output = Result<Self::Carry, <ClientSecurityWrapper<'ws, WS> as Stack>::RequestUnionError>,
    > {
        todo!()
    }

    fn process_response(
        &mut self,
        response: &<ClientSecurityWrapper<'ws, WS> as Stack>::ResponseMessage<'_>,
        carry: Self::Carry,
    ) -> impl Future<Output = Self::Output> {
        todo!()
    }
}
