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
    _phantom: PhantomData<&'ws ()>, // FIXME or other?
}

impl<'ws, WS> Stack for ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    type RequestUnionError = core::convert::Infallible; // WS::RequestUnionError; // FIXME: probably more …

    type RequestMessage<'a>
        = core::convert::Infallible
    where
        Self: 'a;

    type ResponseMessage<'a>
        = core::convert::Infallible
    where
        Self: 'a;

    type TransportError = core::convert::Infallible; // FIXME: probably more

    async fn request<Req: Request<Self>>(
        &mut self,
        request: Req,
    ) -> Result<Req::Output, Self::TransportError> {
        todo!()
    }
}

struct RequestWrapper<'ws, WS, R: Request<WS>>
where
    R: Request<WS>,
    WS: Stack + 'ws,
{
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
