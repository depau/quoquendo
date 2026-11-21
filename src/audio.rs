//! Playing speech and saving it: rodio and a file dialog on the desktop, Web
//! Audio and a download in the browser.

use quoquendo::synth::{self, Speech};

/// What Save writes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Wav,
    Mp3,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Wav => "wav",
            Format::Mp3 => "mp3",
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn mime(self) -> &'static str {
        match self {
            Format::Wav => "audio/wav",
            Format::Mp3 => "audio/mpeg",
        }
    }

    fn bytes(self, speech: &Speech) -> Result<Vec<u8>, String> {
        match self {
            Format::Wav => Ok(synth::to_wav_bytes(speech)),
            Format::Mp3 => synth::to_mp3_bytes(speech),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Player {
    out: Option<(rodio::OutputStream, rodio::Sink)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Player {
    pub fn new() -> Player {
        Player { out: None }
    }

    pub fn play(&mut self, speech: &Speech) -> Result<(), String> {
        self.stop();
        let (stream, handle) = rodio::OutputStream::try_default().map_err(|e| format!("no audio output device ({e}): use Save WAV instead"))?;
        let sink = rodio::Sink::try_new(&handle).map_err(|e| format!("audio output: {e}"))?;
        let samples: Vec<f32> = speech.samples.clone();
        sink.append(rodio::buffer::SamplesBuffer::new(1, speech.rate, samples));
        sink.play();
        self.out = Some((stream, sink));
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some((_, sink)) = self.out.take() {
            sink.stop();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.out.as_ref().is_some_and(|(_, s)| !s.empty())
    }
}

/// Ask where to save, then write the speech there. Returns the path written.
#[cfg(not(target_arch = "wasm32"))]
pub fn save(speech: &Speech, format: Format) -> Result<Option<String>, String> {
    let ext = format.ext();
    let Some(path) = rfd::FileDialog::new().set_file_name(format!("quoquendo.{ext}")).add_filter(ext.to_uppercase(), &[ext]).save_file() else {
        return Ok(None);
    };
    std::fs::write(&path, format.bytes(speech)?).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

#[cfg(target_arch = "wasm32")]
pub struct Player {
    ctx: Option<web_sys::AudioContext>,
    source: Option<web_sys::AudioBufferSourceNode>,
    ends_at: f64,
}

#[cfg(target_arch = "wasm32")]
impl Player {
    pub fn new() -> Player {
        Player {
            ctx: None,
            source: None,
            ends_at: 0.0,
        }
    }

    pub fn play(&mut self, speech: &Speech) -> Result<(), String> {
        let js = |e: wasm_bindgen::JsValue| format!("audio: {e:?}");
        self.stop();
        // made on the first Speak, a user gesture, so the browser lets it play
        if self.ctx.is_none() {
            self.ctx = Some(web_sys::AudioContext::new().map_err(js)?);
        }
        let ctx = self.ctx.as_ref().unwrap();
        let _ = ctx.resume();
        let n = speech.samples.len().max(1) as u32;
        let buffer = ctx.create_buffer(1, n, speech.rate as f32).map_err(js)?;
        let mut samples = speech.samples.clone();
        samples.resize(n as usize, 0.0);
        buffer.copy_to_channel(&samples, 0).map_err(js)?;
        let source = ctx.create_buffer_source().map_err(js)?;
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&ctx.destination()).map_err(js)?;
        source.start().map_err(js)?;
        self.ends_at = ctx.current_time() + n as f64 / speech.rate as f64;
        self.source = Some(source);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(s) = self.source.take() {
            #[allow(deprecated)]
            let _ = s.stop();
        }
    }

    pub fn is_playing(&self) -> bool {
        match (&self.ctx, &self.source) {
            (Some(ctx), Some(_)) => ctx.current_time() < self.ends_at,
            _ => false,
        }
    }
}

/// Download the speech (`quoquendo.wav` / `.mp3`) through a link the page
/// clicks.
#[cfg(target_arch = "wasm32")]
pub fn save(speech: &Speech, format: Format) -> Result<Option<String>, String> {
    use wasm_bindgen::JsCast;
    let js = |e: wasm_bindgen::JsValue| format!("save: {e:?}");
    let bytes = format.bytes(speech)?;
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
    Ok(Some(name))
}
