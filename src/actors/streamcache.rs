use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Cursor};
use std::time::{Duration, Instant};

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Response};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use failure::Fail;
use serde::Serialize;
use uuid::Uuid;

use crate::actors::websocket::{StreamWebSocket, WebSocketResponse};

const EXPIRATION_INTERVAL: Duration = Duration::from_secs(5);
const LOG_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Fail, Debug)]
#[fail(display = "could not write chunk")]
pub struct WriteChunkError;

#[derive(Debug, Serialize, Clone)]
pub struct LogLine {
    idx: usize,
    ts: DateTime<Utc>,
    line: String,
}

impl LogLine {
    pub fn now(idx: usize) -> LogLine {
        LogLine {
            idx,
            ts: Utc::now(),
            line: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct LogStream {
    lines: Vec<LogLine>,
    next_idx: usize,
    created: Instant,
    last_change: Instant,
    sockets: HashSet<Addr<StreamWebSocket>>,
}

impl LogStream {
    pub fn new() -> LogStream {
        let now = Instant::now();
        LogStream {
            lines: Vec::new(),
            next_idx: 0,
            created: now,
            last_change: now,
            sockets: HashSet::new(),
        }
    }

    pub fn tail(&self, count: usize) -> Vec<LogLine> {
        self.lines[self.lines.len().saturating_sub(count)..].to_vec()
    }
}

#[derive(Debug)]
pub struct StreamCache {
    streams: HashMap<Uuid, LogStream>,
}

impl StreamCache {
    pub fn new() -> StreamCache {
        StreamCache {
            streams: HashMap::new(),
        }
    }

    fn clear_expired(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(EXPIRATION_INTERVAL, |act, _ctx| {
            let mut to_remove = HashSet::new();
            for (stream_id, stream) in act.streams.iter() {
                if Instant::now().duration_since(stream.last_change) > LOG_TIMEOUT {
                    to_remove.insert(*stream_id);
                }
            }
            act.streams
                .retain(|stream_id, _| !to_remove.contains(stream_id));
        });
    }
}

impl Actor for StreamCache {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.clear_expired(ctx);
    }
}

#[derive(Debug)]
pub struct TailLog {
    pub stream_id: Uuid,
    pub lines: usize,
}

#[derive(Debug)]
pub struct SubscribeSocket {
    pub stream_id: Uuid,
    pub socket: Addr<StreamWebSocket>,
}

impl Message for SubscribeSocket {
    type Result = ();
}

impl Handler<SubscribeSocket> for StreamCache {
    type Result = ();

    fn handle(&mut self, message: SubscribeSocket, _context: &mut Self::Context) -> Self::Result {
        let stream = self
            .streams
            .entry(message.stream_id)
            .or_insert_with(LogStream::new);
        message.socket.do_send(WebSocketResponse::Tail {
            stream_id: message.stream_id,
            lines: stream.tail(1000),
        });
        stream.sockets.insert(message.socket);
    }
}

#[derive(Debug)]
pub struct UnsubscribeSocket {
    pub stream_id: Uuid,
    pub socket: Addr<StreamWebSocket>,
}

impl Message for UnsubscribeSocket {
    type Result = ();
}

impl Handler<UnsubscribeSocket> for StreamCache {
    type Result = ();

    fn handle(&mut self, message: UnsubscribeSocket, _context: &mut Self::Context) -> Self::Result {
        if let Some(stream) = self.streams.get_mut(&message.stream_id) {
            stream.sockets.remove(&message.socket);
        }
    }
}

#[derive(Debug)]
pub struct WriteChunk {
    pub stream_id: Uuid,
    pub bytes: Bytes,
}

impl Message for WriteChunk {
    type Result = Result<(), WriteChunkError>;
}

impl Handler<WriteChunk> for StreamCache {
    type Result = Response<(), WriteChunkError>;

    fn handle(&mut self, message: WriteChunk, _context: &mut Self::Context) -> Self::Result {
        let now = Instant::now();
        let stream = if let Some(stream) = self.streams.get_mut(&message.stream_id) {
            stream.last_change = now;
            stream
        } else {
            let stream = LogStream::new();
            self.streams.insert(message.stream_id, stream);
            self.streams.get_mut(&message.stream_id).unwrap()
        };

        // nothing to do here
        if message.bytes.is_empty() {
            return Response::reply(Ok(()));
        }

        let mut line_buf = Vec::new();
        let mut rdr = Cursor::new(&message.bytes);

        loop {
            line_buf.clear();
            let read = rdr.read_until(b'\n', &mut line_buf).unwrap();
            if read == 0 {
                break;
            }
            let s = String::from_utf8_lossy(&line_buf);

            // last line was terminated, start a new one.
            if stream.lines.last().map_or(true, |x| x.line.ends_with('\n')) {
                stream.lines.push(LogLine::now(stream.next_idx));
                stream.next_idx += 1;
            }
            let line_idx = stream.lines.len() - 1;
            stream.lines[line_idx].line.push_str(&s);

            for socket in stream.sockets.iter() {
                socket.do_send(WebSocketResponse::Tail {
                    stream_id: message.stream_id,
                    lines: vec![stream.lines[line_idx].clone()],
                });
            }
        }

        Response::reply(Ok(()))
    }
}

impl Message for TailLog {
    type Result = Result<Vec<LogLine>, WriteChunkError>;
}

impl Handler<TailLog> for StreamCache {
    type Result = Response<Vec<LogLine>, WriteChunkError>;

    fn handle(&mut self, message: TailLog, _context: &mut Self::Context) -> Self::Result {
        if let Some(stream) = self.streams.get(&message.stream_id) {
            Response::reply(Ok(stream.tail(message.lines)))
        } else {
            Response::reply(Ok(vec![]))
        }
    }
}
