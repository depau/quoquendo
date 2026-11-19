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
    val demoSentence: String,
    val languageMissing: Boolean,
    val unsupported: String?,
    /** `Italian`: the folder under data/languages its language data comes from */
    val languageFolder: String?,
    /** what deleting it removes, relative to the store */
    val files: List<String>,
) {
    /** It can speak (possibly on built-in rules alone, when its language data is missing). */
    val speakable: Boolean get() = unsupported == null

    /** Speak is offered: it speaks, and with its own language data. */
    val ready: Boolean get() = unsupported == null && !languageMissing
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
    /** why its voice can't speak yet, known from the catalog alone */
    val unsupported: String?,
) {
    /** `English` for `UK English`: the filter chip it belongs to. */
    val family: String get() = language.substringAfterLast(' ')
}

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
            val locales = o.getJSONArray("locales").strings()
                .map { Locale.forLanguageTag(it) }
                .filter { it.language.isNotEmpty() }
            InstalledVoice(
                name = o.getString("name"),
                description = o.optString("description"),
                motherTongue = o.optString("motherTongue"),
                locales = locales,
                gender = o.optString("gender"),
                rate = o.optInt("rate"),
                demoSentence = o.optString("demoSentence"),
                languageMissing = o.optBoolean("languageMissing"),
                unsupported = o.stringOrNull("unsupported"),
                languageFolder = o.stringOrNull("languageFolder"),
                files = o.getJSONArray("files").strings(),
            )
        }
    }

    /** Bytes the voice's own files take (its language data aside). */
    fun bytesOf(v: InstalledVoice): Long = v.files.sumOf { bytesUnder(File(root, it)) }

    /** Bytes the whole store takes. */
    fun bytesUsed(): Long = bytesUnder(root)

    private fun bytesUnder(f: File): Long = f.walkTopDown().filter { it.isFile }.sumOf { it.length() }

    fun delete(name: String) = Native.deleteVoice(root.path, name)

    /** Unpack an installer file into the store; returns the voices it brought. */
    fun importInstaller(exe: File): List<String> =
        JSONObject(Native.unpackInstaller(exe.path, root.path)).getJSONArray("voices").strings()

    /** Copy [input] to a temporary file (reporting copy progress), unpack it, delete the copy. */
    fun importStream(input: InputStream, size: Long, progress: Progress, unpacking: () -> Unit): List<String> {
        val tmp = File.createTempFile("import", ".exe", cache)
        try {
            tmp.outputStream().use { out -> copy(input, out, size, progress) }
            unpacking()
            return importInstaller(tmp)
        } finally {
            tmp.delete()
        }
    }

    /**
     * Install [installer]'s voice: from the mirror when one is configured and
     * has it, otherwise the installer from archive.org.
     */
    fun download(installer: Installer, mirror: String, progress: Progress, unpacking: () -> Unit): List<String> {
        if (mirror.isNotEmpty()) {
            try {
                val catalog = mirrorCatalog(mirror)
                val voice = catalog.voices().firstOrNull {
                    it.optString("installer") == installer.file || it.getString("name").equals(installer.voice, ignoreCase = true)
                }
                if (voice != null) {
                    fetch(mirror, voice.getJSONArray("files").objects() + catalog.languageFiles(voice), progress)
                    return listOf(voice.getString("name"))
                }
            } catch (e: Exception) {
                // a cancelled download stops here, without falling back
                if (e is InterruptedException || e is java.io.InterruptedIOException || Thread.currentThread().isInterrupted) throw e
                android.util.Log.w(TAG, "mirror failed for ${installer.file}, falling back to archive.org", e)
            }
        }
        val tmp = File(cache, "download.exe.part")
        try {
            get(installer.archiveUrl).use { conn ->
                val total = conn.contentLengthLong.takeIf { it > 0 } ?: installer.size
                conn.inputStream.use { input -> tmp.outputStream().use { copy(input, it, total, progress) } }
            }
            unpacking()
            return importInstaller(tmp)
        } finally {
            tmp.delete()
        }
    }

    /** Fetch only the language data [voice] needs, from the mirror. */
    fun downloadLanguage(voice: String, mirror: String, progress: Progress) {
        val catalog = mirrorCatalog(mirror)
        val entry = catalog.voices().firstOrNull { it.getString("name").equals(voice, ignoreCase = true) }
            ?: throw IllegalStateException("the mirror doesn't know $voice")
        val files = catalog.languageFiles(entry)
        if (files.isEmpty()) throw IllegalStateException("the mirror has no language data for $voice")
        fetch(mirror, files, progress)
    }

    private fun mirrorCatalog(mirror: String): JSONObject =
        get("$mirror/v1/catalog.json").use { c -> JSONObject(c.inputStream.bufferedReader().readText()) }

    private fun JSONObject.voices(): List<JSONObject> = getJSONArray("voices").objects()

    private fun JSONObject.languageFiles(voice: JSONObject): List<JSONObject> {
        val folder = voice.stringOrNull("language_folder") ?: return emptyList()
        return getJSONObject("languages").optJSONObject(folder)?.getJSONArray("files")?.objects() ?: emptyList()
    }

    /**
     * Fetch every mirror file not stored yet, each checked against its
     * sha256. If any fails or the download is cancelled, the files this call
     * stored are removed again.
     */
    private fun fetch(mirror: String, files: List<JSONObject>, progress: Progress) {
        val wanted = files.filter { f ->
            val target = File(root, f.getString("path"))
            !(target.isFile && target.length() == f.getLong("size"))
        }
        val total = wanted.sumOf { it.getLong("size") }
        var before = 0L
        val stored = mutableListOf<File>()
        try {
            for (f in wanted) {
                val path = f.getString("path")
                require(!path.split('/').contains("..") && !path.startsWith("/")) { "$path: outside the store" }
                val target = File(root, path)
                target.parentFile?.mkdirs()
                val part = File(target.path + ".part")
                val digest = MessageDigest.getInstance("SHA-256")
                try {
                    get("$mirror/v1/" + path.split('/').joinToString("/") { Uri.encode(it) }).use { conn ->
                        conn.inputStream.use { input ->
                            part.outputStream().use { out -> copy(input, out, total, { d, t -> progress(before + d, t) }, digest) }
                        }
                    }
                } catch (e: Exception) {
                    part.delete()
                    throw e
                }
                val hex = digest.digest().joinToString("") { "%02x".format(it) }
                if (!hex.equals(f.getString("sha256"), ignoreCase = true)) {
                    part.delete()
                    throw IllegalStateException("$path: checksum mismatch — nothing was installed")
                }
                if (!part.renameTo(target)) throw IllegalStateException("$path: could not be stored")
                stored += target
                before += f.getLong("size")
            }
        } catch (e: Exception) {
            stored.forEach { it.delete() }
            throw e
        }
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
                throw IllegalStateException("${current.host} answered $code")
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
            if (done - reported >= 256 * 1024) {
                reported = done
                progress(done, total)
            }
        }
        progress(done, total)
    }

    companion object {
        const val TAG = "quoquendo"

        fun catalog(): List<Installer> = JSONArray(Native.catalog()).objects().map { o ->
            Installer(
                file = o.getString("file"),
                label = o.getString("label"),
                language = o.getString("language"),
                voice = o.getString("voice"),
                gender = o.getString("gender"),
                size = o.getLong("size"),
                archiveUrl = o.getString("archiveUrl"),
                unsupported = o.stringOrNull("unsupported"),
            )
        }

        /** The Android TTS voice name of [voice] speaking [locale]. */
        fun ttsVoiceName(voice: String, locale: Locale) = "$voice-${locale.toLanguageTag()}"
    }
}

private fun JSONArray.strings(): List<String> = (0 until length()).map { getString(it) }
private fun JSONArray.objects(): List<JSONObject> = (0 until length()).map { getJSONObject(it) }
private fun JSONObject.stringOrNull(key: String): String? = if (isNull(key)) null else getString(key)
