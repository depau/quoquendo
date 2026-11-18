package eu.depau.quoquendo

import android.app.Application
import android.net.Uri
import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.runInterruptible
import kotlinx.coroutines.withContext
import org.json.JSONObject

/** A download or import under way: bytes done and total (-1 unknown), or unpacking. */
data class Task(val title: String, val done: Long = 0, val total: Long = -1, val unpacking: Boolean = false)

/** What the analysis shows: the phones, the recorded pieces and the waveform outline. */
data class Analysis(val phrases: List<AnPhrase>, val pieces: List<AnPiece>, val missing: List<String>, val envelope: FloatArray)
data class AnPhone(val sym: String, val known: Boolean, val wordStart: Boolean)
data class AnPhrase(val phones: List<AnPhone>, val pauseMs: Int, val mood: String?)
data class AnPiece(val covers: String, val unit: Int, val sentence: String, val atS: Double, val lenMs: Double, val pitch: Int, val hard: Boolean)

/** The app's state; its work lives as long as the app is open. */
class MainModel(app: Application) : AndroidViewModel(app) {
    val store = VoiceStore(app)
    val catalog: List<Installer> = VoiceStore.catalog()
    val mirror: String = BuildConfig.QQ_MIRROR_URL

    var installed by mutableStateOf<List<InstalledVoice>>(emptyList())
        private set
    /** keyed by installer file, or "import" */
    val tasks = mutableStateMapOf<String, Task>()
    private val jobs = HashMap<String, Job>()
    val messages = mutableStateListOf<String>()

    var analysis by mutableStateOf<Analysis?>(null)
        private set
    private var analysisVoice: String? = null
    private var analysisHandle = 0L

    init {
        refresh()
    }

    fun say(message: String) {
        messages.add(0, message)
        while (messages.size > 5) messages.removeAt(messages.size - 1)
    }

    fun refresh() {
        viewModelScope.launch {
            installed = try {
                withContext(Dispatchers.IO) { store.installed() }
            } catch (e: Exception) {
                say("Could not list voices: ${e.message}")
                emptyList()
            }
        }
    }

    fun isInstalled(i: Installer) = installed.any { it.name.equals(i.voice, ignoreCase = true) }

    fun download(i: Installer) {
        if (jobs[i.file]?.isActive == true) return
        tasks[i.file] = Task("Downloading ${i.label}", 0, i.size)
        jobs[i.file] = viewModelScope.launch {
            try {
                val voices = runInterruptible(Dispatchers.IO) {
                    store.download(i, mirror) { done, total ->
                        tasks[i.file] = Task("Downloading ${i.label}", done, total, unpacking = total > 0 && done >= total)
                    }
                }
                say("Installed ${voices.joinToString()}")
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "download of ${i.file} failed", e)
                say("${i.label}: ${e.message ?: e.javaClass.simpleName}")
            } finally {
                tasks.remove(i.file)
                jobs.remove(i.file)
                refresh()
            }
        }
    }

    fun cancel(i: Installer) {
        jobs[i.file]?.cancel()
    }

    fun import(uri: Uri) {
        val key = "import"
        if (jobs[key]?.isActive == true) return
        tasks[key] = Task("Importing")
        jobs[key] = viewModelScope.launch {
            try {
                val voices = runInterruptible(Dispatchers.IO) {
                    val input = getApplication<Application>().contentResolver.openInputStream(uri)
                        ?: throw IllegalStateException("cannot open $uri")
                    input.use { s -> store.importStream(s) { done, _ -> tasks[key] = Task("Importing", done, -1) } }
                }
                say("Installed ${voices.joinToString()}")
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "import failed", e)
                say("Import failed: ${e.message ?: e.javaClass.simpleName}")
            } finally {
                tasks.remove(key)
                jobs.remove(key)
                refresh()
            }
        }
    }

    fun delete(v: InstalledVoice) {
        viewModelScope.launch {
            try {
                withContext(Dispatchers.IO) {
                    if (analysisVoice == v.name) freeAnalysis()
                    store.delete(v.name)
                }
                say("Deleted ${v.name}")
            } catch (e: Exception) {
                say("Could not delete ${v.name}: ${e.message}")
            }
            refresh()
        }
    }

    /** Run the analysis of [text] with [voice] (loaded in this process, apart from the service's). */
    fun analyse(voice: String, text: String) {
        viewModelScope.launch {
            analysis = try {
                withContext(Dispatchers.Default) {
                    synchronized(this@MainModel) {
                        if (analysisVoice != voice) {
                            freeAnalysis()
                            analysisHandle = Native.loadVoice(store.root.path, voice)
                            analysisVoice = voice
                        }
                        parse(JSONObject(Native.analyse(analysisHandle, text)))
                    }
                }
            } catch (e: Exception) {
                say("Analysis failed: ${e.message}")
                null
            }
        }
    }

    private fun freeAnalysis() = synchronized(this) {
        if (analysisHandle != 0L) Native.freeVoice(analysisHandle)
        analysisHandle = 0
        analysisVoice = null
    }

    override fun onCleared() {
        freeAnalysis()
    }

    private fun parse(o: JSONObject): Analysis {
        val ph = o.getJSONArray("phrases")
        val phrases = (0 until ph.length()).map { i ->
            val p = ph.getJSONObject(i)
            val phones = p.getJSONArray("phones")
            AnPhrase(
                (0 until phones.length()).map { j ->
                    val x = phones.getJSONObject(j)
                    AnPhone(x.getString("sym"), x.getBoolean("known"), x.getBoolean("wordStart"))
                },
                p.getInt("pauseMs"),
                if (p.isNull("mood")) null else p.getString("mood"),
            )
        }
        val pc = o.getJSONArray("pieces")
        val pieces = (0 until pc.length()).map { i ->
            val p = pc.getJSONObject(i)
            val covers = p.getJSONArray("covers")
            AnPiece(
                (0 until covers.length()).joinToString(" ") { covers.getString(it) },
                p.getInt("unit"),
                p.getString("sentence"),
                p.getDouble("atS"),
                p.getDouble("lenMs"),
                p.getInt("pitch"),
                p.getBoolean("hard"),
            )
        }
        val m = o.getJSONArray("missing")
        val env = o.getJSONArray("envelope")
        return Analysis(
            phrases,
            pieces,
            (0 until m.length()).map { m.getString(it) },
            FloatArray(env.length()) { env.getDouble(it).toFloat() },
        )
    }
}
