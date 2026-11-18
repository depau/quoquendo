package eu.depau.quoquendo

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.util.Log

/** Answers `CHECK_TTS_DATA`: the locales the installed, speakable voices cover. */
class CheckVoiceDataActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val available = try {
            VoiceStore(this).installed().filter { it.speakable }
                .flatMap { v -> v.locales.map { it.toLanguageTag() } }
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
}
