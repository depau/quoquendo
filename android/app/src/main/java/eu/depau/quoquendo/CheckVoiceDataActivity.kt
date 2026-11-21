package eu.depau.quoquendo

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.util.Log
import java.util.Locale
import java.util.MissingResourceException

/**
 * Answers `CHECK_TTS_DATA`: the locales the installed, speakable voices cover,
 * as `eng-USA` (ISO 639-2 language, ISO 3166 alpha-3 country), the form the
 * system TTS settings compare its default language against before enabling
 * "Listen to an example".
 */
class CheckVoiceDataActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val available = try {
            VoiceStore(this).installed().filter { it.speakable }
                .flatMap { v -> v.locales.map(::iso3) }
                .distinct()
        } catch (e: Exception) {
            Log.e(VoiceStore.TAG, "listing voices failed", e)
            emptyList()
        }
        val data = Intent()
            .putStringArrayListExtra(TextToSpeech.Engine.EXTRA_AVAILABLE_VOICES, ArrayList(available))
            .putStringArrayListExtra(TextToSpeech.Engine.EXTRA_UNAVAILABLE_VOICES, ArrayList())
        setResult(TextToSpeech.Engine.CHECK_VOICE_DATA_PASS, data)
        finish()
    }

    private fun iso3(locale: Locale): String {
        val lang = try { locale.isO3Language } catch (_: MissingResourceException) { locale.language }
        val country = try { locale.isO3Country } catch (_: MissingResourceException) { locale.country }
        return listOf(lang, country, locale.variant).filter { it.isNotEmpty() }.joinToString("-")
    }
}

/**
 * Answers `GET_SAMPLE_TEXT` (what the system TTS settings speak for "Listen to
 * an example"): the demo sentence of the voice apps get for the requested
 * language.
 */
class SampleTextActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val lang = intent.getStringExtra("language").orEmpty()
        val country = intent.getStringExtra("country").orEmpty()
        val sample = try {
            val voices = VoiceStore(this).installed().filter { it.speakable }
            val locales = voices.flatMap { it.locales }.filter { it.language.equals(lang, true) || it.isO3LanguageOrNull().equals(lang, true) }
            val locale = locales.firstOrNull { it.country.equals(country, true) } ?: locales.firstOrNull()
            locale?.let { VoiceStore.appVoiceFor(voices, it, VoiceStore.prefs(this))?.demoSentence }
        } catch (e: Exception) {
            Log.e(VoiceStore.TAG, "listing voices failed", e)
            null
        }
        if (sample.isNullOrBlank()) {
            setResult(TextToSpeech.LANG_NOT_SUPPORTED)
        } else {
            setResult(TextToSpeech.LANG_AVAILABLE, Intent().putExtra("sampleText", sample))
        }
        finish()
    }

    private fun Locale.isO3LanguageOrNull(): String? = try { isO3Language } catch (_: MissingResourceException) { null }
}
