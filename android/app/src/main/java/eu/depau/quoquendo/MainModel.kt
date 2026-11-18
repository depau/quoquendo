package eu.depau.quoquendo

import android.app.Application
import android.content.ContentValues
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.net.ConnectivityManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.os.SystemClock
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runInterruptible
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder

/** A catalog download: running, waiting its turn, or failed. */
sealed interface Download {
    data class Running(val done: Long, val total: Long, val bytesPerSecond: Double, val unpacking: Boolean) : Download
    data object Queued : Download
    data class Failed(val message: String) : Download
}

/** The import dialog: copying the picked file, then unpacking it. */
data class Import(val name: String, val done: Long, val total: Long, val unpacking: Boolean)

/** A snackbar to show, with a Retry action when [retry] is set. */
data class Snack(val message: String, val error: Boolean = false, val retry: (() -> Unit)? = null)

/** The status line under Speak. */
data class Status(val kind: Kind, val text: String) {
    enum class Kind { Ok, Busy, Info, Warn, Error }
}

/** What the analysis shows: sounds, recorded pieces and the waveform outline. */
data class Analysis(
    val voice: String,
    val text: String,
    val phrases: List<AnPhrase>,
    val plan: List<AnPlanned>,
    val pieces: List<AnPiece>,
    val sentences: Int,
    val audio: AnAudio?,
)
data class AnPhone(val sym: String, val known: Boolean, val wordStart: Boolean)
data class AnPhrase(val phones: List<AnPhone>, val pauseMs: Int, val mood: String?)
data class AnPlanned(val pauseMs: Int, val mood: String?, val missing: List<String>)
data class AnPiece(
    val phrase: Int, val covers: String, val unit: Int, val sentence: String,
    val from: Int, val to: Int, val atS: Double, val lenMs: Double, val pitch: Int, val hard: Boolean,
)
data class AnAudio(val rate: Int, val samples: Long, val seconds: Double, val envelope: FloatArray)

/** The app's state; its work lives as long as the app is open. */
class MainModel(app: Application) : AndroidViewModel(app) {
    private val prefs = app.getSharedPreferences("quoquendo", Context.MODE_PRIVATE)
    val store = VoiceStore(app)
    val catalog: List<Installer> = VoiceStore.catalog().sortedBy { it.label.lowercase() }

    var installed by mutableStateOf<List<InstalledVoice>>(emptyList())
        private set
    var bytesUsed by mutableStateOf(0L)
        private set

    // settings
    var mirror by mutableStateOf(prefs.getString("mirror_url", null) ?: BuildConfig.QQ_MIRROR_URL)
        private set
    var wifiOnly by mutableStateOf(prefs.getBoolean("wifi_only", true))
        private set
    var showCurtain by mutableStateOf(prefs.getBoolean("show_curtain", false))
        private set

    // voices
    val downloads = mutableStateMapOf<String, Download>()
    private val downloadJobs = HashMap<String, Job>()
    private val downloadTurn = Mutex()
    var importing by mutableStateOf<Import?>(null)
        private set
    private var importJob: Job? = null
    val snacks = Channel<Snack>(Channel.BUFFERED)

    // try it
    var voiceName by mutableStateOf(prefs.getString("voice", null))
        private set
    var text by mutableStateOf(prefs.getString("text", "") ?: "")
        private set
    var volume by mutableStateOf(prefs.getFloat("volume", 0.8f))
        private set
    var status by mutableStateOf<Status?>(null)
        private set
    var speaking by mutableStateOf(false)
        private set
    var playheadS by mutableStateOf<Double?>(null)
        private set
    var analysis by mutableStateOf<Analysis?>(null)
        private set
    var playingPiece by mutableStateOf<Int?>(null)
        private set
    var isDefaultEngine by mutableStateOf(true)
        private set
    var ttsReady by mutableStateOf(false)
        private set

    private val tts: TextToSpeech = TextToSpeech(app, { ok ->
        ttsReady = ok == TextToSpeech.SUCCESS
        refreshDefaultEngine()
    }, app.packageName)
    private var utterance = 0
    private var speakJob: Job? = null

    /** The voice loaded in this process for analysis, saving and piece playback. */
    private var handle = 0L
    private var handleVoice: String? = null
    private val handleLock = Any()
    private var pieceTrack: AudioTrack? = null

    init {
        tts.setOnUtteranceProgressListener(Progress())
        refresh()
    }

    // ---- library ----

    fun refresh() {
        viewModelScope.launch {
            try {
                val (list, used) = withContext(Dispatchers.IO) { store.installed() to store.bytesUsed() }
                installed = list
                bytesUsed = used
            } catch (e: Exception) {
                fail("Could not list voices: ${e.message}")
            }
        }
    }

    fun installerOf(v: InstalledVoice): Installer? = catalog.firstOrNull { it.voice.equals(v.name, ignoreCase = true) }

    /** `US English` for Allison: the catalog's language name, else the MotherTongue. */
    fun languageOf(v: InstalledVoice): String = installerOf(v)?.language ?: v.motherTongue

    fun installedFor(i: Installer): InstalledVoice? = installed.firstOrNull { it.name.equals(i.voice, ignoreCase = true) }

    private fun snack(message: String) {
        snacks.trySend(Snack(message))
    }

    private fun fail(message: String, retry: (() -> Unit)? = null) {
        snacks.trySend(Snack(message, error = true, retry = retry))
    }

    private fun blockedByWifi(): String? {
        if (!wifiOnly) return null
        val cm = getApplication<Application>().getSystemService(ConnectivityManager::class.java)
        return if (cm.isActiveNetworkMetered) "Download on Wi-Fi only is on, and this network is metered" else null
    }

    fun download(i: Installer) {
        if (downloadJobs[i.file]?.isActive == true) return
        downloads[i.file] = Download.Queued
        downloadJobs[i.file] = viewModelScope.launch {
            try {
                val voices = downloadTurn.withLock {
                    blockedByWifi()?.let { throw IllegalStateException(it) }
                    val started = SystemClock.elapsedRealtime()
                    downloads[i.file] = Download.Running(0, i.size, 0.0, false)
                    runInterruptible(Dispatchers.IO) {
                        store.download(i, mirror, { done, total ->
                            val s = (SystemClock.elapsedRealtime() - started) / 1000.0
                            downloads[i.file] = Download.Running(done, total, if (s > 0.5) done / s else 0.0, false)
                        }) {
                            downloads[i.file] = Download.Running(i.size, i.size, 0.0, true)
                        }
                    }
                }
                downloads.remove(i.file)
                snack("${voices.joinToString()} installed")
            } catch (e: CancellationException) {
                downloads.remove(i.file)
                throw e
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "download of ${i.file} failed", e)
                val message = e.message ?: e.javaClass.simpleName
                downloads[i.file] = Download.Failed(message)
                fail("${i.label}: $message") { download(i) }
            } finally {
                downloadJobs.remove(i.file)
                refresh()
            }
        }
    }

    fun cancel(i: Installer) {
        downloadJobs[i.file]?.cancel()
        downloads.remove(i.file)
    }

    /** Fetch the language data [v] lacks from the mirror. */
    fun downloadLanguage(v: InstalledVoice) {
        if (mirror.isEmpty()) {
            snack("No voice mirror is set: add any ${languageOf(v)} voice from the catalog to bring its language data")
            return
        }
        viewModelScope.launch {
            try {
                blockedByWifi()?.let { throw IllegalStateException(it) }
                runInterruptible(Dispatchers.IO) { store.downloadLanguage(v.name, mirror) { _, _ -> } }
                snack("${v.motherTongue} language data installed")
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "language data for ${v.name} failed", e)
                fail("${v.name}: ${e.message}") { downloadLanguage(v) }
            }
            refresh()
        }
    }

    fun import(uri: Uri) {
        if (importJob?.isActive == true) return
        val app = getApplication<Application>()
        var name = "installer.exe"
        var total = -1L
        app.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                c.getString(0)?.let { name = it }
                if (!c.isNull(1)) total = c.getLong(1)
            }
        }
        importing = Import(name, 0, total, false)
        importJob = viewModelScope.launch {
            try {
                val voices = runInterruptible(Dispatchers.IO) {
                    val input = app.contentResolver.openInputStream(uri) ?: throw IllegalStateException("cannot open $name")
                    input.use { s ->
                        store.importStream(s, total, { done, _ -> importing = Import(name, done, total, false) }) {
                            importing = Import(name, total, total, true)
                        }
                    }
                }
                snack("${voices.joinToString()} installed")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "import of $name failed", e)
                val why = e.message ?: e.javaClass.simpleName
                val notVoice = "no Loquendo voice" in why || "no voice cabinet" in why
                fail(if (notVoice) "Not a Loquendo voice: $name holds no voice configurator (.vcf)" else "$name: $why")
            } finally {
                importing = null
                refresh()
            }
        }
    }

    /** Stop the import while it is still copying (unpacking runs to its end). */
    fun cancelImport() {
        if (importing?.unpacking == false) importJob?.cancel()
    }

    fun delete(v: InstalledVoice) {
        viewModelScope.launch {
            try {
                withContext(Dispatchers.IO) {
                    synchronized(handleLock) { if (handleVoice == v.name) freeHandle() }
                    store.delete(v.name)
                }
                if (analysis?.voice == v.name) analysis = null
                snack("${v.name} deleted")
            } catch (e: Exception) {
                fail("Could not delete ${v.name}: ${e.message}")
            }
            refresh()
        }
    }

    // ---- settings ----

    fun updateMirror(url: String) {
        mirror = url.trim().trimEnd('/')
        prefs.edit().putString("mirror_url", mirror).apply()
    }

    fun updateWifiOnly(on: Boolean) {
        wifiOnly = on
        prefs.edit().putBoolean("wifi_only", on).apply()
    }

    fun updateShowCurtain(on: Boolean) {
        showCurtain = on
        prefs.edit().putBoolean("show_curtain", on).apply()
        val v = selectedVoice()
        if (on && v != null && v.ready && text.isNotBlank()) analyse(v, text)
    }

    fun refreshDefaultEngine() {
        isDefaultEngine = try {
            tts.defaultEngine == getApplication<Application>().packageName
        } catch (_: Exception) {
            false
        }
    }

    // ---- try it ----

    /** The picked voice, else the first one that is ready. */
    fun selectedVoice(): InstalledVoice? =
        installed.firstOrNull { it.name == voiceName } ?: installed.firstOrNull { it.ready } ?: installed.firstOrNull()

    fun selectVoice(v: InstalledVoice) {
        val previous = selectedVoice()
        voiceName = v.name
        prefs.edit().putString("voice", v.name).apply()
        if (text.isBlank() || text == previous?.demoSentence) updateText(v.demoSentence)
    }

    fun updateText(t: String) {
        text = t
        prefs.edit().putString("text", t).apply()
    }

    fun updateVolume(v: Float) {
        volume = v
        prefs.edit().putFloat("volume", v).apply()
    }

    /** Why [v] can't speak, as the status line puts it, or null. */
    fun cannotSpeak(v: InstalledVoice): String? = when {
        v.unsupported != null -> "${v.name} can't speak yet: ${v.unsupported}"
        v.languageMissing -> "${v.name} can't speak: needs ${v.motherTongue} language data — add any ${languageOf(v)} voice or download it from the mirror"
        else -> null
    }

    fun speak() {
        val v = selectedVoice() ?: return
        cannotSpeak(v)?.let { status = Status(Status.Kind.Warn, it); return }
        if (text.isBlank()) {
            status = Status(Status.Kind.Info, "Nothing to speak — type some text first")
            return
        }
        val locale = v.locales.firstOrNull()
        tts.voices?.firstOrNull { locale != null && it.name == VoiceStore.ttsVoiceName(v.name, locale) }
            ?.let { tts.voice = it }
            ?: locale?.let { tts.language = it }
        val id = "qq-${++utterance}"
        current = Utterance(id, SystemClock.elapsedRealtime())
        speaking = true
        status = Status(Status.Kind.Busy, "Rendering…")
        tts.speak(text, TextToSpeech.QUEUE_FLUSH, Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, volume) }, id)
        if (showCurtain) analyse(v, text)
    }

    fun stop() {
        tts.stop()
        finishSpeaking(null)
        status = Status(Status.Kind.Info, "Stopped")
    }

    /** Timing of the utterance being spoken, from the engine's callbacks. */
    private class Utterance(val id: String, val asked: Long) {
        var rendered: Long = 0
        var rate: Int = 0
        var bytes: Long = 0
        var playing: Long = 0
        val seconds: Double get() = if (rate > 0) bytes / 2.0 / rate else 0.0
    }
    @Volatile private var current: Utterance? = null

    private inner class Progress : UtteranceProgressListener() {
        override fun onBeginSynthesis(id: String, sampleRate: Int, audioFormat: Int, channelCount: Int) {
            current?.takeIf { it.id == id }?.let { it.rate = sampleRate; it.rendered = SystemClock.elapsedRealtime() }
        }

        override fun onAudioAvailable(id: String, audio: ByteArray) {
            current?.takeIf { it.id == id }?.let { it.bytes += audio.size }
        }

        override fun onStart(id: String) {
            val u = current?.takeIf { it.id == id } ?: return
            u.playing = SystemClock.elapsedRealtime()
            speakJob?.cancel()
            speakJob = viewModelScope.launch {
                while (isActive && current === u) {
                    val at = (SystemClock.elapsedRealtime() - u.playing) / 1000.0
                    playheadS = at
                    // the length is known up front only from the analysis of this same text
                    val total = analysis?.takeIf { it.voice == selectedVoice()?.name && it.text == text }?.audio?.seconds
                    status = Status(
                        Status.Kind.Busy,
                        if (total != null) "Speaking… %.1f s of %.2f s".format(java.util.Locale.ROOT, minOf(at, total), total)
                        else "Speaking… %.1f s".format(java.util.Locale.ROOT, at),
                    )
                    delay(100)
                }
            }
        }

        override fun onDone(id: String) {
            val u = current?.takeIf { it.id == id } ?: return
            val renderedIn = if (u.rendered > 0) (u.rendered - u.asked) / 1000.0 else 0.0
            finishSpeaking(u)
            status = Status(Status.Kind.Ok, "Spoke %.2f s · rendered in %.1f s".format(java.util.Locale.ROOT, u.seconds, renderedIn))
        }

        override fun onStop(id: String, interrupted: Boolean) {
            current?.takeIf { it.id == id }?.let { finishSpeaking(it) }
        }

        @Deprecated("Deprecated in Java")
        override fun onError(id: String) = onError(id, TextToSpeech.ERROR)

        override fun onError(id: String, errorCode: Int) {
            val u = current?.takeIf { it.id == id } ?: return
            finishSpeaking(u)
            status = Status(Status.Kind.Error, "Speaking failed (engine error $errorCode) — see the log for details")
        }
    }

    private fun finishSpeaking(u: Utterance?) {
        if (u == null || current === u) current = null
        speakJob?.cancel()
        speaking = false
        playheadS = null
    }

    /** The voice loaded in this process, loading it when another one is. Call holding [handleLock]. */
    private fun handleFor(v: InstalledVoice): Long {
        if (handleVoice != v.name || handle == 0L) {
            freeHandle()
            handle = Native.loadVoice(store.root.path, v.name)
            handleVoice = v.name
        }
        return handle
    }

    private fun freeHandle() {
        pieceTrack?.release()
        pieceTrack = null
        if (handle != 0L) Native.freeVoice(handle)
        handle = 0
        handleVoice = null
    }

    fun analyse(v: InstalledVoice, text: String) {
        viewModelScope.launch {
            try {
                analysis = withContext(Dispatchers.Default) {
                    val json = synchronized(handleLock) { Native.analyse(handleFor(v), text) }
                    parse(v.name, text, JSONObject(json))
                }
                playingPiece = null
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "analysis failed", e)
                status = Status(Status.Kind.Error, "Analysis failed: ${e.message}")
            }
        }
    }

    /** Play one recorded piece as it sits in the recordings, stopping the one before. */
    fun playPiece(index: Int) {
        val a = analysis ?: return
        val p = a.pieces.getOrNull(index) ?: return
        val v = installed.firstOrNull { it.name == a.voice } ?: return
        viewModelScope.launch {
            try {
                val track = withContext(Dispatchers.Default) {
                    synchronized(handleLock) {
                        val h = handleFor(v)
                        val pcm = Native.pieceSamples(h, p.unit, p.from, p.to)
                        val rate = Native.audioRate(h)
                        pieceTrack?.release()
                        pieceTrack = null
                        if (pcm.isEmpty()) return@synchronized null
                        AudioTrack.Builder()
                            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build())
                            .setAudioFormat(AudioFormat.Builder().setEncoding(AudioFormat.ENCODING_PCM_16BIT).setSampleRate(rate).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
                            .setTransferMode(AudioTrack.MODE_STATIC)
                            .setBufferSizeInBytes(pcm.size * 2)
                            .build()
                            .also { t ->
                                t.write(pcm, 0, pcm.size)
                                t.setVolume(volume)
                                t.play()
                                pieceTrack = t
                            } to (pcm.size * 1000L / rate)
                    }
                } ?: return@launch
                playingPiece = index
                delay(track.second + 100)
                if (playingPiece == index) playingPiece = null
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "piece playback failed", e)
                status = Status(Status.Kind.Error, "Could not play the piece: ${e.message}")
            }
        }
    }

    /** Render with the picked voice in this process and save it to Downloads as quoquendo.wav. */
    fun saveWav() {
        val v = selectedVoice() ?: return
        cannotSpeak(v)?.let { status = Status(Status.Kind.Warn, it); return }
        if (text.isBlank()) {
            status = Status(Status.Kind.Info, "Nothing to speak — type some text first")
            return
        }
        val words = text
        status = Status(Status.Kind.Busy, "Rendering ${v.name}…")
        viewModelScope.launch {
            try {
                val (wav, seconds) = withContext(Dispatchers.Default) {
                    val (pcm, rate) = synchronized(handleLock) {
                        val h = handleFor(v)
                        Native.synthesize(h, words) to Native.sampleRate(h)
                    }
                    wav(pcm, rate) to pcm.size.toDouble() / rate
                }
                val where = withContext(Dispatchers.IO) { writeDownload("quoquendo.wav", wav) }
                status = Status(Status.Kind.Ok, "Saved %s · %.2f s · %d KB".format(java.util.Locale.ROOT, where, seconds, wav.size / 1000))
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "save failed", e)
                status = Status(Status.Kind.Error, "Could not save: ${e.message}")
            }
        }
    }

    private fun writeDownload(name: String, bytes: ByteArray): String {
        val app = getApplication<Application>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, name)
                put(MediaStore.MediaColumns.MIME_TYPE, "audio/wav")
                put(MediaStore.MediaColumns.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
            }
            val uri = app.contentResolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
                ?: throw IllegalStateException("Downloads refused the file")
            app.contentResolver.openOutputStream(uri)!!.use { it.write(bytes) }
            return app.contentResolver.query(uri, arrayOf(MediaStore.MediaColumns.DISPLAY_NAME), null, null, null)
                ?.use { c -> if (c.moveToFirst()) c.getString(0) else null } ?: name
        }
        val dir = app.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS) ?: throw IllegalStateException("no storage")
        File(dir, name).writeBytes(bytes)
        return name
    }

    private fun wav(pcm: ShortArray, rate: Int): ByteArray {
        val data = ByteBuffer.allocate(pcm.size * 2).order(ByteOrder.LITTLE_ENDIAN)
        pcm.forEach { data.putShort(it) }
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN).apply {
            put("RIFF".toByteArray()); putInt(36 + pcm.size * 2); put("WAVE".toByteArray())
            put("fmt ".toByteArray()); putInt(16); putShort(1); putShort(1); putInt(rate); putInt(rate * 2); putShort(2); putShort(16)
            put("data".toByteArray()); putInt(pcm.size * 2)
        }
        return ByteArrayOutputStream(44 + pcm.size * 2).apply { write(header.array()); write(data.array()) }.toByteArray()
    }

    override fun onCleared() {
        tts.shutdown()
        synchronized(handleLock) { freeHandle() }
    }

    private fun parse(voice: String, text: String, o: JSONObject): Analysis {
        fun JSONArray.objs() = (0 until length()).map { getJSONObject(it) }
        fun JSONObject.str(k: String) = if (isNull(k)) null else getString(k)
        val phrases = o.getJSONArray("phrases").objs().map { p ->
            AnPhrase(
                p.getJSONArray("phones").objs().map { AnPhone(it.getString("sym"), it.getBoolean("known"), it.getBoolean("wordStart")) },
                p.getInt("pauseMs"),
                p.str("mood"),
            )
        }
        val plan = o.getJSONArray("plan").objs().map { p ->
            val m = p.getJSONArray("missing")
            AnPlanned(p.getInt("pauseMs"), p.str("mood"), (0 until m.length()).map { m.getString(it) })
        }
        val pieces = o.getJSONArray("pieces").objs().map { p ->
            val covers = p.getJSONArray("covers")
            AnPiece(
                p.getInt("phrase"),
                (0 until covers.length()).joinToString(" ") { covers.getString(it) },
                p.getInt("unit"),
                p.getString("sentence"),
                p.getInt("from"),
                p.getInt("to"),
                p.getDouble("atS"),
                p.getDouble("lenMs"),
                p.getInt("pitch"),
                p.getBoolean("hard"),
            )
        }
        val a = o.getJSONObject("audio")
        val audio = if (a.has("error")) null else {
            val env = a.getJSONArray("envelope")
            AnAudio(a.getInt("rate"), a.getLong("samples"), a.getDouble("seconds"), FloatArray(env.length()) { env.getDouble(it).toFloat() })
        }
        return Analysis(voice, text, phrases, plan, pieces, o.getInt("sentences"), audio)
    }
}
