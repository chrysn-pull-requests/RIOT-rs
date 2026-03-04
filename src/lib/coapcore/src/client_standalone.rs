use core::marker::PhantomData;

use coap_request::{Stack, Request};

pub struct ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    _phantom: PhantomData<&'ws WS>,
}

impl<'ws, WS> Stack for ClientSecurityWrapper<'ws, WS>
where
    WS: Stack + 'ws,
{
    type RequestUnionError = core::convert::Infallible;

    type RequestMessage<'a>
        = core::convert::Infallible
    where
        Self: 'a;

    type ResponseMessage<'a>
        = core::convert::Infallible
    where
        Self: 'a;

    type TransportError = core::convert::Infallible;

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
    _phantom: PhantomData<&'ws (WS, R)>,
}

impl<'ws, WS, R: Request<WS>> Request<ClientSecurityWrapper<'ws, WS>> for RequestWrapper<'ws, WS, R>
where
    R: Request<WS>,
    WS: Stack,
{
    type Output = R::Output;

    type Carry = R::Carry;

    fn build_request(
        &mut self,
        request: &mut <ClientSecurityWrapper<'ws, WS> as Stack>::RequestMessage<'_>,
    ) -> impl Future<
        Output = Result<Self::Carry, <ClientSecurityWrapper<'ws, WS> as Stack>::RequestUnionError>,
    > {
        core::future::pending()
    }

    fn process_response(
        &mut self,
        response: &<ClientSecurityWrapper<'ws, WS> as Stack>::ResponseMessage<'_>,
        carry: Self::Carry,
    ) -> impl Future<Output = Self::Output> {
        core::future::pending()
    }
}
