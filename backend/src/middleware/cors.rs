use futures::FutureExt;
use rpc_toolkit::hyper::http::Error as HttpError;
use rpc_toolkit::hyper::{Body, Method, Request, Response};
use rpc_toolkit::rpc_server_helpers::{
    DynMiddlewareStage2, DynMiddlewareStage3, DynMiddlewareStage4,
};
use rpc_toolkit::Metadata;

pub async fn cors<M: Metadata>(
    req: &mut Request<Body>,
    _metadata: M,
) -> Result<Result<DynMiddlewareStage2, Response<Body>>, HttpError> {
    if req.method() == Method::OPTIONS {
        Ok(Err(Response::builder()
            .header(
                "Access-Control-Allow-Origin",
                if let Some(origin) = req.headers().get("origin").and_then(|s| s.to_str().ok()) {
                    origin
                } else {
                    "*"
                },
            )
            .header("Access-Control-Allow-Methods", "*")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Allow-Credentials", "true")
            .body(Body::empty())?))
    } else {
        Ok(Ok(Box::new(|_, _| {
            async move {
                let res: DynMiddlewareStage3 = Box::new(|_, _| {
                    async move {
                        let res: DynMiddlewareStage4 = Box::new(|res| {
                            async move {
                                res.headers_mut()
                                    .insert("Access-Control-Allow-Origin", "*".parse()?);
                                Ok::<_, HttpError>(())
                            }
                            .boxed()
                        });
                        Ok::<_, HttpError>(Ok(res))
                    }
                    .boxed()
                });
                Ok::<_, HttpError>(Ok(res))
            }
            .boxed()
        })))
    }
}
