use std::sync::Arc;

use actix::fut::FinishStream;
use actix::{Actor, Addr, ResponseFuture, System};
use actix_web::http::Method;
use actix_web::{
    server, ws, App, AsyncResponder, Error, HttpMessage, HttpRequest, HttpResponse, Json, Path,
    ResponseError,
};
use failure::{self, Fail};
use futures::{Future, Stream};
use uuid::Uuid;

use crate::actors::streamcache::{LogLine, StreamCache, TailLog, WriteChunk};
use crate::actors::websocket::StreamWebSocket;

pub struct ServerState {
    stream_cache: Addr<StreamCache>,
}

impl ServerState {}

#[derive(Fail, Debug)]
#[fail(display = "error reading input")]
pub struct ReadError;

impl ResponseError for ReadError {}

fn write_to_stream(
    req: HttpRequest<Arc<ServerState>>,
    info: Path<(Uuid,)>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    let stream_cache = req.state().stream_cache.clone();

    req.payload()
        .map_err(|_| Error::from(ReadError))
        .and_then(move |bytes| {
            stream_cache
                .send(WriteChunk {
                    stream_id: info.0,
                    bytes: bytes,
                })
                .map_err(Error::from)
        })
        .finish()
        .map(|_| HttpResponse::Ok().into())
        .responder()
}

fn stream_to_websocket(req: &HttpRequest<Arc<ServerState>>) -> Result<HttpResponse, Error> {
    ws::start(req, StreamWebSocket::new(req.state().stream_cache.clone()))
}

fn fetch_log(
    req: HttpRequest<Arc<ServerState>>,
    info: Path<(Uuid,)>,
) -> ResponseFuture<Json<Vec<LogLine>>, ReadError> {
    let future = req
        .state()
        .stream_cache
        .send(TailLog {
            stream_id: info.0,
            lines: 1000,
        })
        .map_err(|_| ReadError)
        .and_then(|result| result.map_err(|_| ReadError))
        .map(Json);
    Box::new(future)
}

pub fn serve(host: &str, port: u16) -> Result<(), failure::Error> {
    let sys = System::new("logteewoop");
    let stream_cache = StreamCache::new().start();
    let state = Arc::new(ServerState {
        stream_cache: stream_cache,
    });
    server::new(move || {
        App::with_state(state.clone())
            .resource("/{stream}/tail", |r| r.method(Method::GET).with(fetch_log))
            .resource("/ws", |r| r.method(Method::GET).f(stream_to_websocket))
            .resource("/{stream}/write", |r| {
                r.method(Method::POST).with(write_to_stream)
            })
    })
    .bind((host, port))?
    .start();
    sys.run();
    Ok(())
}
