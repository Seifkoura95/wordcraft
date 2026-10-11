//! The browser shell: web `Services`, drag-and-drop, and the eframe web runner.

#[cfg(not(feature = "simplehub"))]
use wasm_bindgen::JsCast as _;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{Inbox, Services, WordApp};

#[cfg(not(feature = "simplehub"))]
const DOC_EXTS: &[&str] = &["docx", "docm", "dotx", "odt", "rtf", "txt", "md", "html", "htm", "json"];
#[cfg(not(feature = "simplehub"))]
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];
#[cfg(not(feature = "simplehub"))]
const CANVAS_ID: &str = "wordcraft_canvas";
#[cfg(not(feature = "simplehub"))]
const LOADING_ID: &str = "wordcraft_loading";

#[cfg(not(feature = "simplehub"))]
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            log::error!("no document");
            return;
        };
        let Some(canvas) = document.get_element_by_id(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
            log::error!("missing <canvas id=\"{CANVAS_ID}\">");
            return;
        };
        let mut options = eframe::WebOptions::default();
        if query().contains("webgl")
            && let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup
        {
            create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
        }
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(move |cc| {
                    if let Some(rs) = &cc.wgpu_render_state {
                        log::info!("wordcraft-web: wgpu backend {:?}", rs.adapter.get_info().backend);
                    }
                    let inbox: Inbox = Inbox::default();
                    let doc = if query().contains("sample") { wordcraft_engine::sample::sample_document() } else { wordcraft_doc::Document::new() };
                    let mut app = WordApp::new(Session::new(doc), services(inbox.clone(), cc.egui_ctx.clone()));
                    app.autosave = false;
                    Ok(Box::new(WebShell { app, inbox, theme: None }))
                }),
            )
            .await;
        if let Some(el) = document.get_element_by_id(LOADING_ID) {
            match result {
                Ok(()) => el.remove(),
                Err(e) => el.set_inner_html(&format!("<p>{} failed to start: {e:?}</p><p>A browser with WebGPU or WebGL2 is required.</p>", wordcraft_ui_egui::APP_NAME)),
            }
        }
    });
}

#[cfg(not(feature = "simplehub"))]
fn query() -> String {
    web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default()
}

/// Wraps the app to read dropped files asynchronously (browsers can't read them synchronously)
/// and feed them through the inbox.
struct WebShell {
    app: WordApp,
    inbox: Inbox,
    /// A theme change requested by the host (`true` = dark), applied on the next frame.
    theme: Option<ThemeRequest>,
}

/// Shared with the host-facing handle; `Some(dark)` until the app picks it up.
type ThemeRequest = std::rc::Rc<std::cell::Cell<Option<bool>>>;

impl eframe::App for WebShell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(dark) = self.theme.as_ref().and_then(|t| t.take()) {
            self.app.ui.dark = dark;
        }
        let dropped = ctx.input_mut(|i| std::mem::take(&mut i.raw.dropped_files));
        for f in dropped {
            let inbox = self.inbox.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let name = f.path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "dropped".into());
                match f.bytes_async().await {
                    Ok(bytes) => {
                        inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes));
                        ctx.request_repaint();
                    }
                    Err(e) => log::error!("couldn't read dropped file {name}: {e}"),
                }
            });
        }
        self.app.logic(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

#[cfg(not(feature = "simplehub"))]
fn services(inbox: Inbox, ctx: egui::Context) -> Services {
    let open_inbox = inbox.clone();
    Services {
        open_async: Some(Box::new(move |purpose: &str| {
            let inbox = open_inbox.clone();
            let ctx = ctx.clone();
            let dialog = if purpose == "picture" {
                rfd::AsyncFileDialog::new().add_filter("Pictures", IMAGE_EXTS)
            } else {
                rfd::AsyncFileDialog::new().add_filter("Documents", DOC_EXTS)
            };
            wasm_bindgen_futures::spawn_local(async move {
                let Some(file) = dialog.pick_file().await else {
                    return;
                };
                let bytes = file.read().await;
                inbox.lock().unwrap_or_else(|e| e.into_inner()).push((file.file_name(), bytes));
                ctx.request_repaint();
            });
        })),
        // Exports ask for a name; the browser decides where the download goes.
        pick_save: Some(Box::new(|name: &str| Some(name.to_string()))),
        download: Some(Box::new(|name: &str, bytes: &[u8]| {
            if let Err(e) = download(name, bytes) {
                log::error!("download of {name} failed: {e}");
            }
        })),
        inbox: Some(inbox),
        ..Default::default()
    }
}

#[cfg(not(feature = "simplehub"))]
/// Trigger a browser download of `bytes` named after the last component of `path`.
fn download(path: &str, bytes: &[u8]) -> Result<(), String> {
    let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "document".into());
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_for(&name));
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(js)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js)?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(js)?.dyn_into().map_err(|_| "not an anchor")?;
    a.set_href(&url);
    a.set_download(&name);
    a.style().set_property("display", "none").map_err(js)?;
    let body = document.body().ok_or("no body")?;
    body.append_child(&a).map_err(js)?;
    a.click();
    a.remove();
    // Revoke after the click has been dispatched; the download keeps its own reference.
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        web_sys::Url::revoke_object_url(&url).ok();
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000).map_err(js)?;
    Ok(())
}

#[cfg(not(feature = "simplehub"))]
fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("pdf") => "application/pdf",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("odt") => "application/vnd.oasis.opendocument.text",
        Some("rtf") => "application/rtf",
        Some("html") => "text/html",
        Some("md" | "txt") => "text/plain",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

/// Simple Write inside SimpleHub. The page mounts the app on a canvas it owns and hands over a
/// bridge object (`open(): Promise<{name, bytes}|null>`, `save(name, bytes): Promise<void>`,
/// optional `theme: "dark" | "light"`) that moves documents to and from Simple Cloud; nothing
/// here touches the browser's file picker.
#[cfg(feature = "simplehub")]
mod simplehub {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::{Inbox, Services, Session, ThemeRequest, WebShell, WordApp};
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    /// A running instance. SimpleHub calls `destroy` when the module unmounts.
    #[wasm_bindgen]
    pub struct WriteHandle {
        runner: eframe::WebRunner,
        theme: ThemeRequest,
        ctx: Rc<RefCell<Option<egui::Context>>>,
    }

    #[wasm_bindgen]
    impl WriteHandle {
        pub fn destroy(&self) {
            self.runner.destroy();
        }

        pub fn has_panicked(&self) -> bool {
            self.runner.has_panicked()
        }

        /// Switch the running app to `"dark"` or `"light"` on its next frame.
        pub fn set_theme(&self, theme: &str) {
            let Some(dark) = parse_theme(theme) else {
                log::warn!("SimpleHub: ignoring unknown theme {theme:?}");
                return;
            };
            self.theme.set(Some(dark));
            if let Some(ctx) = self.ctx.borrow().as_ref() {
                ctx.request_repaint();
            }
        }
    }

    /// `"dark"` → dark, `"light"` → light; anything else is ignored.
    fn parse_theme(theme: &str) -> Option<bool> {
        match theme.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(true),
            "light" => Some(false),
            _ => None,
        }
    }

    /// Starts Simple Write on `canvas`, with `bridge` providing open and save (and, optionally,
    /// the initial `theme`).
    #[wasm_bindgen]
    pub async fn start_simple_write(canvas: web_sys::HtmlCanvasElement, bridge: JsValue) -> Result<WriteHandle, JsValue> {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        let initial_dark = js_sys::Reflect::get(&bridge, &JsValue::from_str("theme")).ok().and_then(|v| v.as_string()).and_then(|s| parse_theme(&s));
        let theme: ThemeRequest = Rc::new(Cell::new(None));
        let ctx: Rc<RefCell<Option<egui::Context>>> = Rc::new(RefCell::new(None));
        let runner = eframe::WebRunner::new();
        let app_theme = theme.clone();
        let app_ctx = ctx.clone();
        runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    *app_ctx.borrow_mut() = Some(cc.egui_ctx.clone());
                    let inbox: Inbox = Inbox::default();
                    let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), services(bridge, inbox.clone(), cc.egui_ctx.clone()));
                    app.autosave = false;
                    // Applied by `WordApp::logic` before the first frame is painted.
                    if let Some(dark) = initial_dark {
                        app.ui.dark = dark;
                    }
                    Ok(Box::new(WebShell { app, inbox, theme: Some(app_theme) }))
                }),
            )
            .await?;
        Ok(WriteHandle { runner, theme, ctx })
    }

    fn services(bridge: JsValue, inbox: Inbox, ctx: egui::Context) -> Services {
        let open_bridge = bridge.clone();
        let open_inbox = inbox.clone();
        Services {
            // `purpose` is "document" or "picture"; it is passed to `open` so the host can filter.
            open_async: Some(Box::new(move |purpose: &str| {
                let bridge = open_bridge.clone();
                let inbox = open_inbox.clone();
                let ctx = ctx.clone();
                let purpose = purpose.to_string();
                wasm_bindgen_futures::spawn_local(async move {
                    match call_open(&bridge, &purpose).await {
                        Ok(Some((name, bytes))) => {
                            inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes));
                            ctx.request_repaint();
                        }
                        Ok(None) => {}
                        Err(e) => log::error!("SimpleHub open failed: {e:?}"),
                    }
                });
            })),
            // Save As and Export ask for a name; the bridge decides where it goes.
            pick_save: Some(Box::new(|name: &str| Some(name.to_string()))),
            download: Some(Box::new(move |name: &str, bytes: &[u8]| {
                let bridge = bridge.clone();
                let name = std::path::Path::new(name).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Document1.docx".into());
                let bytes = js_sys::Uint8Array::from(bytes);
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = call_save(&bridge, &name, &bytes).await {
                        log::error!("SimpleHub save of {name} failed: {e:?}");
                    }
                });
            })),
            inbox: Some(inbox),
            ..Default::default()
        }
    }

    fn method(bridge: &JsValue, name: &str) -> Result<js_sys::Function, JsValue> {
        js_sys::Reflect::get(bridge, &JsValue::from_str(name))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str(&format!("bridge.{name} is not a function")))
    }

    async fn call_open(bridge: &JsValue, purpose: &str) -> Result<Option<(String, Vec<u8>)>, JsValue> {
        let promise: js_sys::Promise = method(bridge, "open")?.call1(bridge, &JsValue::from_str(purpose))?.dyn_into()?;
        let result = JsFuture::from(promise).await?;
        if result.is_null() || result.is_undefined() {
            return Ok(None);
        }
        let name = js_sys::Reflect::get(&result, &JsValue::from_str("name"))?.as_string().unwrap_or_else(|| "Document1.docx".into());
        let bytes: js_sys::Uint8Array = js_sys::Reflect::get(&result, &JsValue::from_str("bytes"))?.dyn_into()?;
        Ok(Some((name, bytes.to_vec())))
    }

    async fn call_save(bridge: &JsValue, name: &str, bytes: &js_sys::Uint8Array) -> Result<(), JsValue> {
        let ret = method(bridge, "save")?.call2(bridge, &JsValue::from_str(name), bytes)?;
        if let Ok(promise) = ret.dyn_into::<js_sys::Promise>() {
            JsFuture::from(promise).await?;
        }
        Ok(())
    }
}
