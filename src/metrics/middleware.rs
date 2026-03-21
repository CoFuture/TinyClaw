//! HTTP metrics middleware
//!
//! Axum middleware that records request timing and metrics for each HTTP endpoint.

use axum::{middleware::Next, response::Response};
use http::Request;
use std::time::Instant;

/// Create a metrics middleware function that records request timing
pub fn metrics_middleware(
    collector: std::sync::Arc<crate::metrics::MetricsCollector>,
) -> impl Fn(Request<axum::body::Body>, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request<axum::body::Body>, next: Next| {
        let collector = collector.clone();
        Box::pin(async move {
            let start = Instant::now();
            let method = req.method().to_string();
            let path = req.uri().path().to_string();

            // Call the next middleware/handler
            let response = next.run(req).await;

            // Record metrics
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let endpoint = format!("{} {}", method, path);
            
            // Check if response is an error (5xx)
            let is_error = response.status().is_server_error();
            
            collector.record_request(&endpoint, elapsed_ms, is_error);

            response
        })
    }
}
