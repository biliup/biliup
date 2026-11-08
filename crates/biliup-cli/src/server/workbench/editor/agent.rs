//! Editor assistant. Provider calls, planning and commits share the existing editor services.
mod analysis;
mod brand;
mod brand_reference;
pub(crate) mod capture;
pub(crate) mod cards;
mod cleanup;
mod create;
mod downloads;
mod exports;
mod library;
mod motion;
pub(crate) mod plan;
pub(crate) mod planner;
mod research;
pub(crate) mod review;
pub(crate) mod service;
mod shaders;
mod speech;
pub(crate) mod store;
mod subtitles;
pub(crate) mod views;
mod web_search;

#[derive(Clone, Copy)]
struct Turn<'a> {
    project: i64,
    id: &'a str,
    cancel: &'a tokio_util::sync::CancellationToken,
}
