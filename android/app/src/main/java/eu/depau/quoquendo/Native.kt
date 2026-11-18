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
    @JvmStatic external fun freeVoice(handle: Long)
    @JvmStatic external fun sampleRate(handle: Long): Int
    @JvmStatic external fun synthesize(handle: Long, text: String): ShortArray
    @JvmStatic external fun deleteVoice(storeDir: String, name: String)

    /** JSON `{phrases:[{phones:[{sym,known,wordStart}],pauseMs,mood}], pieces:[…], missing:[…], envelope:[…]}`. */
    @JvmStatic external fun analyse(handle: Long, text: String): String
}
