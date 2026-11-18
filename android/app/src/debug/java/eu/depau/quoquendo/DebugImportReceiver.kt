package eu.depau.quoquendo

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import java.io.File

/**
 * Debug builds only: unpack the installer named by the `file` extra, found in
 * the app's external files folder, into the voice store (see the debug manifest).
 * Without `file` it only creates that folder, so `adb push` lands where the app can read.
 */
class DebugImportReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val drop = context.getExternalFilesDir(null)
        if (drop == null) {
            Log.e(VoiceStore.TAG, "DEBUG_IMPORT: no external files folder")
            return
        }
        val name = intent.getStringExtra("file")
        if (name == null) {
            Log.i(VoiceStore.TAG, "DEBUG_IMPORT: push installers to ${drop.path}")
            return
        }
        val path = File(drop, name).path
        val pending = goAsync()
        Thread {
            try {
                val voices = VoiceStore(context).importInstaller(File(path))
                Log.i(VoiceStore.TAG, "DEBUG_IMPORT $path: installed $voices")
            } catch (e: Exception) {
                Log.e(VoiceStore.TAG, "DEBUG_IMPORT $path failed", e)
            } finally {
                pending.finish()
            }
        }.start()
    }
}
