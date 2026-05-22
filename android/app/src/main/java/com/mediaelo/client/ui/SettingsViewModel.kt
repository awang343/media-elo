package com.mediaelo.client.ui

import androidx.lifecycle.ViewModel
import com.mediaelo.client.data.Repo
import com.mediaelo.client.data.Settings
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class SettingsUiState(
    val draft: String,
    val saved: String,
    val error: String? = null,
    val justSaved: Boolean = false,
) {
    val dirty: Boolean get() = draft.trim().trimEnd('/') != saved
}

class SettingsViewModel : ViewModel() {
    private val initial = Settings.baseUrl.value
    private val _state = MutableStateFlow(SettingsUiState(draft = initial, saved = initial))
    val state: StateFlow<SettingsUiState> = _state.asStateFlow()

    fun onDraftChange(value: String) {
        _state.value = _state.value.copy(draft = value, error = null, justSaved = false)
    }

    fun resetToDefault() {
        onDraftChange(Settings.DEFAULT_BASE_URL)
    }

    fun save() {
        val current = _state.value
        try {
            Settings.setBaseUrl(current.draft)
            Repo.invalidate()
            val saved = Settings.baseUrl.value
            _state.value = current.copy(draft = saved, saved = saved, error = null, justSaved = true)
        } catch (e: IllegalArgumentException) {
            _state.value = current.copy(error = e.message ?: "Invalid URL", justSaved = false)
        }
    }
}
