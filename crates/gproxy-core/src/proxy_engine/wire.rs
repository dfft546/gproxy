use bytes::Bytes;

use serde_json;

use gproxy_protocol::sse::{SseEvent, SseParser};
use gproxy_provider_core::{Proto, StreamEvent, StreamFormat};

#[derive(Debug)]
pub struct StreamDecoder {
    proto: Proto,
    format: StreamFormat,
    sse: SseParser,
    // Best-effort JSON-line decoding for Gemini-style streams.
    json_buf: String,
}

impl StreamDecoder {
    pub fn new(proto: Proto, format: StreamFormat) -> Self {
        Self {
            proto,
            format,
            sse: SseParser::new(),
            json_buf: String::new(),
        }
    }

    pub fn push_bytes(&mut self, chunk: &Bytes) -> Vec<StreamEvent> {
        let mut out = Vec::new();

        match self.format {
            StreamFormat::SseNamedEvent | StreamFormat::SseDataOnly => {
                for ev in self.sse.push_bytes(chunk) {
                    if let Some(item) = decode_sse_event(self.proto, &ev) {
                        out.push(item);
                    }
                }
            }
            StreamFormat::JsonStream => {
                // 1) Try SSE framing (some upstreams use SSE even for "JSON object stream").
                for ev in self.sse.push_bytes(chunk) {
                    if let Some(item) = decode_sse_event(self.proto, &ev) {
                        out.push(item);
                    }
                }
                // 2) Try newline-delimited JSON objects as a fallback.
                if let Ok(s) = std::str::from_utf8(chunk) {
                    self.json_buf.push_str(s);
                    while let Some(pos) = self.json_buf.find('\n') {
                        let mut line = self.json_buf[..pos].to_string();
                        self.json_buf.drain(..=pos);
                        if line.ends_with('\r') {
                            line.pop();
                        }
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        // Ignore SSE "data:"/etc lines if present.
                        if !(line.starts_with('{') || line.starts_with('[')) {
                            continue;
                        }
                        if let Some(item) = decode_json_line(self.proto, line) {
                            out.push(item);
                        }
                    }
                }
            }
        }

        out
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        for ev in self.sse.finish() {
            if let Some(item) = decode_sse_event(self.proto, &ev) {
                out.push(item);
            }
        }
        if self.format == StreamFormat::JsonStream {
            let line = self.json_buf.trim();
            if !line.is_empty()
                && (line.starts_with('{') || line.starts_with('['))
                && let Some(item) = decode_json_line(self.proto, line)
            {
                out.push(item);
            }
            self.json_buf.clear();
        }
        out
    }
}

pub fn encode_stream_event(dst_proto: Proto, event: &StreamEvent) -> Option<Bytes> {
    match (dst_proto, event) {
        (Proto::Claude, StreamEvent::Claude(ev)) => {
            let value = serde_json::to_value(ev).ok()?;
            let event_name = value
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let data = serde_json::to_string(ev).ok()?;
            Some(encode_sse(event_name.as_deref(), &data))
        }
        (Proto::OpenAIChat, StreamEvent::OpenAIChat(ev)) => {
            let data = serde_json::to_string(ev).ok()?;
            Some(encode_sse(None, &data))
        }
        (Proto::OpenAIResponse, StreamEvent::OpenAIResponse(ev)) => {
            let value = serde_json::to_value(ev).ok()?;
            let event_name = value
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let data = serde_json::to_string(ev).ok()?;
            Some(encode_sse(event_name.as_deref(), &data))
        }
        (Proto::Gemini, StreamEvent::Gemini(ev)) => {
            let mut data = serde_json::to_vec(ev).ok()?;
            data.push(b'\n');
            Some(Bytes::from(data))
        }
        _ => None,
    }
}

pub fn encode_openai_chat_done() -> Bytes {
    Bytes::from_static(b"data: [DONE]\n\n")
}

pub fn content_type_for_stream(proto: Proto) -> &'static str {
    match proto {
        Proto::Gemini => "application/json",
        _ => "text/event-stream",
    }
}

fn decode_sse_event(proto: Proto, ev: &SseEvent) -> Option<StreamEvent> {
    let data = ev.data.trim();
    if data.is_empty() {
        return None;
    }
    if data == "[DONE]" {
        return None;
    }

    match proto {
        Proto::Claude => serde_json::from_str(data).ok().map(StreamEvent::Claude),
        Proto::OpenAIChat => serde_json::from_str(data).ok().map(StreamEvent::OpenAIChat),
        Proto::OpenAIResponse => serde_json::from_str(data)
            .ok()
            .map(StreamEvent::OpenAIResponse),
        Proto::Gemini => serde_json::from_str(data).ok().map(StreamEvent::Gemini),
        Proto::OpenAI => None,
    }
}

fn decode_json_line(proto: Proto, line: &str) -> Option<StreamEvent> {
    match proto {
        Proto::Gemini => serde_json::from_str(line).ok().map(StreamEvent::Gemini),
        _ => None,
    }
}

fn encode_sse(event: Option<&str>, data: &str) -> Bytes {
    // Minimal SSE encoding: `event:` is optional. For multi-line data, each line gets `data:`.
    let mut out = String::new();
    if let Some(event) = event {
        out.push_str("event: ");
        out.push_str(event);
        out.push('\n');
    }
    for line in data.split('\n') {
        out.push_str("data: ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    Bytes::from(out)
}
