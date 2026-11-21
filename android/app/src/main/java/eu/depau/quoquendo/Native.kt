package eu.depau.quoquendo

/** The engine, in libquoquendo_jni.so (crates/quoquendo-jni). Errors throw RuntimeException. */
object Native {
    init {
        System.loadLibrary("quoquendo_jni")
    }

    /** Unpack an installer `.exe` into the LTTS7 tree [storeDir]; JSON `{"voices":[…]}`. */
    @JvmStatic external fun unpackInstaller(installerPath: String, storeDir: String): String

    /** JSON array of `{name, description, motherTongue, locales, gender, rate, languageMissing, unsupported}`. */
    @JvmStatic external fun listVoices(storeDir: String): String

    /** JSON array of the pack's installers `{file, label, language, voice, gender, size, archiveUrl}`. */
    @JvmStatic external fun catalog(): String

    @JvmStatic external fun loadVoice(storeDir: String, name: String): Long
    /** The rate `synthesize` returns (the engine's output rate). */
    @JvmStatic external fun sampleRate(handle: Long): Int

    /** The recordings bank's own rate: the rate of [pieceSamples] (44000 for CodecMsx voices). */
    @JvmStatic external fun audioRate(handle: Long): Int

    /** One recorded piece, samples [from]..[to] of the recordings (an `analyse` piece's `from`/`to`). */
    @JvmStatic external fun pieceSamples(handle: Long, unit: Int, from: Int, to: Int): ShortArray

    @JvmStatic external fun freeVoice(handle: Long)
    @JvmStatic external fun synthesize(handle: Long, text: String): ShortArray
    /** [pcm] at [rate] (32, 44.1 or 48 kHz) as an MP3 file. */
    @JvmStatic external fun encodeMp3(pcm: ShortArray, rate: Int): ByteArray
    @JvmStatic external fun deleteVoice(storeDir: String, name: String)

    /**
     * JSON `{phrases:[{phones:[{sym,known,wordStart}],pauseMs,mood}], plan:[{pauseMs,mood,missing}],
     * pieces:[{phrase,covers,unit,sentence,from,to,atS,lenMs,pitch,hard}], sentences,
     * audio:{rate,samples,seconds,envelope} | {error}}`.
     */
    @JvmStatic external fun analyse(handle: Long, text: String): String
}
