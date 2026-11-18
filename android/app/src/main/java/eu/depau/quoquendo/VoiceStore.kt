package eu.depau.quoquendo

import android.content.Context
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.Locale

/** A voice installed in the store (one entry of `Native.listVoices`). */
data class InstalledVoice(
    val name: String,
    val description: String,
    val motherTongue: String,
    val locales: List<Locale>,
    val gender: String,
    val rate: Int,
    val languageMissing: Boolean,
    val unsupported: String?,
) {
    /** Why it can't speak, or null when it can. */
    val problem: String?
        get() = unsupported?.let { "Unsupported yet: $it" }

    /** Shown under the name when its language data comes from another installer. */
    val warning: String?
        get() = if (languageMissing && unsupported == null)
            "Needs $motherTongue language data: add any $motherTongue voice from the pack, or download it from the mirror"
        else null

    val speakable: Boolean get() = unsupported == null
}

/** One installer of the pack (`Native.catalog`). */
data class Installer(
    val file: String,
    val label: String,
    val language: String,
    val voice: String,
    val gender: String,
    val size: Long,
    val archiveUrl: String,
)

/** Progress callback: bytes done, total (or -1 when unknown). */
typealias Progress = (done: Long, total: Long) -> Unit

/** The LTTS7 voice store in the app's files dir, and the ways to fill it. */
class VoiceStore(context: Context) {
    val root: File = File(context.filesDir, "LTTS7").apply { mkdirs() }
    private val cache: File = context.cacheDir

    fun installed(): List<InstalledVoice> {
        val arr = JSONArray(Native.listVoices(root.path))
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            val locales = o.getJSONArray("locales").let { l -> (0 until l.length()).map { l.getString(it) } }
                .map { Locale.forLanguageTag(it) }
                .filter { it.language.isNotEmpty() }
            InstalledVoice(
                name = o.getString("name"),
                description = o.optString("description"),
                motherTongue = o.optString("motherTongue"),
                locales = locales,
                gender = o.optString("gender"),
                rate = o.optInt("rate"),
                languageMissing = o.optBoolean("languageMissing"),
                unsupported = if (o.isNull("unsupported")) null else o.getString("unsupported"),
            )
        }
    }

    fun delete(name: String) = Native.deleteVoice(root.path, name)

    /** Unpack an installer file into the store; returns the voices it brought. */
    fun importInstaller(exe: File): List<String> {
        val out = JSONObject(Native.unpackInstaller(exe.path, root.path)).getJSONArray("voices")
        return (0 until out.length()).map { out.getString(it) }
    }

    /** Copy [input] to a temporary file, unpack it, delete the copy. */
    fun importStream(input: InputStream, progress: Progress): List<String> {
        val tmp = File.createTempFile("import", ".exe", cache)
        try {
            tmp.outputStream().use { out -> copy(input, out, -1, progress) }
            return importInstaller(tmp)
        } finally {
            tmp.delete()
        }
    }

    /**
     * Install [installer]'s voice: from the mirror when one is configured and
     * has it, otherwise the installer from archive.org.
     */
    fun download(installer: Installer, mirror: String, progress: Progress): List<String> {
        if (mirror.isNotEmpty()) {
            try {
                if (downloadFromMirror(installer, mirror, progress)) return listOf(installer.voice)
            } catch (e: Exception) {
                android.util.Log.w(TAG, "mirror failed for ${installer.file}, falling back to archive.org", e)
            }
        }
        val tmp = File(cache, "download.exe.part")
        try {
            get(installer.archiveUrl).use { conn ->
                val total = conn.contentLengthLong.takeIf { it > 0 } ?: installer.size
                conn.inputStream.use { input -> tmp.outputStream().use { copy(input, it, total, progress) } }
            }
            return importInstaller(tmp)
        } finally {
            tmp.delete()
        }
    }

    /** False when the mirror's catalog doesn't have the voice. */
    private fun downloadFromMirror(installer: Installer, mirror: String, progress: Progress): Boolean {
        val catalog = get("$mirror/v1/catalog.json").use { c -> JSONObject(c.inputStream.bufferedReader().readText()) }
        val voices = catalog.getJSONArray("voices")
        val voice = (0 until voices.length()).map { voices.getJSONObject(it) }.firstOrNull {
            it.optString("installer") == installer.file || it.getString("name").equals(installer.voice, ignoreCase = true)
        } ?: return false
        val files = mutableListOf<JSONObject>()
        voice.getJSONArray("files").let { f -> (0 until f.length()).forEach { files += f.getJSONObject(it) } }
        if (!voice.isNull("language_folder")) {
            val folder = voice.getString("language_folder")
            catalog.getJSONObject("languages").optJSONObject(folder)?.getJSONArray("files")?.let { f ->
                (0 until f.length()).forEach { files += f.getJSONObject(it) }
            }
        }
        val wanted = files.filter { f ->
            val target = File(root, f.getString("path"))
            !(target.isFile && target.length() == f.getLong("size"))
        }
        val total = wanted.sumOf { it.getLong("size") }
        var before = 0L
        for (f in wanted) {
            val path = f.getString("path")
            require(!path.split('/').contains("..") && !path.startsWith("/")) { "$path: outside the store" }
            val target = File(root, path)
            target.parentFile?.mkdirs()
            val part = File(target.path + ".part")
            val digest = MessageDigest.getInstance("SHA-256")
            get("$mirror/v1/" + path.split('/').joinToString("/") { Uri.encode(it) }).use { conn ->
                conn.inputStream.use { input ->
                    part.outputStream().use { out ->
                        copy(input, out, total, { d, t -> progress(before + d, t) }, digest)
                    }
                }
            }
            val hex = digest.digest().joinToString("") { "%02x".format(it) }
            if (!hex.equals(f.getString("sha256"), ignoreCase = true)) {
                part.delete()
                throw IllegalStateException("$path: checksum mismatch")
            }
            if (!part.renameTo(target)) throw IllegalStateException("$path: could not be stored")
            before += f.getLong("size")
        }
        return true
    }

    private fun get(url: String): ConnectionHandle {
        var current = URL(url)
        repeat(5) {
            val conn = current.openConnection() as HttpURLConnection
            conn.connectTimeout = 20_000
            conn.readTimeout = 60_000
            conn.instanceFollowRedirects = true
            val code = conn.responseCode
            if (code in 300..399) {
                val next = conn.getHeaderField("Location") ?: throw IllegalStateException("$url: redirect without Location")
                conn.disconnect()
                current = URL(current, next)
                return@repeat
            }
            if (code !in 200..299) {
                conn.disconnect()
                throw IllegalStateException("$current: HTTP $code")
            }
            return ConnectionHandle(conn)
        }
        throw IllegalStateException("$url: too many redirects")
    }

    class ConnectionHandle(private val conn: HttpURLConnection) : AutoCloseable {
        val contentLengthLong: Long get() = conn.contentLengthLong
        val inputStream: InputStream get() = conn.inputStream
        override fun close() = conn.disconnect()
    }

    private fun copy(
        input: InputStream,
        out: java.io.OutputStream,
        total: Long,
        progress: Progress,
        digest: MessageDigest? = null,
    ) {
        val buf = ByteArray(256 * 1024)
        var done = 0L
        var reported = 0L
        while (true) {
            if (Thread.currentThread().isInterrupted) throw InterruptedException()
            val n = input.read(buf)
            if (n < 0) break
            out.write(buf, 0, n)
            digest?.update(buf, 0, n)
            done += n
            if (done - reported >= 512 * 1024) {
                reported = done
                progress(done, total)
            }
        }
        progress(done, total)
    }

    companion object {
        const val TAG = "quoquendo"

        fun catalog(): List<Installer> {
            val arr = JSONArray(Native.catalog())
            return (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                Installer(
                    file = o.getString("file"),
                    label = o.getString("label"),
                    language = o.getString("language"),
                    voice = o.getString("voice"),
                    gender = o.getString("gender"),
                    size = o.getLong("size"),
                    archiveUrl = o.getString("archiveUrl"),
                )
            }
        }

        /** The Android TTS voice name of [voice] speaking [locale]. */
        fun ttsVoiceName(voice: String, locale: Locale) = "$voice-${locale.toLanguageTag()}"
    }
}
