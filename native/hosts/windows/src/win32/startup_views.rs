//! Startup Lab actions, canvas composition, and ambient invalidation.
//!
//! This module renders only host-selected views. Linked tiles map to fixed
//! host actions; no application-provided path, handle, or native command can
//! enter the rendering or launch flow.

use super::*;

/// Starts one product session for the Startup Lab's launch tile.
///
/// The blocking verification and launch run on a worker; this returns
/// immediately so the message loop keeps pumping.
pub(super) fn begin_product_session(window: Hwnd) {
    product_tile::request_start(move || {
        // SAFETY: posting to a window this process created is safe from any
        // thread, and the message carries no pointer or payload.
        let posted = unsafe { PostMessageW(window, WM_ANODREL_PRODUCT_SESSION, 0, 0) };
        if posted == 0 {
            // The surface closed while this session was starting, so nothing
            // will ever collect it. Ending it here is what stops a verified
            // child from outliving the host.
            product_tile::discard();
        }
    });
}

/// Builds the document behind a linked action tile.
pub(super) fn action_document(
    action: startup_lab::ActionKind,
    lab: &StartupLab,
) -> Option<(String, Document)> {
    match action {
        startup_lab::ActionKind::OpenLogs => Some((
            "Anodrel - Runtime Logs".to_owned(),
            Document {
                title: "Runtime Logs".to_owned(),
                subtitle: "Typed events for this process".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "STARTUP EVENTS".to_owned(),
                        rows: lab
                            .log
                            .entries()
                            .map(|entry| {
                                (
                                    format!("#{:04}", entry.sequence()),
                                    format!(
                                        "{} | {} | {}",
                                        entry.level().label(),
                                        entry.component(),
                                        entry.message()
                                    ),
                                )
                            })
                            .collect(),
                    },
                    Section {
                        heading: "BOUNDARY".to_owned(),
                        rows: vec![
                            (
                                "Retention".to_owned(),
                                "64 in-memory events; oldest entries drop first".to_owned(),
                            ),
                            ("Application input".to_owned(), "not accepted".to_owned()),
                            (
                                "Persistence or export".to_owned(),
                                "not available".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::InspectPackage => Some((
            "Anodrel - Inspect Package".to_owned(),
            Document {
                title: "Inspect Package".to_owned(),
                subtitle: "Facts verified before this surface opened".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "IDENTITY".to_owned(),
                        rows: vec![
                            ("Display name".to_owned(), lab.package.display_name.clone()),
                            (
                                "Application ID".to_owned(),
                                lab.package.application_id.clone(),
                            ),
                        ],
                    },
                    Section {
                        heading: "CONTENT".to_owned(),
                        rows: vec![
                            ("Format".to_owned(), lab.package.content_format.clone()),
                            ("Path".to_owned(), lab.package.content_path.clone()),
                            ("Bytes".to_owned(), lab.package.content_bytes.to_string()),
                            ("SHA-256".to_owned(), lab.package.content_digest.clone()),
                        ],
                    },
                    Section {
                        heading: "LIMITS".to_owned(),
                        rows: vec![
                            (
                                "Max manifest".to_owned(),
                                format!("{} bytes", anodrel_application::MAX_MANIFEST_BYTES),
                            ),
                            (
                                "Max content".to_owned(),
                                format!("{} bytes", anodrel_application::MAX_CONTENT_BYTES),
                            ),
                            (
                                "Publisher trust".to_owned(),
                                "not verified - see ROADMAP".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::RuntimeDiagnostics => Some((
            "Anodrel - Runtime Diagnostics".to_owned(),
            Document {
                title: "Runtime Diagnostics".to_owned(),
                subtitle: "Protocol, transport, and renderer state".to_owned(),
                body: Body::Sections(vec![
                    Section {
                        heading: "PROTOCOL".to_owned(),
                        rows: vec![
                            (
                                "Version".to_owned(),
                                format!(
                                    "{}.{}",
                                    anodrel_protocol::PROTOCOL_MAJOR,
                                    anodrel_protocol::PROTOCOL_MINOR
                                ),
                            ),
                            (
                                "Max request".to_owned(),
                                format!("{} bytes", anodrel_core::MAX_REQUEST_BYTES),
                            ),
                            (
                                "JSON depth".to_owned(),
                                anodrel_json::DEFAULT_MAX_DEPTH.to_string(),
                            ),
                        ],
                    },
                    Section {
                        heading: "TRANSPORT".to_owned(),
                        rows: vec![
                            (
                                "Frame magic".to_owned(),
                                String::from_utf8_lossy(&anodrel_wire::MAGIC).into_owned(),
                            ),
                            (
                                "Max payload".to_owned(),
                                format!("{} bytes", anodrel_wire::MAX_PAYLOAD_BYTES),
                            ),
                            (
                                "Frames per read".to_owned(),
                                anodrel_wire::MAX_FRAMES_PER_RECEIVE.to_string(),
                            ),
                            ("Pipe scope".to_owned(), "current logon session".to_owned()),
                        ],
                    },
                    Section {
                        heading: "PROCESS".to_owned(),
                        rows: vec![
                            (
                                "Working set".to_owned(),
                                format!(
                                    "{:.1} MB",
                                    stats::memory_readings().working_set_bytes as f32
                                        / (1024.0 * 1024.0)
                                ),
                            ),
                            (
                                // Reported beside the working set because the
                                // two answer different questions, and only this
                                // one adds up across a process tree.
                                "Private bytes".to_owned(),
                                format!(
                                    "{:.1} MB",
                                    stats::memory_readings().private_bytes as f32
                                        / (1024.0 * 1024.0)
                                ),
                            ),
                            ("Startup".to_owned(), format!("{} ms", lab.startup_millis)),
                            (
                                "Last frame".to_owned(),
                                format!("{:.2} ms", lab.last_frame_micros as f32 / 1000.0),
                            ),
                            (
                                "Runtime dependencies".to_owned(),
                                "0 third-party crates".to_owned(),
                            ),
                        ],
                    },
                ]),
            },
        )),
        startup_lab::ActionKind::LaunchDevelopmentFixture => None,
    }
}

thread_local! {
    /// The animated surface, retained between paints.
    ///
    /// Repainting only a region requires the rest of the previous frame to
    /// still be there, so the animated window keeps its canvas rather than
    /// composing a fresh one each time. Document windows always redraw whole
    /// and need no such state.
    static SURFACE: std::cell::RefCell<Option<(Hwnd, Canvas)>> =
        const { std::cell::RefCell::new(None) };
}

/// Returns `true` when `inner` lies entirely within `outer`.
pub(super) fn region_covers(outer: CanvasRect, inner: Rect) -> bool {
    (inner.left as f32) >= outer.left.floor()
        && (inner.top as f32) >= outer.top.floor()
        && (inner.right as f32) <= outer.right.ceil()
        && (inner.bottom as f32) <= outer.bottom.ceil()
}

/// Paints a window's view and presents it.
///
/// `update` is the rectangle Windows asked to be repainted. When it falls
/// inside the animated region and the surface has settled, only that region is
/// recomposed and sent; anything else redraws the whole surface.
///
/// Returns the time the frame took, which the Startup Lab reports on the next
/// frame.
pub(super) fn paint(window: Hwnd, device_context: Hdc, view: &View, update: Rect) -> u64 {
    let started = Instant::now();
    let rect = client_rect(window);
    let width = rect.width().max(1) as u32;
    let height = rect.height().max(1) as u32;

    match view {
        View::Document(document) => {
            let mut canvas = Canvas::new(width, height);
            document::draw(&mut canvas, document);
            present::present(device_context, &canvas);
        }
        View::StartupLab(lab) => {
            let elapsed = lab.revealed_at.elapsed().as_millis() as u64;
            SURFACE.with(|cell| {
                let mut slot = cell.borrow_mut();
                let reusable = slot.as_ref().is_some_and(|(owner, canvas)| {
                    *owner == window && canvas.width() == width && canvas.height() == height
                });
                if !reusable {
                    // A new size invalidates every cached layer, including the
                    // backdrop and the pre-composed hero.
                    startup_lab::invalidate_caches();
                    *slot = Some((window, Canvas::new(width, height)));
                }
                let Some((_, canvas)) = slot.as_mut() else {
                    return;
                };

                let region = (elapsed >= startup_lab::REVEAL_MILLIS)
                    .then(|| startup_lab::ambient_region(width as f32, height as f32))
                    .flatten()
                    .filter(|region| reusable && region_covers(*region, update));

                match region {
                    Some(region) if startup_lab::draw_ambient(canvas, lab, elapsed) => {
                        present::present_region(
                            device_context,
                            canvas,
                            region.left.floor().max(0.0) as u32,
                            region.top.floor().max(0.0) as u32,
                            region.width().ceil() as u32,
                            region.height().ceil() as u32,
                        );
                    }
                    _ => {
                        startup_lab::draw(canvas, lab, elapsed);
                        present::present(device_context, canvas);
                    }
                }
            });
        }
        View::UiLab(lab) => {
            let mut canvas = Canvas::new(width, height);
            ui_lab::draw(&mut canvas, lab);
            present::present(device_context, &canvas);
        }
        View::UiSession(session) => {
            let mut canvas = Canvas::new(width, height);
            ui_lab::draw(&mut canvas, session.lab());
            present::present(device_context, &canvas);
        }
    }
    started.elapsed().as_micros() as u64
}

/// Starts or stops ambient motion for a settled surface.
///
/// Motion is suspended whenever the window cannot be seen or is not being
/// looked at. A background window must cost nothing.
pub(super) fn set_ambient_running(window: Hwnd, running: bool) {
    let settled = registry::with_startup_lab(window, |lab| lab.ambient)
        .ok()
        .flatten()
        .unwrap_or(false);
    if !settled {
        return;
    }
    // SAFETY: the window belongs to this process; setting a timer that already
    // exists resets it, and killing one that does not is a no-op.
    unsafe {
        if running {
            SetTimer(window, REVEAL_TIMER, AMBIENT_INTERVAL_MILLIS, 0);
        } else {
            KillTimer(window, REVEAL_TIMER);
        }
    }
}
