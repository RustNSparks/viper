//! Stream API - Node.js compatible stream module
//!
//! Full implementation of Node.js streams including:
//! - Readable - Read data from a source
//! - Writable - Write data to a destination
//! - Duplex - Both readable and writable
//! - Transform - Modify data as it passes through
//! - PassThrough - Pass data through unchanged
//! - pipeline() - Connect streams together
//! - finished() - Get notified when stream is done

use boa_engine::{Context, JsResult, Source};

/// Register the stream module
pub fn register_stream_module(context: &mut Context) -> JsResult<()> {
    let stream_module_code = include_str!("stream_module.js");
    let source = Source::from_bytes(stream_module_code.as_bytes());
    context.eval(source)?;
    Ok(())
}
