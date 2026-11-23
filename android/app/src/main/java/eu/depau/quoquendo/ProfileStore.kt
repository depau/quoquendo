package eu.depau.quoquendo

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

/** One profile as shown in the list. */
data class ProfileEntry(val name: String, val json: String, val builtin: Boolean)

/**
 * The speech profiles, stored as one JSON document in the app's preferences.
 * Built-ins come from the Rust engine and are never written; user profiles
 * and the active name are.
 */
object ProfileStore {
    const val KEY = "profiles"

    private val builtins: List<ProfileEntry> by lazy {
        val array = JSONArray(Native.builtinProfiles())
        (0 until array.length()).map {
            val o = array.getJSONObject(it)
            ProfileEntry(o.getString("name"), o.toString(), true)
        }
    }

    private fun defaultBook() = JSONObject()
        .put("version", 1)
        .put("active", "Modern")
        .put("profiles", JSONArray())

    private fun book(prefs: SharedPreferences): JSONObject =
        prefs.getString(KEY, null)?.let { runCatching { JSONObject(it) }.getOrNull() } ?: defaultBook()

    private fun save(prefs: SharedPreferences, book: JSONObject) {
        prefs.edit().putString(KEY, book.toString()).apply()
    }

    fun all(prefs: SharedPreferences): List<ProfileEntry> {
        val user = book(prefs).optJSONArray("profiles") ?: JSONArray()
        val seen = builtins.map { it.name.lowercase() }.toMutableSet()
        val out = builtins.toMutableList()
        for (i in 0 until user.length()) {
            val o = user.optJSONObject(i) ?: continue
            val name = o.optString("name").trim()
            if (name.isEmpty() || !seen.add(name.lowercase())) continue
            out += ProfileEntry(name, o.toString(), false)
        }
        return out
    }

    fun activeName(prefs: SharedPreferences): String {
        val name = book(prefs).optString("active")
        return all(prefs).firstOrNull { it.name.equals(name, ignoreCase = true) }?.name ?: "Modern"
    }

    fun activeJson(prefs: SharedPreferences): String =
        all(prefs).firstOrNull { it.name.equals(activeName(prefs), ignoreCase = true) }?.json
            ?: builtins.first { it.name == "Modern" }.json

    fun smooth(prefs: SharedPreferences): Boolean =
        runCatching { JSONObject(activeJson(prefs)).optString("joins", "original") == "smooth" }.getOrDefault(false)

    fun setActive(prefs: SharedPreferences, name: String) {
        val b = book(prefs)
        if (all(prefs).any { it.name.equals(name, ignoreCase = true) }) {
            b.put("active", name)
            save(prefs, b)
        }
    }

    /** Replace (or add) a custom profile under the name it had. */
    fun update(prefs: SharedPreferences, oldName: String, json: String): String? {
        val o = runCatching { JSONObject(json) }.getOrElse { return it.message ?: "not a profile" }
        val name = o.optString("name").trim()
        if (name.isEmpty()) return "a profile needs a name"
        if (builtins.any { it.name.equals(name, ignoreCase = true) }) return "that name belongs to a built-in profile"
        if (builtins.any { it.name.equals(oldName, ignoreCase = true) }) return "built-in profiles can't be changed"
        runCatching { Native.applyProfile("", o.toString(), null) }.onFailure { return it.message ?: "invalid profile" }
        val b = book(prefs)
        val user = b.optJSONArray("profiles") ?: JSONArray()
        for (i in 0 until user.length()) {
            val existing = user.getJSONObject(i).optString("name")
            if (!existing.equals(oldName, ignoreCase = true) && existing.equals(name, ignoreCase = true)) {
                return "a profile named $name already exists"
            }
        }
        var found = false
        for (i in 0 until user.length()) {
            if (user.getJSONObject(i).optString("name").equals(oldName, ignoreCase = true)) {
                user.put(i, o)
                found = true
                break
            }
        }
        if (!found) user.put(o)
        b.put("profiles", user)
        if (b.optString("active").equals(oldName, ignoreCase = true)) b.put("active", name)
        save(prefs, b)
        return null
    }

    fun duplicate(prefs: SharedPreferences, name: String): String? {
        val source = all(prefs).firstOrNull { it.name.equals(name, ignoreCase = true) } ?: return null
        val taken = all(prefs).map { it.name.lowercase() }.toSet()
        var candidate = "${source.name} copy"
        var n = 2
        while (candidate.lowercase() in taken) candidate = "${source.name} copy ${n++}"
        val copy = JSONObject(source.json).put("name", candidate)
        val b = book(prefs)
        val user = b.optJSONArray("profiles") ?: JSONArray()
        user.put(copy)
        b.put("profiles", user).put("active", candidate)
        save(prefs, b)
        return candidate
    }

    fun delete(prefs: SharedPreferences, name: String): String? {
        if (builtins.any { it.name.equals(name, ignoreCase = true) }) return "built-in profiles can't be deleted"
        val b = book(prefs)
        val user = b.optJSONArray("profiles") ?: JSONArray()
        val kept = JSONArray()
        var found = false
        for (i in 0 until user.length()) {
            val o = user.getJSONObject(i)
            if (o.optString("name").equals(name, ignoreCase = true)) found = true else kept.put(o)
        }
        if (!found) return "no profile named $name"
        b.put("profiles", kept)
        if (b.optString("active").equals(name, ignoreCase = true)) b.put("active", "Modern")
        save(prefs, b)
        return null
    }

    fun import(prefs: SharedPreferences, json: String): String? {
        val o = runCatching { JSONObject(json) }.getOrElse { return it.message ?: "not a profile" }
        var name = o.optString("name").trim()
        if (name.isEmpty()) return "a profile needs a name"
        val taken = all(prefs).map { it.name.lowercase() }.toSet()
        if (name.lowercase() in taken) {
            var candidate = "$name copy"
            var n = 2
            while (candidate.lowercase() in taken) candidate = "$name copy ${n++}"
            name = candidate
        }
        o.put("name", name)
        runCatching { Native.applyProfile("", o.toString(), null) }.onFailure { return it.message ?: "invalid profile" }
        val b = book(prefs)
        val user = b.optJSONArray("profiles") ?: JSONArray()
        user.put(o)
        b.put("profiles", user).put("active", name)
        save(prefs, b)
        return name
    }

    fun export(prefs: SharedPreferences, name: String): String? =
        all(prefs).firstOrNull { it.name.equals(name, ignoreCase = true) }?.json

    /** Every setting that makes speech, as one backup document. */
    fun backup(prefs: SharedPreferences): String {
        val b = book(prefs)
        b.put("version", 1)
        return b.toString()
    }

    fun restore(prefs: SharedPreferences, json: String): String? {
        val b = runCatching { JSONObject(json) }.getOrElse { return it.message ?: "not a settings backup" }
        if (!b.has("profiles")) return "not a settings backup"
        val user = b.optJSONArray("profiles") ?: JSONArray()
        for (i in 0 until user.length()) {
            val o = user.optJSONObject(i) ?: continue
            runCatching { Native.applyProfile("", o.toString(), null) }.onFailure { return "profile ${i + 1}: ${it.message}" }
        }
        save(prefs, b)
        return null
    }
}
