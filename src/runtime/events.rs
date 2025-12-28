//! Events API - Node.js compatible events module
//!
//! Full implementation of Node.js EventEmitter class.
//! Provides:
//! - EventEmitter class with all standard methods
//! - events.once() for promise-based single event
//! - events.on() for async iteration
//! - events.getEventListeners()
//! - events.setMaxListeners()
//! - events.defaultMaxListeners
//! - events.errorMonitor symbol
//! - events.captureRejections

use boa_engine::{Context, JsResult, Source};

/// Register the events module
pub fn register_events_module(context: &mut Context) -> JsResult<()> {
    let events_module_code = include_str!("events_module.js");
    let source = Source::from_bytes(events_module_code.as_bytes());
    context.eval(source)?;
    Ok(())
}
