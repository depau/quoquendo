package eu.depau.quoquendo

import android.media.AudioFormat
import android.speech.tts.SynthesisCallback
import android.speech.tts.SynthesisRequest
import android.speech.tts.TextToSpeech
import android.speech.tts.TextToSpeechService
import android.speech.tts.Voice
import android.util.Log
import java.util.Locale
import java.util.MissingResourceException

/** quoquendo as an Android text-to-speech engine. */
class QuoquendoTtsService : TextToSpeechService() {
    private lateinit var store: VoiceStore

    /** The one loaded voice: its name and engine handle. */
    private var loadedName: String? = null
    private var handle: Long = 0
    private var language: Array<String> = arrayOf("", "", "")
    private val lock = Any()
    private val listLock = Any()

    @Volatile
    private var stopped = false

    private var cached: List<InstalledVoice> = emptyList()
    private var cachedAt = 0L

    override fun onCreate() {
        store = VoiceStore(this)
        super.onCreate()
    }

    override fun onDestroy() {
        synchronized(lock) { unload() }
        super.onDestroy()
    }

    /** Installed voices that can speak, re-read at most every two seconds. */
    private fun voices(): List<InstalledVoice> = synchronized(listLock) {
        val now = System.currentTimeMillis()
        if (now - cachedAt > 2000) {
            cached = try {
                store.installed().filter { it.speakable }.also { VoiceStore.migrateChoices(VoiceStore.prefs(this), it) }
            } catch (e: Exception) {
                Log.e(TAG, "listing voices failed", e)
                emptyList()
            }
            cachedAt = now
        }
        cached
    }

    private fun Locale.iso3Language(): String = try { isO3Language } catch (_: MissingResourceException) { language }
    private fun Locale.iso3Country(): String = try { isO3Country } catch (_: MissingResourceException) { country }

    /** How well [locale] matches a request (TextToSpeech.LANG_* level). */
    private fun level(locale: Locale, lang: String?, country: String?, variant: String?): Int {
        val l = lang.orEmpty()
        if (l.isEmpty() || !(locale.iso3Language().equals(l, true) || locale.language.equals(l, true))) {
            return TextToSpeech.LANG_NOT_SUPPORTED
        }
        val c = country.orEmpty()
        if (c.isEmpty() || !(locale.iso3Country().equals(c, true) || locale.country.equals(c, true))) {
            return TextToSpeech.LANG_AVAILABLE
        }
        val v = variant.orEmpty()
        return if (v.isNotEmpty() && locale.variant.equals(v, true)) TextToSpeech.LANG_COUNTRY_VAR_AVAILABLE
        else TextToSpeech.LANG_COUNTRY_AVAILABLE
    }

    /**
     * The (voice, locale, level) a request gets, or null: the voice chosen in
     * Settings for the language, in every region, at the level its own
     * locales match; else the strongest match, preferring a voice with its
     * language data.
     */
    private fun best(lang: String?, country: String?, variant: String?): Triple<InstalledVoice, Locale, Int>? {
        val voices = voices()
        val matches = voices.flatMap { v -> v.locales.map { Triple(v, it, level(it, lang, country, variant)) } }
            .filter { it.third >= TextToSpeech.LANG_AVAILABLE }
        val first = matches.firstOrNull() ?: return null
        val chosen = VoiceStore.chosenFor(voices, first.second.language, VoiceStore.prefs(this))
        return matches.filter { chosen == null || it.first == chosen }
            .maxWithOrNull(compareBy({ it.third }, { !it.first.languageMissing }))
    }

    override fun onIsLanguageAvailable(lang: String?, country: String?, variant: String?): Int =
        best(lang, country, variant)?.third ?: TextToSpeech.LANG_NOT_SUPPORTED

    override fun onGetLanguage(): Array<String> = synchronized(lock) { language }

    override fun onLoadLanguage(lang: String?, country: String?, variant: String?): Int {
        val (voice, locale, level) = best(lang, country, variant) ?: return TextToSpeech.LANG_NOT_SUPPORTED
        synchronized(lock) {
            if (!load(voice.name)) return TextToSpeech.LANG_NOT_SUPPORTED
            language = arrayOf(locale.iso3Language(), locale.iso3Country(), locale.variant)
        }
        return level
    }

    override fun onGetVoices(): List<Voice> = voices().flatMap { v ->
        v.locales.map { locale ->
            Voice(
                VoiceStore.ttsVoiceName(v.name, locale),
                locale,
                Voice.QUALITY_HIGH,
                Voice.LATENCY_NORMAL,
                false,
                emptySet(),
            )
        }
    }

    override fun onGetDefaultVoiceNameFor(lang: String?, country: String?, variant: String?): String? =
        best(lang, country, variant)?.let { (v, locale, _) -> VoiceStore.ttsVoiceName(v.name, locale) }

    /** The installed voice and locale behind a TTS voice name. */
    private fun byVoiceName(voiceName: String?): Pair<InstalledVoice, Locale>? {
        if (voiceName == null) return null
        for (v in voices()) for (l in v.locales) {
            if (VoiceStore.ttsVoiceName(v.name, l) == voiceName) return v to l
        }
        return null
    }

    override fun onIsValidVoiceName(voiceName: String?): Int =
        if (byVoiceName(voiceName) != null) TextToSpeech.SUCCESS else TextToSpeech.ERROR

    override fun onLoadVoice(voiceName: String?): Int {
        val (voice, locale) = byVoiceName(voiceName) ?: return TextToSpeech.ERROR
        synchronized(lock) {
            if (!load(voice.name)) return TextToSpeech.ERROR
            language = arrayOf(locale.iso3Language(), locale.iso3Country(), locale.variant)
        }
        return TextToSpeech.SUCCESS
    }

    /** Make [name] the one loaded voice. Call holding [lock]. */
    private fun load(name: String): Boolean {
        if (loadedName == name && handle != 0L) return true
        unload()
        return try {
            handle = Native.loadVoice(store.root.path, name)
            loadedName = name
            true
        } catch (e: Exception) {
            Log.e(TAG, "loading $name failed", e)
            false
        }
    }

    private fun unload() {
        if (handle != 0L) Native.freeVoice(handle)
        handle = 0
        loadedName = null
    }

    override fun onStop() {
        stopped = true
    }

    /** Into the speech request log, when Save speech requests is on. */
    private fun save(request: SynthesisRequest, text: String) {
        val prefs = VoiceStore.prefs(this)
        if (!SpeechLog.enabled(prefs) || text.isBlank()) return
        val pm = packageManager
        // apps sharing a uid can't be told apart: Settings and the other system apps are "Android System"
        val packages = pm.getPackagesForUid(request.callerUid).orEmpty()
        val app = packages.singleOrNull() ?: "android".takeIf { it in packages } ?: pm.getNameForUid(request.callerUid) ?: "uid ${request.callerUid}"
        val label = runCatching { pm.getApplicationLabel(pm.getApplicationInfo(app, 0)).toString() }.getOrDefault(app)
        // a float from Bundle params, a string from the old HashMap ones
        @Suppress("DEPRECATION")
        val volume = when (val v = request.params?.get(TextToSpeech.Engine.KEY_PARAM_VOLUME)) {
            is Number -> v.toFloat()
            is String -> v.toFloatOrNull()
            else -> null
        }?.takeIf { it < 1f }
        val entry = SpeechRequest(System.currentTimeMillis(), app, label, text, request.language.orEmpty(), request.pitch.takeIf { it != 100 }, volume)
        runCatching { SpeechLog.add(this, prefs, entry) }.onFailure { Log.w(TAG, "saving the speech request failed", it) }
    }

    override fun onSynthesizeText(request: SynthesisRequest, callback: SynthesisCallback) {
        stopped = false
        val text = request.charSequenceText?.toString().orEmpty()
        Log.i(TAG, "onSynthesizeText voice=${request.voiceName} lang=${request.language}-${request.country} chars=${text.length}")
        save(request, text)
        val speech = try {
            synchronized(lock) {
                val wanted = byVoiceName(request.voiceName)?.first
                    ?: best(request.language, request.country, request.variant)?.first
                    ?: throw IllegalStateException("no installed voice speaks ${request.language}-${request.country}")
                if (!load(wanted.name)) throw IllegalStateException("could not load ${wanted.name}")
                // rate and pitch from the request are ignored: the engine has no speed or pitch control yet
                val prefs = VoiceStore.prefs(this)
                Native.synthesize(handle, text, ProfileStore.smooth(prefs), null, ProfileStore.activeJson(prefs))
            }
        } catch (e: Exception) {
            Log.e(TAG, "synthesis failed", e)
            callback.error(TextToSpeech.ERROR_SYNTHESIS)
            return
        }
        val samples = speech.pcm
        Log.i(TAG, "synthesized ${samples.size} samples at ${speech.rate} Hz")
        if (callback.start(speech.rate, AudioFormat.ENCODING_PCM_16BIT, 1) != TextToSpeech.SUCCESS) return
        // each word's start, reported to the app as playback reaches it (onRangeStart)
        val m = speech.marks
        for (i in 0 until m.size / 3) callback.rangeStart(m[3 * i], m[3 * i + 1], m[3 * i + 2])
        val bytes = ByteArray(samples.size * 2)
        for (i in samples.indices) {
            val s = samples[i].toInt()
            bytes[2 * i] = s.toByte()
            bytes[2 * i + 1] = (s shr 8).toByte()
        }
        val chunk = callback.maxBufferSize.coerceAtLeast(2)
        var at = 0
        while (at < bytes.size) {
            if (stopped) return
            val n = minOf(chunk, bytes.size - at)
            if (callback.audioAvailable(bytes, at, n) != TextToSpeech.SUCCESS) return
            at += n
        }
        callback.done()
    }

    companion object {
        const val TAG = "quoquendo"
    }
}
