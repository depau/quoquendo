package eu.depau.quoquendo

import android.content.Context
import android.content.SharedPreferences
import org.json.JSONObject
import java.io.File

/** A request an app sent the engine, with its text as the app wrote it (before any profile). */
data class SpeechRequest(
    val time: Long,
    /** the app's package, and its name when the request came */
    val app: String,
    val label: String,
    val text: String,
    /** the request's language (ISO 639: `ita`) */
    val language: String,
    /** pitch in percent, only when the app asked for other than 100 */
    val pitch: Int?,
    /** volume from 0 to 1, only when the app asked for less than full */
    val volume: Float?,
)

/**
 * "Save speech requests": what apps ask the engine to say, so a profile can be
 * written for them. One JSON object per line in the app's own files; off by
 * default, and entries older than the retention are dropped.
 */
object SpeechLog {
    const val ENABLED = "save_speech_requests"
    const val DAYS = "speech_requests_days"
    /** How long entries are kept, in days; 0 keeps them. */
    val RETENTION = listOf(1, 7, 30, 0)
    private const val DAY_MS = 86_400_000L
    private val lock = Any()

    fun enabled(prefs: SharedPreferences) = prefs.getBoolean(ENABLED, false)
    fun days(prefs: SharedPreferences) = prefs.getInt(DAYS, 7)

    private fun file(context: Context) = File(context.filesDir, "speech-requests.jsonl")

    private fun cutoff(prefs: SharedPreferences): Long =
        days(prefs).let { if (it == 0) Long.MIN_VALUE else System.currentTimeMillis() - it * DAY_MS }

    fun add(context: Context, prefs: SharedPreferences, r: SpeechRequest) = synchronized(lock) {
        val f = file(context)
        val oldest = if (f.exists()) f.bufferedReader().use { it.readLine() }?.let(::parse)?.time else null
        if (oldest != null && oldest < cutoff(prefs)) prune(f, prefs)
        f.appendText(json(r) + "\n")
    }

    /** The entries still kept, oldest first. */
    fun all(context: Context, prefs: SharedPreferences): List<SpeechRequest> = synchronized(lock) {
        val f = file(context)
        if (f.exists()) prune(f, prefs) else emptyList()
    }

    fun clear(context: Context) = synchronized(lock) { file(context).delete() }

    /** Drop entries past the retention (and unreadable lines); the rest, oldest first. */
    private fun prune(f: File, prefs: SharedPreferences): List<SpeechRequest> {
        val lines = f.readLines().filter { it.isNotBlank() }
        val since = cutoff(prefs)
        val kept = lines.mapNotNull(::parse).filter { it.time >= since }
        if (kept.size != lines.size) f.writeText(kept.joinToString("") { json(it) + "\n" })
        return kept
    }

    private fun json(r: SpeechRequest): String = JSONObject()
        .put("time", r.time)
        .put("app", r.app)
        .put("label", r.label)
        .put("text", r.text)
        .put("language", r.language)
        .apply { r.pitch?.let { put("pitch", it) } }
        .apply { r.volume?.let { put("volume", it.toDouble()) } }
        .toString()

    private fun parse(line: String): SpeechRequest? = runCatching {
        val o = JSONObject(line)
        SpeechRequest(
            o.getLong("time"),
            o.getString("app"),
            o.optString("label", o.getString("app")),
            o.getString("text"),
            o.optString("language"),
            if (o.has("pitch")) o.getInt("pitch") else null,
            if (o.has("volume")) o.getDouble("volume").toFloat() else null,
        )
    }.getOrNull()
}
