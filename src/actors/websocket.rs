use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web::ws;
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;

use crate::actors::streamcache::{LogLine, StreamCache, SubscribeSocket, UnsubscribeSocket};
use crate::server::ServerState;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "op")]
pub enum WebSocketRequest {
    #[serde(rename_all = "camelCase")]
    Subscribe { stream_id: Uuid },
    #[serde(rename_all = "camelCase")]
    Unsubscribe { stream_id: Uuid },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "op")]
pub enum WebSocketResponse {
    #[serde(rename_all = "camelCase")]
    Tail {
        stream_id: Uuid,
        lines: Vec<LogLine>,
    },
}

#[derive(Debug)]
pub struct StreamWebSocket {
    stream_cache: Addr<StreamCache>,
    last_heartbeat: Instant,
}

impl StreamWebSocket {
    pub fn new(stream_cache: Addr<StreamCache>) -> StreamWebSocket {
        StreamWebSocket {
            stream_cache: stream_cache,
            last_heartbeat: Instant::now(),
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }
            ctx.ping("");
        });
    }
}

impl Actor for StreamWebSocket {
    type Context = ws::WebsocketContext<Self, Arc<ServerState>>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for StreamWebSocket {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.last_heartbeat = Instant::now();
            }
            ws::Message::Text(text) => {
                // todo: error handling
                let msg: WebSocketRequest = serde_json::from_str(&text).unwrap();
                match msg {
                    WebSocketRequest::Subscribe { stream_id } => {
                        self.stream_cache.do_send(SubscribeSocket {
                            stream_id,
                            socket: ctx.address(),
                        });
                    }
                    WebSocketRequest::Unsubscribe { stream_id } => {
                        self.stream_cache.do_send(UnsubscribeSocket {
                            stream_id,
                            socket: ctx.address(),
                        });
                    }
                }
            }
            ws::Message::Binary(_) => {}
            ws::Message::Close(_) => {
                ctx.stop();
            }
        }
    }
}

impl Message for WebSocketResponse {
    type Result = ();
}

impl Handler<WebSocketResponse> for StreamWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WebSocketResponse, ctx: &mut Self::Context) {
        let resp = serde_json::to_string(&msg).unwrap();
        ctx.text(&resp);
    }
}
