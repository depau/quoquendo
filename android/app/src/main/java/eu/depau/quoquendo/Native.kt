package eu.depau.quoquendo

/**
 * Speech as 16-bit [pcm] at [rate], and its words: per word in order, the
 * sample it starts at and its UTF-16 range in the text, flattened in [marks].
 */
class Rendered(@JvmField val pcm: ShortArray, @JvmField val rate: Int, @JvmField val marks: IntArray)

/** [Native.analyse]'s JSON, and its audio (null when it could not be rendered). */
class Analysed(@JvmField val json: String, @JvmField val audio: Rendered?)

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

    /** JSON array of the formats [export] writes: `{ext, label, hint, mime}`. */
    @JvmStatic external fun formats(): String

    /** A language's flag (`US English`, `EnglishGb`) as shapes in a 30×21 box, `[]` for none: see [Flag]. */
    @JvmStatic external fun flag(language: String): String

    /** A flag that is a picture (`Trenitalia`) as PNG bytes, or null. */
    @JvmStatic external fun flagImage(language: String): ByteArray?

    @JvmStatic external fun loadVoice(storeDir: String, name: String): Long

    /** The recordings bank's own rate: the rate of [pieceSamples] (44000 for CodecMsx voices). */
    @JvmStatic external fun audioRate(handle: Long): Int

    /** One recorded piece, samples [from]..[to] of the recordings (an `analyse` piece's `from`/`to`). */
    @JvmStatic external fun pieceSamples(handle: Long, unit: Int, from: Int, to: Int): ShortArray

    @JvmStatic external fun freeVoice(handle: Long)

    /** The built-in speech profiles, as their JSON array. */
    @JvmStatic external fun builtinProfiles(): String

    /** [text] with [profile]'s rules, as JSON `{text, warnings}`. */
    @JvmStatic external fun applyProfile(text: String, profile: String, language: String?): String

    /** [text] spoken, pieces joined smoothly when [smooth]. */
    @JvmStatic external fun synthesize(handle: Long, text: String, smooth: Boolean, language: String?, profile: String?): Rendered

    /**
     * [text] spoken as a file in the format with extension [ext] ([formats]),
     * the text and its timing embedded. [language] as in [analyse].
     */
    @JvmStatic external fun export(handle: Long, text: String, ext: String, smooth: Boolean, language: String?, profile: String?): ByteArray

    @JvmStatic external fun deleteVoice(storeDir: String, name: String)

    /** JSON `{language, languages, rows:[[label, value]]}`: the voice's details and the languages it can read with. */
    @JvmStatic external fun details(handle: Long): String

    /**
     * [text] analysed and rendered, read by [language]'s rules (one of
     * [details]' `languages`) or, when null, the voice's own. JSON
     * `{sounds:[{sym,known,atS,endS,word,phrase,wordStart,pause}], plan:[{pauseMs,mood,missing}],
     * pieces:[{phrase,covers,unit,sentence,from,to,recS,lenMs,pitch,hard,atS,endS}],
     * words:[{start,end,at,atS,endS}], sentences, audio:{rate,samples,seconds,step,envelope} | {error}}`;
     * word `start`/`end` are UTF-16 offsets in [text].
     */
    @JvmStatic external fun analyse(handle: Long, text: String, smooth: Boolean, language: String?, profile: String?): Analysed
}
