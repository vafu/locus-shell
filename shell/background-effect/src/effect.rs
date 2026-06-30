use std::{env, error::Error, ffi::c_void, fmt::Write as _, time::Duration, time::Instant};

use gtk::{
    gdk,
    glib::{self, translate::ToGlibPtr},
    prelude::{
        Cast, DisplayExtManual, IsA, NativeExt, ObjectExt, SurfaceExt, WidgetExt, WidgetExtManual,
    },
};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    backend::{Backend, ObjectId},
    delegate_noop,
    globals::{BindError, GlobalList, GlobalListContents, registry_queue_init},
    protocol::{wl_compositor, wl_region, wl_registry, wl_surface},
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};

use crate::{
    BackgroundEffect, BackgroundEffectRegion,
    region::{RegionRectangle, RegionShape, RegionSize, append_region_rectangles},
};

const WINDOW_DATA_KEY: &str = "gtk4-background-effect";
const TRACE_ENV: &str = "GTK4_BACKGROUND_EFFECT_TRACE";
const LEGACY_TRACE_ENV: &str = "SHELL_CORE_BACKGROUND_EFFECT_TRACE";

unsafe extern "C" {
    fn gdk_wayland_display_get_wl_display(display: *mut gdk::ffi::GdkDisplay) -> *mut c_void;
    fn gdk_wayland_surface_get_wl_surface(surface: *mut gdk::ffi::GdkSurface) -> *mut c_void;
    fn gdk_wayland_surface_force_next_commit(surface: *mut gdk::ffi::GdkSurface);
}

/// Apply a compositor background effect to a GTK window.
///
/// This is a Wayland-only helper. It no-ops when GTK is not using the Wayland
/// backend, when the compositor does not advertise `ext-background-effect-v1`,
/// or when the window does not have a realized surface yet.
pub fn apply_background_effect(
    window: &impl IsA<gtk::Window>,
    background_effect: BackgroundEffect,
) {
    match background_effect {
        BackgroundEffect::None => {}
        BackgroundEffect::Blur(region) => enable_background_blur(window.as_ref(), region),
    }
}

fn enable_background_blur(window: &gtk::Window, region: BackgroundEffectRegion) {
    window.connect_map(move |window| {
        if let Err(error) = install_background_blur(window, region) {
            eprintln!("[gtk4-background-effect] failed to enable blur: {error}");
        }
    });
    window.connect_unrealize(|window| unsafe {
        window.steal_data::<BackgroundEffectHandle>(WINDOW_DATA_KEY);
    });

    if window.surface().is_some()
        && let Err(error) = install_background_blur(window, region)
    {
        eprintln!("[gtk4-background-effect] failed to enable blur: {error}");
    }
}

fn install_background_blur(
    window: &impl IsA<gtk::Window>,
    region: BackgroundEffectRegion,
) -> Result<(), Box<dyn Error>> {
    let window = window.as_ref();
    if unsafe {
        window
            .data::<BackgroundEffectHandle>(WINDOW_DATA_KEY)
            .is_some()
    } {
        return Ok(());
    }

    let display = window.display();
    if !display.backend().is_wayland() {
        return Ok(());
    }

    let Some(gdk_surface) = window.surface() else {
        return Ok(());
    };

    let wl_display = wayland_display(&display);
    let wl_surface = wayland_surface(&gdk_surface);
    if wl_display.is_null() || wl_surface.is_null() {
        return Ok(());
    }

    let backend = unsafe { Backend::from_foreign_display(wl_display.cast()) };
    let connection = Connection::from_backend(backend);
    let (globals, event_queue) = registry_queue_init::<BackgroundEffectState>(&connection)?;
    let queue_handle = event_queue.handle();

    let manager = match globals.bind::<ExtBackgroundEffectManagerV1, _, _>(&queue_handle, 1..=1, ())
    {
        Ok(manager) => manager,
        Err(BindError::NotPresent) => return Ok(()),
        Err(error) => return Err(Box::new(error)),
    };
    let compositor = globals.bind::<wl_compositor::WlCompositor, _, _>(&queue_handle, 1..=6, ())?;
    let surface_id =
        unsafe { ObjectId::from_ptr(wl_surface::WlSurface::interface(), wl_surface.cast()) }?;
    let wl_surface = wl_surface::WlSurface::from_id(&connection, surface_id)?;
    let effect_surface = manager.get_background_effect(&wl_surface, &queue_handle, ());

    let mut handle = BackgroundEffectHandle::new(
        connection,
        event_queue,
        globals,
        manager,
        compositor,
        effect_surface,
        region,
    );
    handle.update_blur_region(window, &gdk_surface)?;
    unsafe {
        window.set_data(WINDOW_DATA_KEY, handle);
    }
    install_dynamic_region_refresh(window, region);

    Ok(())
}

fn wayland_display(display: &gdk::Display) -> *mut c_void {
    unsafe { gdk_wayland_display_get_wl_display(display.to_glib_none().0) }
}

fn wayland_surface(surface: &gdk::Surface) -> *mut c_void {
    unsafe { gdk_wayland_surface_get_wl_surface(surface.to_glib_none().0) }
}

fn install_dynamic_region_refresh(window: &gtk::Window, region: BackgroundEffectRegion) {
    if !region.needs_layout_refresh() {
        return;
    }

    let tick_callback =
        window.add_tick_callback(|window, _| match update_installed_blur_region(window) {
            Ok(true) => glib::ControlFlow::Continue,
            Ok(false) => glib::ControlFlow::Break,
            Err(error) => {
                eprintln!("[gtk4-background-effect] failed to refresh blur region: {error}");
                glib::ControlFlow::Break
            }
        });

    unsafe {
        let Some(mut handle) = window.data::<BackgroundEffectHandle>(WINDOW_DATA_KEY) else {
            tick_callback.remove();
            return;
        };
        handle.as_mut().tick_callback = Some(tick_callback);
    }
}

fn update_installed_blur_region(window: &gtk::Window) -> Result<bool, Box<dyn Error>> {
    let Some(gdk_surface) = window.surface() else {
        return Ok(false);
    };

    unsafe {
        let Some(mut handle) = window.data::<BackgroundEffectHandle>(WINDOW_DATA_KEY) else {
            return Ok(false);
        };
        handle.as_mut().update_blur_region(window, &gdk_surface)?;
    }

    Ok(true)
}

fn blur_region_rectangles(
    window: &gtk::Window,
    surface: &gdk::Surface,
    region: BackgroundEffectRegion,
) -> Vec<RegionRectangle> {
    let surface_size = RegionSize {
        width: surface.width().max(window.width()).max(1),
        height: surface.height().max(window.height()).max(1),
    };

    let mut rectangles = Vec::new();
    append_blur_region_rectangles(window, surface_size, region, &mut rectangles);
    rectangles
}

fn append_blur_region_rectangles(
    window: &gtk::Window,
    surface_size: RegionSize,
    region: BackgroundEffectRegion,
    rectangles: &mut Vec<RegionRectangle>,
) {
    match region {
        BackgroundEffectRegion::Surface => {
            rectangles.push(RegionRectangle {
                x: 0,
                y: 0,
                width: surface_size.width,
                height: surface_size.height,
            });
        }
        BackgroundEffectRegion::CssClasses(classes) => {
            collect_blur_region_rectangles_for_css_classes(
                window,
                surface_size,
                classes,
                RegionShape::Rectangle,
                rectangles,
            )
        }
        BackgroundEffectRegion::RoundedCssClasses { classes, radius } => {
            collect_blur_region_rectangles_for_css_classes(
                window,
                surface_size,
                classes,
                RegionShape::Rounded {
                    radius,
                    inset: 0,
                    corner_guard: 0,
                },
                rectangles,
            )
        }
        BackgroundEffectRegion::CornerGuardRoundedCssClasses {
            classes,
            radius,
            corner_guard,
        } => collect_blur_region_rectangles_for_css_classes(
            window,
            surface_size,
            classes,
            RegionShape::Rounded {
                radius,
                inset: 0,
                corner_guard,
            },
            rectangles,
        ),
        BackgroundEffectRegion::InsetRoundedCssClasses {
            classes,
            radius,
            inset,
        } => collect_blur_region_rectangles_for_css_classes(
            window,
            surface_size,
            classes,
            RegionShape::Rounded {
                radius,
                inset,
                corner_guard: 0,
            },
            rectangles,
        ),
        BackgroundEffectRegion::Regions(regions) => {
            for region in regions {
                append_blur_region_rectangles(window, surface_size, *region, rectangles);
            }
        }
    }
}

fn collect_blur_region_rectangles_for_css_classes(
    window: &gtk::Window,
    surface_size: RegionSize,
    classes: &[&str],
    shape: RegionShape,
    rectangles: &mut Vec<RegionRectangle>,
) {
    let root = window.upcast_ref::<gtk::Widget>();
    collect_css_class_rectangles(root, root, surface_size, classes, shape, rectangles);
}

fn collect_css_class_rectangles(
    widget: &gtk::Widget,
    root: &gtk::Widget,
    surface_size: RegionSize,
    classes: &[&str],
    shape: RegionShape,
    rectangles: &mut Vec<RegionRectangle>,
) {
    if widget.is_drawable()
        && classes
            .iter()
            .any(|css_class| widget.has_css_class(css_class))
        && let Some(bounds) = widget.compute_bounds(root)
        && let Some(rectangle) = RegionRectangle::from_bounds(&bounds, surface_size)
    {
        append_region_rectangles(rectangle, shape, rectangles);
    }

    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        collect_css_class_rectangles(&widget, root, surface_size, classes, shape, rectangles);
    }
}

fn force_next_surface_commit(surface: &gdk::Surface) {
    unsafe {
        gdk_wayland_surface_force_next_commit(surface.to_glib_none().0);
    }
    surface.queue_render();
}

#[derive(Debug)]
struct BackgroundEffectState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BackgroundEffectState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(BackgroundEffectState: wl_compositor::WlCompositor);
delegate_noop!(BackgroundEffectState: wl_region::WlRegion);
delegate_noop!(BackgroundEffectState: ignore ExtBackgroundEffectManagerV1);
delegate_noop!(BackgroundEffectState: ExtBackgroundEffectSurfaceV1);

struct BackgroundEffectHandle {
    connection: Connection,
    event_queue: EventQueue<BackgroundEffectState>,
    globals: Option<GlobalList>,
    manager: Option<ExtBackgroundEffectManagerV1>,
    compositor: wl_compositor::WlCompositor,
    surface: Option<ExtBackgroundEffectSurfaceV1>,
    region: BackgroundEffectRegion,
    last_rectangles: Option<Vec<RegionRectangle>>,
    tick_callback: Option<gtk::TickCallbackId>,
}

impl BackgroundEffectHandle {
    fn new(
        connection: Connection,
        event_queue: EventQueue<BackgroundEffectState>,
        globals: GlobalList,
        manager: ExtBackgroundEffectManagerV1,
        compositor: wl_compositor::WlCompositor,
        surface: ExtBackgroundEffectSurfaceV1,
        region: BackgroundEffectRegion,
    ) -> Self {
        Self {
            connection,
            event_queue,
            globals: Some(globals),
            manager: Some(manager),
            compositor,
            surface: Some(surface),
            region,
            last_rectangles: None,
            tick_callback: None,
        }
    }

    fn update_blur_region(
        &mut self,
        window: &gtk::Window,
        gdk_surface: &gdk::Surface,
    ) -> Result<(), Box<dyn Error>> {
        let trace_mode = TraceMode::from_env();
        let generation_started_at = (trace_mode != TraceMode::Off).then(Instant::now);
        let rectangles = blur_region_rectangles(window, gdk_surface, self.region);
        let generation_elapsed = generation_started_at.map(|started_at| started_at.elapsed());
        if self.last_rectangles.as_ref() == Some(&rectangles) {
            if trace_mode == TraceMode::All {
                trace_blur_region("unchanged", &rectangles, generation_elapsed, None);
            }
            return Ok(());
        }

        let apply_started_at = (trace_mode != TraceMode::Off).then(Instant::now);
        let queue_handle = self.event_queue.handle();
        let region = self.compositor.create_region(&queue_handle, ());
        for rectangle in &rectangles {
            region.add(rectangle.x, rectangle.y, rectangle.width, rectangle.height);
        }

        if let Some(surface) = self.surface.as_ref() {
            surface.set_blur_region(Some(&region));
        }
        region.destroy();
        force_next_surface_commit(gdk_surface);
        self.connection.flush()?;
        let apply_elapsed = apply_started_at.map(|started_at| started_at.elapsed());
        trace_blur_region("changed", &rectangles, generation_elapsed, apply_elapsed);
        self.last_rectangles = Some(rectangles);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TraceMode {
    Off,
    Changed,
    All,
}

impl TraceMode {
    fn from_env() -> Self {
        let value = env::var(TRACE_ENV).or_else(|_| env::var(LEGACY_TRACE_ENV));
        match value {
            Ok(value) if value == "all" => Self::All,
            Ok(_) => Self::Changed,
            Err(_) => Self::Off,
        }
    }
}

fn trace_blur_region(
    state: &str,
    rectangles: &[RegionRectangle],
    generation_elapsed: Option<Duration>,
    apply_elapsed: Option<Duration>,
) {
    let Some(generation_elapsed) = generation_elapsed else {
        return;
    };

    let area: i64 = rectangles
        .iter()
        .map(|rectangle| i64::from(rectangle.width) * i64::from(rectangle.height))
        .sum();
    let apply_us = apply_elapsed
        .map(|duration| duration.as_micros().to_string())
        .unwrap_or_else(|| "-".to_owned());
    let bounds = region_bounds(rectangles)
        .map(|bounds| {
            format!(
                "{}:{} {}x{}",
                bounds.x, bounds.y, bounds.width, bounds.height
            )
        })
        .unwrap_or_else(|| "empty".to_owned());
    let sample = rectangle_sample(rectangles);

    eprintln!(
        "[gtk4-background-effect] blur region {state}: rectangles={}, area={}px, bounds={}, sample=[{}], generate={}us, apply={}us",
        rectangles.len(),
        area,
        bounds,
        sample,
        generation_elapsed.as_micros(),
        apply_us,
    );
}

fn region_bounds(rectangles: &[RegionRectangle]) -> Option<RegionRectangle> {
    let first = rectangles.first()?;
    let mut left = first.x;
    let mut top = first.y;
    let mut right = first.x + first.width;
    let mut bottom = first.y + first.height;

    for rectangle in &rectangles[1..] {
        left = left.min(rectangle.x);
        top = top.min(rectangle.y);
        right = right.max(rectangle.x + rectangle.width);
        bottom = bottom.max(rectangle.y + rectangle.height);
    }

    Some(RegionRectangle {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn rectangle_sample(rectangles: &[RegionRectangle]) -> String {
    const MAX_SAMPLE_RECTANGLES: usize = 12;

    let mut sample = String::new();
    for (index, rectangle) in rectangles.iter().take(MAX_SAMPLE_RECTANGLES).enumerate() {
        if index > 0 {
            sample.push_str(", ");
        }
        let _ = write!(
            sample,
            "{}:{} {}x{}",
            rectangle.x, rectangle.y, rectangle.width, rectangle.height
        );
    }
    if rectangles.len() > MAX_SAMPLE_RECTANGLES {
        let _ = write!(sample, ", +{}", rectangles.len() - MAX_SAMPLE_RECTANGLES);
    }

    sample
}

impl Drop for BackgroundEffectHandle {
    fn drop(&mut self) {
        if let Some(tick_callback) = self.tick_callback.take() {
            tick_callback.remove();
        }

        if let Some(surface) = self.surface.take() {
            surface.destroy();
        }

        if let Some(manager) = self.manager.take() {
            manager.destroy();
        }

        if let Some(globals) = self.globals.take() {
            globals.destroy();
        }

        let _ = self.connection.flush();
    }
}
