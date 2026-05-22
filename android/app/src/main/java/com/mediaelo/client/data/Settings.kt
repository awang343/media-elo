package com.mediaelo.client.data

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.edit
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Persistent app settings. Initialized once from [MediaEloApp.onCreate].
 *
 * The base URL is read live from [baseUrl]'s current value by the HTTP layer,
 * so updates take effect on the next request without rebuilding the client.
 */
object Settings {
    const val DEFAULT_BASE_URL = "http://10.0.2.2:7878"

    private const val PREFS = "media_elo_settings"
    private const val KEY_BASE_URL = "base_url"

    private lateinit var prefs: SharedPreferences

    private val _baseUrl = MutableStateFlow(DEFAULT_BASE_URL)
    val baseUrl: StateFlow<String> = _baseUrl.asStateFlow()

    fun init(context: Context) {
        prefs = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        _baseUrl.value = prefs.getString(KEY_BASE_URL, DEFAULT_BASE_URL) ?: DEFAULT_BASE_URL
    }

    /** Normalizes scheme + trailing slash. Throws [IllegalArgumentException] on garbage input. */
    fun setBaseUrl(raw: String) {
        val normalized = normalize(raw)
        prefs.edit { putString(KEY_BASE_URL, normalized) }
        _baseUrl.value = normalized
    }

    private fun normalize(raw: String): String {
        val trimmed = raw.trim().trimEnd('/')
        require(trimmed.isNotEmpty()) { "URL is empty" }
        val withScheme = if (trimmed.contains("://")) trimmed else "http://$trimmed"
        val uri = runCatching { java.net.URI(withScheme) }.getOrNull()
            ?: throw IllegalArgumentException("Not a valid URL")
        require(uri.scheme in setOf("http", "https")) { "Scheme must be http or https" }
        require(!uri.host.isNullOrBlank()) { "Host is required" }
        return withScheme
    }
}
