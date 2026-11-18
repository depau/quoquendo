//! The browser build's installer unpacker, run as a Web Worker so the page
//! stays responsive while a large installer (Willem, 84 MB) is decompressed.
//! It receives the installer's bytes and posts back `{files: [[path,
//! Uint8Array], …]}` or `{error: "…"}`.

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    console_error_panic_hook::set_once();
    let scope: web_sys::DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let reply = scope.clone();
    let onmessage = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
        let bytes = js_sys::Uint8Array::new(&e.data()).to_vec();
        let msg = js_sys::Object::new();
        let transfer = js_sys::Array::new();
        match quoquendo::pack::unpack_installer(&bytes) {
            Ok(u) => {
                drop(bytes);
                let files = js_sys::Array::new();
                for (path, data) in u.files {
                    let a = js_sys::Uint8Array::from(data.as_slice());
                    transfer.push(&a.buffer());
                    files.push(&js_sys::Array::of2(&path.into(), &a));
                }
                let _ = js_sys::Reflect::set(&msg, &"files".into(), &files);
            }
            Err(err) => {
                let _ = js_sys::Reflect::set(&msg, &"error".into(), &err.into());
            }
        }
        let _ = reply.post_message_with_transfer(&msg, &transfer);
    });
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    let _ = scope.post_message(&"ready".into());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("qq_unpack_worker only runs in a browser, as the web app's Web Worker");
}
