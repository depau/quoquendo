//! Playing speech and saving it: rodio and a file dialog on the desktop, Web
//! Audio and a download in the browser.

use quoquendo::export::{encode, Format, Tagged};
use quoquendo::synth::Speech;

/// What was saved: the file's name and its size in bytes.
pub struct Saved {
    pub name: String,
    pub bytes: usize,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Player {
    /// opened on the first play and kept, so switching tracks starts at once
    out: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,
    sink: Option<rodio::Sink>,
    /// where in its samples the playing track started (s)
    offset: f64,
}

#[cfg(not(target_arch = "wasm32"))]
impl Player {
    pub fn new() -> Player {
        Player { out: None, sink: None, offset: 0.0 }
    }

    /// Play `samples` from `from` seconds in.
    pub fn play(&mut self, samples: &[f32], rate: u32, from: f64) -> Result<(), String> {
        self.stop();
        if self.out.is_none() {
            self.out = Some(rodio::OutputStream::try_default().map_err(|e| format!("No sound device found ({e})"))?);
        }
        let (_, handle) = self.out.as_ref().unwrap();
        let sink = rodio::Sink::try_new(handle).map_err(|e| format!("audio output: {e}"))?;
        let start = ((from.max(0.0) * rate as f64) as usize).min(samples.len());
        sink.append(rodio::buffer::SamplesBuffer::new(1, rate, samples[start..].to_vec()));
        sink.play();
        self.offset = start as f64 / rate as f64;
        self.sink = Some(sink);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }

    /// Hold the track where it is; `position` stays there.
    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    /// Go on from where `pause` held it.
    pub fn resume(&mut self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.as_ref().is_some_and(|s| s.is_paused() && !s.empty())
    }

    /// Where the playing or paused track is (s), None when nothing plays.
    pub fn position(&self) -> Option<f64> {
        let sink = self.sink.as_ref().filter(|s| !s.empty())?;
        Some(self.offset + sink.get_pos().as_secs_f64())
    }
}

/// Ask where to save, then write the speech there.
#[cfg(not(target_arch = "wasm32"))]
pub fn save(speech: &Speech, format: Format, tags: &Tagged) -> Result<Option<Saved>, String> {
    let ext = format.ext();
    let Some(path) = rfd::FileDialog::new().set_file_name(format!("quoquendo.{ext}")).add_filter(format.label(), &[ext]).save_file() else {
        return Ok(None);
    };
    let bytes = encode(speech, format, Some(tags))?;
    std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().to_string());
    Ok(Some(Saved { name, bytes: bytes.len() }))
}

#[cfg(target_arch = "wasm32")]
pub struct Player {
    ctx: Option<web_sys::AudioContext>,
    /// the track, kept to go on after a pause
    buffer: Option<web_sys::AudioBuffer>,
    source: Option<web_sys::AudioBufferSourceNode>,
    /// the context's time when the track's sample `offset` played
    started_at: f64,
    offset: f64,
    duration: f64,
    /// where a paused track was held (s)
    paused_at: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
impl Player {
    pub fn new() -> Player {
        Player {
            ctx: None,
            buffer: None,
            source: None,
            started_at: 0.0,
            offset: 0.0,
            duration: 0.0,
            paused_at: None,
        }
    }

    /// Play `samples` from `from` seconds in.
    pub fn play(&mut self, samples: &[f32], rate: u32, from: f64) -> Result<(), String> {
        let js = |e: wasm_bindgen::JsValue| format!("audio: {e:?}");
        self.stop();
        // made on the first Speak, a user gesture, so the browser lets it play
        if self.ctx.is_none() {
            self.ctx = Some(web_sys::AudioContext::new().map_err(js)?);
        }
        let ctx = self.ctx.as_ref().unwrap();
        let _ = ctx.resume();
        let n = samples.len().max(1) as u32;
        let buffer = ctx.create_buffer(1, n, rate as f32).map_err(js)?;
        let mut samples = samples.to_vec();
        samples.resize(n as usize, 0.0);
        buffer.copy_to_channel(&samples, 0).map_err(js)?;
        self.duration = n as f64 / rate as f64;
        self.buffer = Some(buffer);
        self.start(from)
    }

    /// Play the kept track from `from` seconds in.
    fn start(&mut self, from: f64) -> Result<(), String> {
        let js = |e: wasm_bindgen::JsValue| format!("audio: {e:?}");
        let (Some(ctx), Some(buffer)) = (&self.ctx, &self.buffer) else { return Ok(()) };
        let source = ctx.create_buffer_source().map_err(js)?;
        source.set_buffer(Some(buffer));
        source.connect_with_audio_node(&ctx.destination()).map_err(js)?;
        self.offset = from.clamp(0.0, self.duration);
        source.start_with_when_and_grain_offset(0.0, self.offset).map_err(js)?;
        self.started_at = ctx.current_time();
        self.source = Some(source);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.paused_at = None;
        if let Some(s) = self.source.take() {
            #[allow(deprecated)]
            let _ = s.stop();
        }
    }

    /// Hold the track where it is; `position` stays there. The source is
    /// stopped and a new one starts at that moment on `resume`, so the
    /// position is exact whatever the output's latency.
    pub fn pause(&mut self) {
        if let Some(at) = self.position().filter(|_| self.paused_at.is_none()) {
            if let Some(s) = self.source.take() {
                #[allow(deprecated)]
                let _ = s.stop();
            }
            self.paused_at = Some(at);
        }
    }

    /// Go on from where `pause` held it.
    pub fn resume(&mut self) {
        if let Some(at) = self.paused_at.take() {
            let _ = self.start(at);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// Where the playing or paused track is (s), None when nothing plays.
    pub fn position(&self) -> Option<f64> {
        if self.paused_at.is_some() {
            return self.paused_at;
        }
        let ctx = self.ctx.as_ref()?;
        self.source.as_ref()?;
        let at = self.offset + (ctx.current_time() - self.started_at).max(0.0);
        (at < self.duration).then_some(at)
    }
}

/// Download the speech (`quoquendo.<ext>`) through a link the page clicks.
#[cfg(target_arch = "wasm32")]
pub fn save(speech: &Speech, format: Format, tags: &Tagged) -> Result<Option<Saved>, String> {
    use wasm_bindgen::JsCast;
    let js = |e: wasm_bindgen::JsValue| format!("save: {e:?}");
    let bytes = encode(speech, format, Some(tags))?;
    let array = js_sys::Uint8Array::from(bytes.as_slice());
    let parts = js_sys::Array::of1(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(format.mime());
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(js)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js)?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(js)?.dyn_into().map_err(|_| "no anchor")?;
    let name = format!("quoquendo.{}", format.ext());
    a.set_href(&url);
    a.set_download(&name);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(Some(Saved { name, bytes: bytes.len() }))
}
