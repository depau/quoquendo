package eu.depau.quoquendo

import android.app.Application
import android.content.ContentValues
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.net.ConnectivityManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.SystemClock
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.provider.Settings
import android.util.Log
import android.webkit.MimeTypeMap
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
import java.io.File
import java.util.Locale

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

/** The line under Speak, when something stands in the way. */
data class Status(val kind: Kind, val text: String) {
    enum class Kind { Warn, Error }
}

/** A format Save writes (`Native.formats`). */
data class SaveFormat(val ext: String, val label: String, val hint: String, val mime: String)

/** What an analysis was made of: its voice, text, joining and reading language (null: the voice's own). */
data class AnalysisKey(val voice: String, val text: String, val smooth: Boolean, val language: String?, val profile: String)

/** Try it's player. */
enum class Playback { Stopped, Playing, Paused }

/** What the analysis shows: sounds, recorded pieces, words and the waveform outline, all timed. */
class Analysis(
    val key: AnalysisKey,
    val sounds: List<AnSound>,
    val plan: List<AnPlanned>,
    val pieces: List<AnPiece>,
    val words: List<AnWord>,
    val sentences: Int,
    val audio: AnAudio?,
    /** the audio itself, which Speak plays */
    val rendered: Rendered?,
) {
    val voice: String get() = key.voice
    val text: String get() = key.text

    /** The sound playing at [s] seconds (an index into [sounds]). */
    fun soundAt(s: Double): Int? = sounds.indexOfLast { it.atS <= s }.takeIf { it >= 0 && s < sounds[it].endS }

    /** The piece playing at [s] seconds (an index into [pieces]). */
    fun pieceAt(s: Double): Int? = pieces.indexOfFirst { s >= it.atS && s < it.endS }.takeIf { it >= 0 }
}
data class AnSound(
    val sym: String, val known: Boolean, val atS: Double, val endS: Double,
    val word: Int?, val phrase: Int, val wordStart: Boolean, val pause: Boolean,
)
data class AnPlanned(val pauseMs: Int, val mood: String?, val missing: List<String>)
data class AnPiece(
    val phrase: Int, val covers: String, val unit: Int, val sentence: String,
    val from: Int, val to: Int, val recS: Double, val lenMs: Double, val pitch: Int, val hard: Boolean,
    val atS: Double, val endS: Double,
)
/** A word: its UTF-16 range in the text and, when spoken, its time. */
data class AnWord(val start: Int, val end: Int, val atS: Double?, val endS: Double?)
data class AnAudio(val rate: Int, val samples: Long, val seconds: Double, val step: Int, val envelope: FloatArray)

/** The Voice details rows, and the languages the voice can read with (its own first). */
data class VoiceDetails(val voice: String, val language: String, val languages: List<String>, val rows: List<Pair<String, String>>)

/** The app's state; its work lives as long as the app is open. */
class MainModel(app: Application) : AndroidViewModel(app) {
    private val prefs = VoiceStore.prefs(app)
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
    var advanced by mutableStateOf(prefs.getBoolean(VoiceStore.ADVANCED, false))
        private set
    /** The speech profiles, and the active one's JSON (what the engine reads). */
    var profiles by mutableStateOf(ProfileStore.all(prefs))
        private set
    var activeProfile by mutableStateOf(ProfileStore.activeName(prefs))
        private set
    var activeProfileJson by mutableStateOf(ProfileStore.activeJson(prefs))
        private set
    /** Apps with their own profile: package to profile name. */
    var appProfiles by mutableStateOf(ProfileStore.appProfiles(prefs))
        private set
    /** The voice chosen for each language (`en`) in Settings. */
    var voiceChoices by mutableStateOf(readChoices())
        private set
    /** Save speech requests: on, how many days they're kept (0: kept), and how many there are. */
    var saveRequests by mutableStateOf(SpeechLog.enabled(prefs))
        private set
    var requestDays by mutableStateOf(SpeechLog.days(prefs))
        private set
    var requests by mutableStateOf<List<SpeechRequest>>(emptyList())
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
    /** The reading language picked in Advanced (null: the voice's own). */
    var language by mutableStateOf<String?>(null)
        private set
    val formats: List<SaveFormat> = JSONArray(Native.formats()).let { a ->
        (0 until a.length()).map { a.getJSONObject(it) }.map { SaveFormat(it.getString("ext"), it.getString("label"), it.getString("hint"), it.getString("mime")) }
    }
    var saveFormat by mutableStateOf(formats.firstOrNull { it.ext == prefs.getString("save_format", "flac") } ?: formats.first())
        private set
    var saving by mutableStateOf(false)
        private set
    var status by mutableStateOf<Status?>(null)
        private set
    /** Try it's player: stopped, playing [spoken], or paused in it. */
    var playback by mutableStateOf(Playback.Stopped)
        private set
    /** The analysis being played, and how far into it. */
    var spoken by mutableStateOf<Analysis?>(null)
        private set
    var playheadS by mutableStateOf<Double?>(null)
        private set
    var analysis by mutableStateOf<Analysis?>(null)
        private set
    var details by mutableStateOf<VoiceDetails?>(null)
        private set
    var playingPiece by mutableStateOf<Int?>(null)
        private set
    var isDefaultEngine by mutableStateOf(true)
        private set

    private var analysisJob: Job? = null
    private var pendingKey: AnalysisKey? = null

    // Try it's player
    private val audioManager = app.getSystemService(AudioManager::class.java)
    private val speechAttributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build()
    private val focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT)
        .setAudioAttributes(speechAttributes)
        .setOnAudioFocusChangeListener { change -> if (change < 0) pause() }
        .build()
    private var track: AudioTrack? = null
    private var startJob: Job? = null
    private var writer: Job? = null
    private var ticker: Job? = null

    /** The voice loaded in this process for analysis, saving and piece playback. */
    private var handle = 0L
    private var handleVoice: String? = null
    private val handleLock = Any()
    private var pieceTrack: AudioTrack? = null

    init {
        refresh()
        refreshDefaultEngine()
    }

    // ---- library ----

    fun refresh() {
        viewModelScope.launch {
            try {
                val (list, used) = withContext(Dispatchers.IO) { store.installed() to store.bytesUsed() }
                installed = list
                bytesUsed = used
                VoiceStore.migrateChoices(prefs, list)
                voiceChoices = readChoices()
            } catch (e: Exception) {
                fail("Could not list voices: ${e.message}")
            }
        }
        refreshRequests()
    }

    fun refreshRequests() {
        viewModelScope.launch {
            requests = withContext(Dispatchers.IO) { runCatching { SpeechLog.all(getApplication(), prefs) }.getOrDefault(emptyList()) }
        }
    }

    fun installerOf(v: InstalledVoice): Installer? = catalog.firstOrNull { it.voice.equals(v.name, ignoreCase = true) }

    /** `US English` for Allison: the catalog's language name, else the MotherTongue. */
    fun languageOf(v: InstalledVoice): String = installerOf(v)?.language ?: v.motherTongue

    fun installedFor(i: Installer): InstalledVoice? = installed.firstOrNull { it.name.equals(i.voice, ignoreCase = true) }

    fun message(message: String) = snack(message)

    private fun snack(message: String) {
        snacks.trySend(Snack(message))
    }

    private fun fail(message: String, retry: (() -> Unit)? = null) {
        snacks.trySend(Snack(message, error = true, retry = retry))
    }

    private fun blockedByWifi(): String? {
        if (!wifiOnly) return null
        val cm = getApplication<Application>().getSystemService(ConnectivityManager::class.java)
        return if (cm.isActiveNetworkMetered) "metered network, Wi-Fi only is on" else null
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
            snack("No voice mirror: add any ${languageOf(v)} voice from the catalog")
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
                fail(if (notVoice) "Not a Loquendo voice: $name" else "$name: $why")
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
                if (details?.voice == v.name) details = null
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

    fun updateAdvanced(on: Boolean) {
        advanced = on
        prefs.edit().putBoolean(VoiceStore.ADVANCED, on).apply()
        scheduleAnalysis()
    }

    fun updateSaveRequests(on: Boolean) {
        saveRequests = on
        prefs.edit().putBoolean(SpeechLog.ENABLED, on).apply()
    }

    fun updateRequestDays(days: Int) {
        requestDays = days
        prefs.edit().putInt(SpeechLog.DAYS, days).apply()
        refreshRequests()
    }

    fun clearRequests() {
        SpeechLog.clear(getApplication())
        requests = emptyList()
    }

    private fun refreshProfiles() {
        profiles = ProfileStore.all(prefs)
        activeProfile = ProfileStore.activeName(prefs)
        activeProfileJson = ProfileStore.activeJson(prefs)
        appProfiles = ProfileStore.appProfiles(prefs)
        scheduleAnalysis()
    }

    /** Give [app] its own profile, or (null) the active one. */
    fun setAppProfile(app: String, name: String?) {
        ProfileStore.setAppProfile(prefs, app, name)
        refreshProfiles()
    }

    fun useProfile(name: String) {
        ProfileStore.setActive(prefs, name)
        refreshProfiles()
    }

    fun saveProfile(oldName: String, json: String): String? {
        val error = ProfileStore.update(prefs, oldName, json)
        if (error == null) refreshProfiles()
        return error
    }

    fun duplicateProfile(name: String): String? {
        val created = ProfileStore.duplicate(prefs, name)
        if (created != null) refreshProfiles()
        return created
    }

    fun deleteProfile(name: String): String? {
        val error = ProfileStore.delete(prefs, name)
        if (error == null) refreshProfiles()
        return error
    }

    fun importProfile(json: String): String? {
        val name = ProfileStore.import(prefs, json) ?: return "invalid profile"
        refreshProfiles()
        return name
    }

    fun exportProfile(name: String): String? = ProfileStore.export(prefs, name)

    fun backupSettings(): String {
        val o = org.json.JSONObject(ProfileStore.backup(prefs))
        o.put("advanced", advanced)
        o.put("mirror_url", mirror)
        o.put("wifi_only", wifiOnly)
        o.put("volume", volume.toDouble())
        o.put("save_format", saveFormat.ext)
        o.put(SpeechLog.ENABLED, saveRequests)
        o.put(SpeechLog.DAYS, requestDays)
        return o.toString()
    }

    fun restoreSettings(json: String): String? {
        val error = ProfileStore.restore(prefs, json)
        if (error != null) return error
        val o = runCatching { org.json.JSONObject(json) }.getOrNull() ?: return "not a settings backup"
        updateAdvanced(o.optBoolean("advanced", advanced))
        o.optString("mirror_url").takeIf { it.isNotEmpty() }?.let { updateMirror(it) }
        updateWifiOnly(o.optBoolean("wifi_only", wifiOnly))
        if (o.has("volume")) updateVolume(o.optDouble("volume", volume.toDouble()).toFloat())
        formats.firstOrNull { it.ext == o.optString("save_format") }?.let { saveFormat = it }
        updateSaveRequests(o.optBoolean(SpeechLog.ENABLED, saveRequests))
        o.optInt(SpeechLog.DAYS, -1).takeIf { it in SpeechLog.RETENTION }?.let { updateRequestDays(it) }
        refreshProfiles()
        return null
    }

    private fun readChoices(): Map<String, String> = VoiceStore.choices(prefs)

    /** The languages (`en`) installed voices speak, by name. */
    fun appLanguages(): List<String> =
        installed.filter { it.speakable }.flatMap { v -> v.locales.map { it.language } }.distinct().sortedBy { languageName(it) }

    /** How many regions the installed voices speak [language] in. */
    fun regions(language: String): Int = VoiceStore.regions(installed, language)

    fun speakersOf(language: String): List<InstalledVoice> = VoiceStore.speakers(installed, language)

    /**
     * The voice chosen for [language] (`en`); else, when its voices all come
     * from one region, the one apps get there; else null (Match the region).
     */
    fun appVoice(language: String): InstalledVoice? {
        val speakers = speakersOf(language)
        voiceChoices[language]?.let { name -> speakers.firstOrNull { it.name == name } }?.let { return it }
        if (regions(language) > 1) return null
        val locale = speakers.flatMap { it.locales }.firstOrNull { it.language == language } ?: return null
        return VoiceStore.regionalFor(installed, locale)
    }

    /** Give apps [name] for every region of [language], or (null) each region's own voice. */
    fun chooseAppVoice(language: String, name: String?) {
        voiceChoices = if (name == null) voiceChoices - language else voiceChoices + (language to name)
        prefs.edit().apply { if (name == null) remove(VoiceStore.voiceForKey(language)) else putString(VoiceStore.voiceForKey(language), name) }.apply()
    }

    fun refreshDefaultEngine() {
        val app = getApplication<Application>()
        isDefaultEngine = Settings.Secure.getString(app.contentResolver, Settings.Secure.TTS_DEFAULT_SYNTH) == app.packageName
    }

    // ---- try it ----

    /** The picked voice, else the first one that is ready. */
    fun selectedVoice(): InstalledVoice? =
        installed.firstOrNull { it.name == voiceName } ?: installed.firstOrNull { it.ready } ?: installed.firstOrNull()

    fun selectVoice(v: InstalledVoice) {
        val previous = selectedVoice()
        voiceName = v.name
        language = null
        prefs.edit().putString("voice", v.name).apply()
        if (text.isBlank() || text == previous?.demoSentence) updateText(v.demoSentence) else scheduleAnalysis()
    }

    fun updateText(t: String) {
        text = t
        prefs.edit().putString("text", t).apply()
        scheduleAnalysis()
    }

    /** Back to the voice's own sentence. */
    fun sampleText() {
        selectedVoice()?.let { updateText(it.demoSentence) }
    }

    fun updateVolume(v: Float) {
        volume = v
        track?.setVolume(v)
        prefs.edit().putFloat("volume", v).apply()
    }

    fun updateLanguage(l: String) {
        language = l.takeIf { it != details?.language }
        scheduleAnalysis()
    }

    fun updateSaveFormat(f: SaveFormat) {
        saveFormat = f
        prefs.edit().putString("save_format", f.ext).apply()
    }

    /** Why [v] can't speak, as the status line puts it, or null. */
    fun cannotSpeak(v: InstalledVoice): String? = when {
        v.unsupported != null -> "${v.name} can't speak yet: ${v.unsupported}"
        v.languageMissing -> "${v.name} needs ${v.motherTongue} language data"
        else -> null
    }

    /** What the Try it screen shows now, or null when there is nothing to analyse. */
    private fun currentKey(v: InstalledVoice? = selectedVoice()): AnalysisKey? {
        if (v == null || !v.ready || text.isBlank()) return null
        return AnalysisKey(v.name, text, ProfileStore.smooth(prefs), language.takeIf { advanced }, activeProfileJson)
    }

    /** Analyse and render what the screen shows after [delayMs], replacing a pending analysis. */
    fun scheduleAnalysis(delayMs: Long = ANALYSIS_DELAY_MS) {
        val v = selectedVoice()
        val key = currentKey(v)
        if (key == null || v == null) {
            analysisJob?.cancel()
            pendingKey = null
            analysis = null
            return
        }
        if (analysis?.key == key || (pendingKey == key && analysisJob?.isActive == true)) return
        analysisJob?.cancel()
        pendingKey = key
        analysisJob = viewModelScope.launch {
            delay(delayMs)
            analyse(v, key)
        }
    }

    private suspend fun analyse(v: InstalledVoice, key: AnalysisKey): Analysis? = try {
        val (a, d) = withContext(Dispatchers.Default) {
            synchronized(handleLock) {
                val h = handleFor(v)
                val d = if (details?.voice != v.name) parseDetails(v.name, JSONObject(Native.details(h))) else null
                parse(key, Native.analyse(h, key.text, key.smooth, key.language, key.profile)) to d
            }
        }
        d?.let { details = it }
        analysis = a
        // what was playing or paused no longer matches the preview
        if (spoken != null && spoken !== a) stop()
        playingPiece = null
        if (status?.kind == Status.Kind.Error) status = null
        a
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Log.e(VoiceStore.TAG, "analysis failed", e)
        status = Status(Status.Kind.Error, "Analysis failed: ${e.message}")
        null
    }

    /**
     * Speak: play the rendered preview from the start, rendering it first if
     * the text just changed; when paused, go on from where it stopped.
     */
    fun speak() {
        if (playback == Playback.Paused && track != null && spoken === analysis) {
            play()
            return
        }
        val v = selectedVoice() ?: return
        val key = currentKey(v) ?: return
        stop()
        startJob = viewModelScope.launch {
            val a = analysis?.takeIf { it.key == key } ?: run {
                analysisJob?.cancel()
                pendingKey = null
                analyse(v, key)
            } ?: return@launch
            val r = a.rendered?.takeIf { it.pcm.isNotEmpty() } ?: return@launch
            val t = AudioTrack.Builder()
                .setAudioAttributes(speechAttributes)
                .setAudioFormat(AudioFormat.Builder().setEncoding(AudioFormat.ENCODING_PCM_16BIT).setSampleRate(r.rate).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setBufferSizeInBytes(maxOf(AudioTrack.getMinBufferSize(r.rate, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_16BIT), r.rate / 2))
                .build()
            t.setVolume(volume)
            track = t
            spoken = a
            playheadS = 0.0
            // the samples, then a little silence so the last of them play out;
            // written without blocking, on this thread, so stop() can release the track at any time
            writer = launch {
                suspend fun feed(data: ShortArray) {
                    var at = 0
                    while (at < data.size) {
                        val n = t.write(data, at, minOf(WRITE_CHUNK, data.size - at), AudioTrack.WRITE_NON_BLOCKING)
                        if (n < 0) return
                        at += n
                        if (n == 0) delay(20)
                    }
                }
                feed(r.pcm)
                feed(ShortArray(r.rate / 5))
            }
            play()
        }
    }

    /** Play [track] from its playback head, which drives the playhead, until its samples have all played. */
    private fun play() {
        val t = track ?: return
        val r = spoken?.rendered ?: return
        if (audioManager.requestAudioFocus(focus) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED) return
        t.play()
        playback = Playback.Playing
        ticker?.cancel()
        ticker = viewModelScope.launch {
            while (isActive) {
                val at = t.playbackHeadPosition
                playheadS = minOf(at, r.pcm.size).toDouble() / r.rate
                if (at >= r.pcm.size) {
                    stop()
                    break
                }
                delay(16)
            }
        }
    }

    /** Hold the playhead where it is; Speak goes on from there. */
    fun pause() {
        if (playback != Playback.Playing) return
        track?.pause()
        ticker?.cancel()
        audioManager.abandonAudioFocusRequest(focus)
        playback = Playback.Paused
    }

    /** Stop, back to the start. */
    fun stop() {
        startJob?.cancel()
        ticker?.cancel()
        writer?.cancel()
        track?.let {
            it.pause()
            it.flush()
            it.release()
        }
        track = null
        audioManager.abandonAudioFocusRequest(focus)
        playback = Playback.Stopped
        spoken = null
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

    /** Render what the screen shows in the chosen format and save it to Downloads. */
    fun save() {
        val v = selectedVoice() ?: return
        val key = currentKey(v) ?: return
        val f = saveFormat
        saving = true
        viewModelScope.launch {
            try {
                val bytes = withContext(Dispatchers.Default) {
                    synchronized(handleLock) { Native.export(handleFor(v), key.text, f.ext, key.smooth, key.language, key.profile) }
                }
                // MediaStore names the file by its own MIME map (audio/x-wav, not audio/wav)
                val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(f.ext) ?: f.mime
                val where = withContext(Dispatchers.IO) { writeDownload("quoquendo.${f.ext}", bytes, mime) }
                snack("Saved $where to Downloads")
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "save failed", e)
                fail("Could not save: ${e.message}")
            } finally {
                saving = false
            }
        }
    }

    private fun writeDownload(name: String, bytes: ByteArray, mime: String): String {
        val app = getApplication<Application>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, name)
                put(MediaStore.MediaColumns.MIME_TYPE, mime)
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

    override fun onCleared() {
        stop()
        synchronized(handleLock) { freeHandle() }
    }

    private fun parse(key: AnalysisKey, analysed: Analysed): Analysis {
        val o = JSONObject(analysed.json)
        fun JSONArray.objs() = (0 until length()).map { getJSONObject(it) }
        fun JSONObject.str(k: String) = if (isNull(k)) null else getString(k)
        fun JSONObject.num(k: String) = if (isNull(k)) null else getDouble(k)
        val sounds = o.getJSONArray("sounds").objs().map { s ->
            AnSound(
                s.getString("sym"), s.getBoolean("known"), s.getDouble("atS"), s.getDouble("endS"),
                if (s.isNull("word")) null else s.getInt("word"), s.getInt("phrase"), s.getBoolean("wordStart"), s.getBoolean("pause"),
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
                p.getDouble("recS"),
                p.getDouble("lenMs"),
                p.getInt("pitch"),
                p.getBoolean("hard"),
                p.getDouble("atS"),
                p.getDouble("endS"),
            )
        }
        val words = o.getJSONArray("words").objs().map { w -> AnWord(w.getInt("start"), w.getInt("end"), w.num("atS"), w.num("endS")) }
        val a = o.getJSONObject("audio")
        val audio = if (a.has("error")) null else {
            val env = a.getJSONArray("envelope")
            AnAudio(a.getInt("rate"), a.getLong("samples"), a.getDouble("seconds"), a.getInt("step"), FloatArray(env.length()) { env.getDouble(it).toFloat() })
        }
        return Analysis(key, sounds, plan, pieces, words, o.getInt("sentences"), audio, analysed.audio)
    }

    private fun parseDetails(voice: String, o: JSONObject): VoiceDetails {
        val langs = o.getJSONArray("languages")
        val rows = o.getJSONArray("rows")
        return VoiceDetails(
            voice,
            o.getString("language"),
            (0 until langs.length()).map { langs.getString(it) },
            (0 until rows.length()).map { rows.getJSONArray(it).let { r -> r.getString(0) to r.getString(1) } },
        )
    }

    companion object {
        const val ANALYSIS_DELAY_MS = 400L

        /** Samples handed to the player per write. */
        private const val WRITE_CHUNK = 4096
    }
}

/** `English` for `en`, in the device's language. */
fun languageName(language: String): String =
    Locale.forLanguageTag(language).displayLanguage.ifEmpty { language }.replaceFirstChar { it.titlecase(Locale.getDefault()) }
